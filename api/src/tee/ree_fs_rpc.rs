// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::string::{String, ToString};

use axfs_ng_vfs::VfsError;
use tee_raw_sys::{TEE_ERROR_BAD_FORMAT, TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_ITEM_NOT_FOUND};

use super::{
    TeeResult,
    common::file_ops::{
        FS_MODE_644, FS_OFLAG_DEFAULT, FS_OFLAG_RW, FS_OFLAG_RW_TRUNC, FileVariant, TeeFileLike,
    },
    fs_dirfile::TeeFsDirfileFileh,
    tee_fs::TEE_FS_NAME_MAX,
    tee_svc_storage::tee_svc_storage_create_filename_dfh,
};

/// Create a filename from a dfh
///
/// # Arguments
/// * `dfh` - the dfh to create a filename from
///
/// # Returns
/// * `TeeResult<String>` - the filename
fn create_filename_from_dfh(dfh: Option<&TeeFsDirfileFileh>) -> TeeResult<String> {
    let mut f_name = [0u8; TEE_FS_NAME_MAX];
    let buf_len = tee_svc_storage_create_filename_dfh(&mut f_name, dfh)?;
    let f_name = str::from_utf8(&f_name[..buf_len]).map_err(|_| TEE_ERROR_BAD_FORMAT)?;
    Ok(f_name.to_string())
}

/// Open a file from a dfh
///
/// # Arguments
/// * `oflag` - the oflag to open the file with
/// * `dfh` - the dfh to open the file from
///
/// # Returns
/// * `TeeResult<FileVariant>` - the file variant
fn operation_open_dfh(oflag: u32, dfh: Option<&TeeFsDirfileFileh>) -> TeeResult<FileVariant> {
    let f_name = create_filename_from_dfh(dfh)?;
    let fd = FileVariant::open(&f_name, oflag, FS_MODE_644);

    match fd {
        Ok(fd) => Ok(fd),
        Err(VfsError::NotFound) => Err(TEE_ERROR_ITEM_NOT_FOUND),
        Err(e) => Err(TEE_ERROR_BAD_PARAMETERS),
    }
}

/// Open a file from a dfh with the default flags
///
/// TODO: check the oflag is valid for the operation
/// # Arguments
/// * `dfh` - the dfh to open the file from
///
/// # Returns
/// * `TeeResult<FileVariant>` - the file variant
pub fn tee_fs_rpc_open_dfh(dfh: Option<&TeeFsDirfileFileh>) -> TeeResult<FileVariant> {
    operation_open_dfh(FS_OFLAG_RW, dfh)
}

/// Create a file from a dfh with the default flags
///
/// # Arguments
/// * `dfh` - the dfh to create the file from
///
/// # Returns
/// * `TeeResult<FileVariant>` - the file variant
pub fn tee_fs_rpc_create_dfh(dfh: Option<&TeeFsDirfileFileh>) -> TeeResult<FileVariant> {
    operation_open_dfh(FS_OFLAG_DEFAULT, dfh)
}

/// Close a file from a dfh
///
/// # Arguments
/// * `fd` - the file variant to close
///
/// # Returns
/// * `TeeResult<()>` - the result of the operation
pub fn tee_fs_rpc_close(_fd: &FileVariant) -> TeeResult {
    // fd.close();
    Ok(())
}

/// Remove a file from a dfh
///
/// # Arguments
/// * `dfh` - the dfh to remove the file from
///
/// # Returns
/// * `TeeResult<()>` - the result of the operation
pub fn tee_fs_rpc_remove_dfh(dfh: Option<&TeeFsDirfileFileh>) -> TeeResult {
    let f_name = create_filename_from_dfh(dfh)?;

    tee_debug!("tee_fs_rpc_remove_dfh: f_name: {}", f_name);
    // Remove the file
    FileVariant::remove(&f_name).map_err(|_| TEE_ERROR_BAD_PARAMETERS)?;

    Ok(())
}

/// Truncate a file from a dfh
///
/// # Arguments
/// * `fd` - the file variant to truncate
/// * `len` - the length to truncate the file to
///
/// # Returns
/// * `TeeResult<()>` - the result of the operation
pub fn tee_fs_rpc_truncate(fd: &mut FileVariant, len: usize) -> TeeResult {
    fd.ftruncate(len).map_err(|_| TEE_ERROR_BAD_PARAMETERS)
}
