// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};

use bytemuck::{Pod, Zeroable};
use tee_raw_sys::{
    TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_ITEM_NOT_FOUND, TEE_ERROR_SHORT_BUFFER,
    TEE_OBJECT_ID_MAX_LEN, TEE_UUID,
};

use super::{
    TeeResult,
    bitstring::{BitStr, bit_clear, bit_ffc, bit_nclear, bit_set, bit_test, bitstr_size},
    fs_htree::TEE_FS_HTREE_HASH_SIZE,
    tee_fs::TeeFileHandle,
    tee_ree_fs::{ReeDirfOps, TeeFsDirfileOperations},
};
/// file handle for dirfile tee_fs_dirfile_fileh
///
/// # Fields
/// - `file_number`: file number
/// - `hash`: hash of the file, used to pass to `tee_fs_htree_open()`
/// - `idx`: index of the file handle in dirfile
#[derive(Debug, Copy, Clone, Default)]
pub struct TeeFsDirfileFileh {
    pub file_number: u32,
    /// hash of the file, used to pass to `tee_fs_htree_open()`
    ///
    /// this hash is the hash of the root node of the file hash tree, used to:
    /// - unique identifier of the file
    /// - file integrity verification
    /// - file location and lookup
    pub hash: [u8; TEE_FS_HTREE_HASH_SIZE],
    pub idx: i32,
}

/// entry of dirfile dirfile_entry
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DirFileEntry {
    uuid: TEE_UUID,
    oid: [u8; TEE_OBJECT_ID_MAX_LEN as _],
    oid_len: u32,
    hash: [u8; TEE_FS_HTREE_HASH_SIZE],
    file_number: u32,
}

impl Default for DirFileEntry {
    fn default() -> Self {
        Self {
            uuid: Default::default(),
            oid: [0; TEE_OBJECT_ID_MAX_LEN as _],
            oid_len: 0,
            hash: [0; TEE_FS_HTREE_HASH_SIZE],
            file_number: 0,
        }
    }
}
pub const OID_EMPTY_NAME: u8 = 1;

#[derive(Debug, Default)]
pub struct TeeFsDirfileDirh {
    fops: ReeDirfOps,
    fh: TeeFileHandle,
    pub nbits: usize,
    pub files: Vec<BitStr>,
    ndents: usize,
}

/// grow the files array if needed
///
/// File layout
///
/// dirfile_entry.0
/// ...
/// dirfile_entry.n
///
/// where n the index is disconnected from file_number in struct dirfile_entry
fn maybe_grow_files(dirh: &mut TeeFsDirfileDirh, idx: usize) -> TeeResult {
    if idx < dirh.nbits {
        return Ok(());
    }

    let new_size = bitstr_size(idx + 1);
    dirh.files.resize(new_size, 0);

    bit_nclear(&mut dirh.files, dirh.nbits, idx);
    dirh.nbits = idx + 1;

    Ok(())
}

/// check if the entry is free
/// An object can have an ID of size zero. This object is represented by
/// oidlen == 0 and oid[0] == OID_EMPTY_NAME. When both are zero, the entry is
/// not a valid object.
///
/// # Arguments
/// * `dent`: the entry to check
fn is_free(dent: &DirFileEntry) -> bool {
    debug_assert!(dent.oid_len != 0 || dent.oid[0] == 0 || dent.oid[0] == OID_EMPTY_NAME);

    dent.oid_len == 0 && dent.oid[0] == 0
}

fn clear_file(dirh: &mut TeeFsDirfileDirh, idx: usize) {
    if idx < dirh.nbits {
        bit_clear(&mut dirh.files, idx);
    }
}

fn test_file(dirh: &mut TeeFsDirfileDirh, idx: usize) -> bool {
    if idx < dirh.nbits {
        return bit_test(&dirh.files, idx);
    }
    false
}

pub fn set_file(dirh: &mut TeeFsDirfileDirh, idx: usize) -> TeeResult {
    maybe_grow_files(dirh, idx)?;
    bit_set(&mut dirh.files, idx);

    Ok(())
}

