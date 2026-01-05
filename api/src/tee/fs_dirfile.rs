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

    if oid.is_empty() || oid.len() > dent.oid.len() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    dent.uuid = *uuid;
    dent.oid[..oid.len()].copy_from_slice(oid);
    dent.oid_len = oid.len() as u32;
    dent.hash.copy_from_slice(&dfh.hash);
    dent.file_number = dfh.file_number;

    if dfh.idx < 0 {
        let dfh2 = match tee_fs_dirfile_find(dirh, uuid, oid) {
            Ok(v) => v,
            Err(res) if res == TEE_ERROR_ITEM_NOT_FOUND => {
                // 再试一次，用空 oid
                tee_fs_dirfile_find(dirh, uuid, &[])? // 如果还错，向上传播错误
            }
            Err(res) => return Err(res),
        };

        dfh.idx = dfh2.idx;
    }

    write_dent(dirh, dfh.idx as usize, &mut dent)?;

    Ok(())
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
    dirh.fops.read(&mut dirh.fh, offset, dent_bytes, &mut len)?;

    // 验证读取的数据长度
    if len != entry_size {
        return Err(TEE_ERROR_ITEM_NOT_FOUND);
    }

    Ok(())
}

pub fn write_dent(dirh: &mut TeeFsDirfileDirh, n: usize, dent: &mut DirFileEntry) -> TeeResult {
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

    let mut n = 0;

    let result = (|| {
        for idx in 0.. {
            n = idx;

            let mut dent: DirFileEntry = unsafe { core::mem::zeroed() };
            match read_dent(&mut *dirh, idx, &mut dent) {
                Err(TEE_ERROR_ITEM_NOT_FOUND) => {
                    // 读到末尾正常退出循环
                    break;
                }
                Err(e) => {
                    // 其他错误直接返回
                    return Err(e);
                }
                Ok(()) => {}
            }

            if dent.oid_len == 0 {
                continue;
            }

            if test_file(&mut *dirh, dent.file_number as usize) {
                // 清除重复文件号
                let mut zero_dent: DirFileEntry = unsafe { core::mem::zeroed() };
                write_dent(&mut *dirh, n, &mut zero_dent)?;
                continue;
            }
            set_file(&mut *dirh, dent.file_number as usize)?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => (),
        Err(e) => {
            // tee_fs_dirfile_close(&mut *dirh)?;
            return Err(e);
        }
    }

    dirh.ndents = n;
    Ok(dirh)
}

pub fn tee_fs_dirfile_find(
    dirh: &mut TeeFsDirfileDirh,
    uuid: &TEE_UUID,
    oid: &[u8],
) -> TeeResult<TeeFsDirfileFileh> {
    let oidlen = oid.len();
    let mut first_free: Option<usize> = None;
    let mut n: usize = 0;
    let mut dent: DirFileEntry = unsafe { core::mem::zeroed() };

    for idx in 0.. {
        n = idx;

        // 读取目录项
        match read_dent(dirh, idx, &mut dent) {
            // 如果目录项不存在且没有指定 oid
            Err(TEE_ERROR_ITEM_NOT_FOUND) if oidlen == 0 => {
                dent = unsafe { core::mem::zeroed() };
                if let Some(free_idx) = first_free {
                    n = free_idx;
                }
                break;
            }
            // 其他错误直接返回
            // Err(TeeResultCode::ErrorItemNotFound) => return Ok(None),
            Err(e) => return Err(e),
            Ok(()) => {}
        }

        // 记录第一个空闲位置
        if dent.oid_len == 0 && first_free.is_none() {
            first_free = Some(idx);
        }

        // 如果 oid 长度不匹配，继续查找
        if dent.oid_len as usize != oidlen {
            continue;
        }

        // 如果指定了 oid，则确保文件存在
        if oidlen > 0 && !test_file(dirh, dent.file_number as usize) {
            continue;
        }

        // 匹配 uuid 和 oid
        if &dent.uuid == uuid && (oidlen == 0 || &dent.oid[..oidlen] == oid) {
            break;
        }
    }

    // 构建文件句柄 dfh
    let mut dfh = TeeFsDirfileFileh {
        idx: n as i32,
        file_number: dent.file_number,
        hash: [0u8; TEE_FS_HTREE_HASH_SIZE],
    };
    dfh.hash.copy_from_slice(&dent.hash);

    Ok(dfh)
}

pub fn tee_fs_dirfile_remove(dirh: &mut TeeFsDirfileDirh, dfh: &TeeFsDirfileFileh) -> TeeResult {
    let mut dent: DirFileEntry = unsafe { core::mem::zeroed() };
    read_dent(dirh, dfh.idx as usize, &mut dent)?;

    if dent.oid_len == 0 {
        return Ok(());
    }

    let file_number = dent.file_number;
    core::assert!(dfh.file_number == file_number);
    core::assert!(test_file(dirh, file_number as usize));

    dent = unsafe { core::mem::zeroed() };
    write_dent(dirh, dfh.idx as usize, &mut dent)?;
    clear_file(dirh, file_number as usize);

    Ok(())
}

pub fn tee_fs_dirfile_update_hash(
    dirh: &mut TeeFsDirfileDirh,
    dfh: &TeeFsDirfileFileh,
) -> TeeResult {
    let mut dent: DirFileEntry = unsafe { core::mem::zeroed() };

    read_dent(dirh, dfh.idx as usize, &mut dent)?;
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
