// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::vec::Vec;

use tee_raw_sys::{TEE_OBJECT_ID_MAX_LEN, TEE_UUID};

use super::{
    bitstring::{BitStr, bit_clear, bit_set, bit_test, bitstr_size},
    fs_htree::TEE_FS_HTREE_HASH_SIZE,
};

/// file handle for dirfile tee_fs_dirfile_fileh
///
/// # Fields
/// - `file_number`: file number
/// - `hash`: hash of the file, used to pass to `tee_fs_htree_open()`
/// - `idx`: index of the file handle in dirfile
#[derive(Copy, Clone, Default)]
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

pub const OID_EMPTY_NAME: u8 = 1;

pub struct TeeFsDirfileDirh {
    // fops: ReeDirfOps,
    // fh: TeeFileHandle,
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
///
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

// pub fn read_dent(dirh: &mut TeeFsDirfileDirh, idx: usize, dent: &mut DirFileEntry) -> TeeResult {
//     let entry_size = core::mem::size_of::<DirFileEntry>();
//     let offset = entry_size * idx;
//     let mut len = entry_size;

//     // 读取目录项数据
//     dirh.fops
//         .read(&mut dirh.fh, offset, bytemuck::bytes_of_mut(dent), &mut len)?;

//     // 验证读取的数据长度
//     if len != entry_size {
//         return TeeResultCode::ErrorItemNotFound.into_result();
//     }

//     Ok(())
// }