pub fn tee_fs_dirfile_fileh_to_fname(
    dfh: Option<&TeeFsDirfileFileh>,
    fname_buffer: &mut [u8],
) -> TeeResult<usize> {
    let s = if let Some(dfh_val) = dfh {
        // Format the file_number as a hexadecimal string
        format!("{:x}", dfh_val.file_number)
    } else {
        "dirf.db".to_string()
    };

    let bytes = s.as_bytes();
    let required_len = bytes.len();

    if fname_buffer.len() < required_len {
        // If the buffer is too small, return the required length and the error
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    // Copy the bytes into the provided buffer
    fname_buffer[..bytes.len()].copy_from_slice(bytes);

    Ok(required_len)
}

pub fn tee_fs_dirfile_rename(
    dirh: &mut TeeFsDirfileDirh,
    uuid: &TEE_UUID,
    dfh: &mut TeeFsDirfileFileh,
    oid: &[u8],
) -> TeeResult {
    let mut dent = DirFileEntry::default();

    if oid.len() > dent.oid.len() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    dent.uuid = *uuid;

    if !oid.is_empty() {
        dent.oid[..oid.len()].copy_from_slice(oid);
    } else {
        dent.oid[0] = OID_EMPTY_NAME;
    }

    dent.oid_len = oid.len() as u32;
    dent.hash.copy_from_slice(&dfh.hash);
    dent.file_number = dfh.file_number;

    if dfh.idx < 0 {
        let res = tee_fs_dirfile_find(dirh, uuid, oid);
        dfh.idx = match res {
            Ok(v) => v.idx,
            Err(res) if res == TEE_ERROR_ITEM_NOT_FOUND => find_empty_idx(dirh)?,
            Err(res) => return Err(res),
        };
    }

    write_dent(dirh, dfh.idx as usize, &dent)
}

pub fn read_dent(dirh: &mut TeeFsDirfileDirh, idx: usize, dent: &mut DirFileEntry) -> TeeResult {
    let entry_size = core::mem::size_of::<DirFileEntry>();
    let offset = entry_size * idx;
    let mut len = entry_size;

    // 读取目录项数据
    // convert DirFileEntry to mutable byte slice
    // safety: DirFileEntry is #[repr(C)], memory layout is determined, size is fixed, can be safely converted
    let dent_bytes = unsafe {
        core::slice::from_raw_parts_mut(dent as *mut DirFileEntry as *mut u8, entry_size)
    };
    dirh.fops
        .read(&mut dirh.fh, offset, dent_bytes, &mut len)
        .inspect_err(|e| {
            error!("read_dent: error: {:X?}", e);
        })?;

    tee_debug!("read_dent: len: {:?}, entry_size: {:?}", len, entry_size);
    // 验证读取的数据长度
    if len != entry_size {
        return Err(TEE_ERROR_ITEM_NOT_FOUND);
    }

    Ok(())
}

pub fn write_dent(dirh: &mut TeeFsDirfileDirh, n: usize, dent: &DirFileEntry) -> TeeResult {
    let entry_size = core::mem::size_of::<DirFileEntry>();

    // convert DirFileEntry to byte slice
    // safety: DirFileEntry is #[repr(C)], memory layout is determined, size is fixed, can be safely converted
    let dent_bytes = unsafe {
        core::slice::from_raw_parts(dent as *const DirFileEntry as *const u8, entry_size)
    };
    dirh.fops.write(&mut dirh.fh, entry_size * n, dent_bytes)?;

    if n >= dirh.ndents {
        dirh.ndents = n + 1;
    }

    Ok(())
}

pub fn tee_fs_dirfile_open(
    create: bool,
    hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
    fops: &ReeDirfOps,
) -> TeeResult<Box<TeeFsDirfileDirh>> {
    let mut dirh = Box::new(TeeFsDirfileDirh::default());
    dirh.fops = *fops;

    let fd = fops.open(create, hash, None, None)?;
    dirh.fh = *fd;

    tee_debug!("tee_fs_dirfile_open: dirh.fh: {:?}", dirh.fh);

    let mut n: usize = 0;

    let res: TeeResult<()> = loop {
        let mut dent: DirFileEntry = unsafe { core::mem::zeroed() };

        match read_dent(&mut *dirh, n, &mut dent) {
            Err(TEE_ERROR_ITEM_NOT_FOUND) => {
                tee_debug!(
                    "read_dent: TEE_ERROR_ITEM_NOT_FOUND at idx {}",
                    n
                );
                break Ok(());
            }
            Err(e) => break Err(e),
            Ok(()) => {}
        }

        /* if (is_free(&dent)) */
        if is_free(&dent) {
            n += 1;
            continue;
        }

        /* if (test_file(dirh, dent.file_number)) */
        if test_file(&mut *dirh, dent.file_number as usize) {
            tee_debug!(
                "clearing duplicate file number {}",
                dent.file_number
            );
            let mut zero_dent: DirFileEntry = unsafe { core::mem::zeroed() };
            if let Err(e) = write_dent(&mut *dirh, n, &mut zero_dent) {
                break Err(e);
            }
            n += 1;
            continue;
        }

        /* res = set_file(dirh, dent.file_number); */
        if let Err(e) = set_file(&mut *dirh, dent.file_number as usize) {
            break Err(e);
        }

        n += 1;
    };

    match res {
        Ok(()) => {
            dirh.ndents = n;
            tee_debug!("tee_fs_dirfile_open: dirh.ndents: {}", dirh.ndents);
            Ok(dirh)
        }
        Err(e) => {
            tee_fs_dirfile_close(&mut *dirh);
            Err(e)
        }
    }
}

pub fn tee_fs_dirfile_find(
    dirh: &mut TeeFsDirfileDirh,
    uuid: &TEE_UUID,
    oid: &[u8],
) -> TeeResult<TeeFsDirfileFileh> {
    let oidlen = oid.len();
    let mut dent: DirFileEntry = unsafe { core::mem::zeroed() };
    let mut n: usize = 0;

    // Note: Do NOT use `for n in 0..` here! In Rust, that creates a new
    // loop variable that shadows the outer `n`. We need to use `loop`
    // and manually increment `n` to match the C behavior.
    loop {
        read_dent(dirh, n, &mut dent)?;

        if is_free(&dent) {
            n += 1;
            continue;
        }

        if dent.oid_len as usize != oidlen {
            n += 1;
            continue;
        }

        debug_assert!(test_file(dirh, dent.file_number as usize));

        if &dent.uuid == uuid && &dent.oid[..oidlen] == oid {
            break;
        }
        n += 1;
    }

    let mut dfh = TeeFsDirfileFileh {
        idx: n as i32,
        file_number: dent.file_number,
        hash: [0u8; TEE_FS_HTREE_HASH_SIZE],
    };
    dfh.hash.copy_from_slice(&dent.hash);

    Ok(dfh)
}

fn find_empty_idx(dirh: &mut TeeFsDirfileDirh) -> TeeResult<i32> {
    let mut dent: DirFileEntry = DirFileEntry::default();
    let mut n: i32 = 0;

    let n = loop {
        match read_dent(dirh, n as usize, &mut dent) {
            Err(TEE_ERROR_ITEM_NOT_FOUND) => break n,
            Err(e) => return Err(e),
            Ok(()) => {}
        }

        if is_free(&dent) {
            break n;
        }

        n += 1;
    };

    Ok(n)
}

pub fn tee_fs_dirfile_remove(dirh: &mut TeeFsDirfileDirh, dfh: &TeeFsDirfileFileh) -> TeeResult {
    let mut dent: DirFileEntry = DirFileEntry::default();
    read_dent(dirh, dfh.idx as usize, &mut dent)?;

    if dent.oid_len == 0 {
        return Ok(());
    }

    let file_number = dent.file_number;
    tee_debug!(
        "tee_fs_dirfile_remove: dfh.file_number: {:?}, file_number: {:?}",
        dfh.file_number,
        file_number
    );
    core::assert!(dfh.file_number == file_number);
    core::assert!(test_file(dirh, file_number as usize));

    dent = unsafe { core::mem::zeroed() };
    write_dent(dirh, dfh.idx as usize, &mut dent)?;
    tee_debug!(
        "tee_fs_dirfile_remove: after write_dent, dirh.fh.ht.data.dirty: {}, dfh.idx: {}",
        dirh.fh.ht.data.dirty,
        dfh.idx
    );
    clear_file(dirh, file_number as usize);

    Ok(())
}

pub fn tee_fs_dirfile_update_hash(
    dirh: &mut TeeFsDirfileDirh,
    dfh: &TeeFsDirfileFileh,
) -> TeeResult {
    let mut dent: DirFileEntry = unsafe { core::mem::zeroed() };

    read_dent(dirh, dfh.idx as usize, &mut dent)?;
    tee_debug!(
        "tee_fs_dirfile_update_hash: dent.file_number: {:?}, dfh.file_number: {:?}",
        dent.file_number,
        dfh.file_number
    );
    core::assert!(dent.file_number == dfh.file_number);
    core::assert!(test_file(dirh, dent.file_number as usize));

    dent.hash.copy_from_slice(&dfh.hash);

    write_dent(dirh, dfh.idx as usize, &mut dent)
}

pub fn tee_fs_dirfile_close(dirh: &mut TeeFsDirfileDirh) -> TeeResult {
    dirh.fops.close(&mut dirh.fh);

    // drop(dirh.files);
    // drop(dirh);
    Ok(())
}

pub fn tee_fs_dirfile_commit_writes(
    dirh: &mut TeeFsDirfileDirh,
    hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
) -> TeeResult {
    tee_debug!(
        "tee_fs_dirfile_commit_writes: dirh.fh.fd: {:?}, dirh.fh.ht.data.head.counter: {}",
        dirh.fh.fd,
        dirh.fh.ht.data.head.counter
    );
    dirh.fops.commit_writes(&mut dirh.fh, hash)
}

pub fn tee_fs_dirfile_get_tmp(
    dirh: &mut TeeFsDirfileDirh,
    dfh: &mut TeeFsDirfileFileh,
) -> TeeResult {
    let mut i: isize = 0;

    if !dirh.files.is_empty() {
        bit_ffc(&dirh.files, dirh.nbits, &mut i);
        if i == -1 {
            i = dirh.nbits as isize;
        }
    }

    set_file(dirh, i as usize)?;
    dfh.file_number = i as u32;

    Ok(())
}

pub fn tee_fs_dirfile_get_next(
    dirh: &mut TeeFsDirfileDirh,
    uuid: &TEE_UUID,
    idx: &mut i32,
    oid: &mut [u8],
) -> TeeResult<usize> {
    let mut i = *idx + 1;

    if i < 0 {
        i = 0;
    }

    let mut dent: DirFileEntry = unsafe { core::mem::zeroed() };

    loop {
        read_dent(dirh, i as usize, &mut dent)?;
        if dent.uuid == *uuid && dent.oid_len > 0 {
            break;
        }
        i += 1;
    }

    // 检查缓冲区是否足够
    let len = dent.oid_len as usize;

    if oid.len() < len {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    oid[..len].copy_from_slice(&dent.oid[..len]);

    *idx = i;

    Ok(len)
}

#[cfg(feature = "tee_test")]
pub mod tests_tee_fs_dirfile {
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{TestDescriptor, TestResult},
        test_fn, tests, tests_name,
    };

    test_fn! {
        using TestResult;

        fn test_fileh_some_zero_filenumber() {
            let dfh = TeeFsDirfileFileh {
                file_number: 0,
                hash: [0; TEE_FS_HTREE_HASH_SIZE],
                idx: 0,
            };
            // Expected: "0" + null = 2 bytes
            let mut buffer = [0u8; 2];
            let result = tee_fs_dirfile_fileh_to_fname(Some(&dfh), &mut buffer);

            assert!(result.is_ok());
            let written_len = result.unwrap();
            assert_eq!(written_len, 1);
            assert_eq!(str::from_utf8(&buffer[..1]).unwrap(), "0");
            //assert_eq!(buffer[1], 0); // Verify null terminator
        }
    }

    test_fn! {
        using TestResult;

        fn test_fileh_some_small_filenumber() {
            let dfh = TeeFsDirfileFileh {
                file_number: 0xABCD,
                hash: [0; TEE_FS_HTREE_HASH_SIZE],
                idx: 0,
            };
            // Expected: "abcd" = 4 bytes
            let mut buffer = [0u8; 4];
            let result = tee_fs_dirfile_fileh_to_fname(Some(&dfh), &mut buffer);

            assert!(result.is_ok());
            let written_len = result.unwrap();
            assert_eq!(written_len, 4);
            assert_eq!(str::from_utf8(&buffer[..4]).unwrap(), "abcd");
            //assert_eq!(buffer[4], 0); // Verify null terminator
        }
    }

    test_fn! {
        using TestResult;

        fn test_fileh_some_large_filenumber() {
            let dfh = TeeFsDirfileFileh {
                file_number: 0xFFFFFFFF,
                hash: [0; TEE_FS_HTREE_HASH_SIZE],
                idx: 0,
            };
            // Expected: "ffffffff" + null = 9 bytes
            let mut buffer = [0u8; 8];
            let result = tee_fs_dirfile_fileh_to_fname(Some(&dfh), &mut buffer);

            assert!(result.is_ok());
            let written_len = result.unwrap();
            assert_eq!(written_len, 8);
            assert_eq!(str::from_utf8(&buffer[..8]).unwrap(), "ffffffff");
            //assert_eq!(buffer[8], 0); // Verify null terminator
        }
    }

    test_fn! {
        using TestResult;

        fn test_fileh_none_case() {
            // Expected: "dirf.db" + null = 8 bytes
            let mut buffer = [0u8; 7];
            let result = tee_fs_dirfile_fileh_to_fname(None, &mut buffer);

            assert!(result.is_ok());
            let written_len = result.unwrap();
            assert_eq!(written_len, 7);
            assert_eq!(str::from_utf8(&buffer[..7]).unwrap(), "dirf.db");
            // assert_eq!(buffer[7], 0); // Verify null terminator
        }
    }

    test_fn! {
        using TestResult;

        fn test_fileh_short_buffer_file_number() {
            let dfh = TeeFsDirfileFileh {
                file_number: 0x1234, // "1234" (4 chars) -> needs 5 bytes total (including null)
                hash: [0; TEE_FS_HTREE_HASH_SIZE],
                idx: 0,
            };
            // Provide 1 byte less than required
            let mut buffer = [0u8; 3];
            let result = tee_fs_dirfile_fileh_to_fname(Some(&dfh), &mut buffer);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), TEE_ERROR_SHORT_BUFFER);
        }
    }

    test_fn! {
        using TestResult;

        fn test_fileh_short_buffer_dirf_db() {
            // "dirf.db" (7 chars) -> needs 7 bytes total (including null)
            // Provide 1 byte less than required
            let mut buffer = [0u8; 6];
            let result = tee_fs_dirfile_fileh_to_fname(None, &mut buffer);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), TEE_ERROR_SHORT_BUFFER);
        }
    }

    test_fn! {
        using TestResult;

        fn test_fileh_empty_buffer() {
            let dfh = TeeFsDirfileFileh {
                file_number: 0x1234,
                hash: [0; TEE_FS_HTREE_HASH_SIZE],
                idx: 0,
            };
            let mut buffer = [0u8; 0];
            let result = tee_fs_dirfile_fileh_to_fname(Some(&dfh), &mut buffer);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), TEE_ERROR_SHORT_BUFFER);
        }
    }

    test_fn! {
        using TestResult;

        fn test_fileh_exact_buffer_file_number() {
            let dfh = TeeFsDirfileFileh {
                file_number: 0xABCDEF, // "abcdef" (6 chars) -> needs 7 bytes total
                hash: [0; TEE_FS_HTREE_HASH_SIZE],
                idx: 0,
            };
            // Provide exact required size
            let mut buffer = [0u8; 7];
            let result = tee_fs_dirfile_fileh_to_fname(Some(&dfh), &mut buffer);

            assert!(result.is_ok());
            let written_len = result.unwrap();
            assert_eq!(written_len, 6);
            assert_eq!(str::from_utf8(&buffer[..6]).unwrap(), "abcdef");
            //assert_eq!(buffer[6], 0); // Verify null terminator
        }
    }

    test_fn! {
        using TestResult;

        fn test_fileh_exact_buffer_dirf_db() {
            // "dirf.db" (7 chars) -> needs 7 bytes total
            // Provide exact required size
            let mut buffer = [0u8; 7];
            let result = tee_fs_dirfile_fileh_to_fname(None, &mut buffer);

            assert!(result.is_ok());
            let written_len = result.unwrap();
            assert_eq!(written_len, 7);
            assert_eq!(str::from_utf8(&buffer[..7]).unwrap(), "dirf.db");
            //assert_eq!(buffer[7], 0); // Verify null terminator
        }
    }
    tests_name! {
        TEST_TEE_FS_DIRFILE;
        //------------------------
        test_fileh_some_zero_filenumber,
        test_fileh_some_small_filenumber,
        test_fileh_some_large_filenumber,
        test_fileh_none_case,
        test_fileh_short_buffer_file_number,
        test_fileh_short_buffer_dirf_db,
        test_fileh_empty_buffer,
        test_fileh_exact_buffer_file_number,
        test_fileh_exact_buffer_dirf_db,
    }
}
