// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{
    boxed::Box,
    collections::VecDeque,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::ffi::c_uint;

use spin::{Mutex, RwLock};
use tee_raw_sys::*;

use super::{
    TeeResult,
    common::file_ops::{FileVariant, TeeFileLike},
    fs_dirfile::TeeFsDirfileFileh,
    fs_htree::{
        TEE_FS_HTREE_HASH_SIZE, TeeFsHtree, TeeFsHtreeImage, TeeFsHtreeNodeImage, TeeFsHtreeType,
    },
    tee_pobj::tee_pobj,
};

#[repr(C)]
pub struct tee_file_handle;

static REE_FS_MUTEX: Mutex<()> = Mutex::new(());

pub const BLOCK_SHIFT: usize = 12;
pub const BLOCK_SIZE: usize = 1 << BLOCK_SHIFT;

#[derive(Debug, Default)]
pub struct TeeFsFd {
    pub ht: Box<TeeFsHtree>,
    pub fd: FileVariant,
    pub dfh: TeeFsDirfileFileh,
    pub uuid: TEE_UUID,
}

/// read data from crypto RNG to buffer
///
/// # Arguments
/// * `buf` - buffer to store read data
///
/// # Returns
/// * `Ok(())` - success
/// * `Err(TEE_ERROR_GENERIC)` - error
/// TODO: Using mbedtls to implement a real RNG
pub fn crypto_rng_read(buf: &mut [u8]) -> TeeResult {
    buf.fill(0);
    Ok(())
}

fn pos_to_block_num(position: usize) -> usize {
    position >> BLOCK_SHIFT
}

pub fn get_tmp_block() -> Result<Box<[u8; BLOCK_SIZE]>, ()> {
    let mut vec = Vec::new();
    if vec.try_reserve_exact(BLOCK_SIZE).is_err() {
        return Err(());
    }
    vec.resize(BLOCK_SIZE, 0);
    vec.into_boxed_slice().try_into().map_err(|_| ())
}

fn put_tmp_block(_block: Box<[u8; BLOCK_SIZE]>) {}

pub fn get_offs_size(typ: TeeFsHtreeType, idx: usize, vers: u8) -> TeeResult<(usize, usize)> {
    let node_size = size_of::<TeeFsHtreeNodeImage>();
    let block_nodes = BLOCK_SIZE / (node_size * 2);

    let _pbn: usize;
    let _bidx: usize;

    assert!(vers == 0 || vers == 1);

    // File layout
    // [demo with input:
    // BLOCK_SIZE = 4096,
    // node_size = 66,
    // block_nodes = 4096/(66*2) = 31 ]
    //
    // phys block 0:
    // tee_fs_htree_image vers 0 @ offs = 0
    // tee_fs_htree_image vers 1 @ offs = sizeof(tee_fs_htree_image)
    //
    // phys block 1:
    // tee_fs_htree_node_image 0  vers 0 @ offs = 0
    // tee_fs_htree_node_image 0  vers 1 @ offs = node_size
    // tee_fs_htree_node_image 1  vers 0 @ offs = node_size * 2
    // tee_fs_htree_node_image 1  vers 1 @ offs = node_size * 3
    // ...
    // tee_fs_htree_node_image 30 vers 0 @ offs = node_size * 60
    // tee_fs_htree_node_image 30 vers 1 @ offs = node_size * 61
    //
    // phys block 2:
    // data block 0 vers 0
    //
    // phys block 3:
    // data block 0 vers 1
    //
    // ...
    // phys block 62:
    // data block 30 vers 0
    //
    // phys block 63:
    // data block 30 vers 1
    //
    // phys block 64:
    // tee_fs_htree_node_image 31  vers 0 @ offs = 0
    // tee_fs_htree_node_image 31  vers 1 @ offs = node_size
    // tee_fs_htree_node_image 32  vers 0 @ offs = node_size * 2
    // tee_fs_htree_node_image 32  vers 1 @ offs = node_size * 3
    // ...
    // tee_fs_htree_node_image 61 vers 0 @ offs = node_size * 60
    // tee_fs_htree_node_image 61 vers 1 @ offs = node_size * 61
    //
    // phys block 65:
    // data block 31 vers 0
    //
    // phys block 66:
    // data block 31 vers 1
    // ...

    match typ {
        TeeFsHtreeType::Head => {
            let offs = size_of::<TeeFsHtreeImage>() * vers as usize;
            let size = size_of::<TeeFsHtreeImage>();
            Ok((offs, size))
        }
        TeeFsHtreeType::Node => {
            let pbn = 1 + ((idx / block_nodes) * block_nodes * 2);
            let offs =
                pbn * BLOCK_SIZE + 2 * node_size * (idx % block_nodes) + node_size * vers as usize;
            let size = node_size;
            Ok((offs, size))
        }
        TeeFsHtreeType::Block => {
            let bidx = 2 * idx + vers as usize;
            let pbn = 2 + bidx + bidx / (block_nodes * 2 - 1);
            Ok((pbn * BLOCK_SIZE, BLOCK_SIZE))
        }
        _ => Err(TEE_ERROR_GENERIC),
    }
}

/// read data from file to buffer at offset using rpc
/// the typical flow is:
///   1. call ree_fs_rpc_read_init to get offs and size to fill params
///   2. send OPTEE_RPC_CMD_FS to ree
/// in starryos, just usign file operations to read data
///
/// # Arguments
/// * `fd` - file descriptor
/// * `typ` - type of the file
/// * `idx` - index of the file
/// * `vers` - version of the file
/// * `data` - buffer to store read data
///
/// # Returns
/// * `Ok(usize)` - number of bytes read
pub fn tee_fs_rpc_read_final(
    fd: &mut FileVariant,
    typ: TeeFsHtreeType,
    idx: usize,
    vers: u8,
    data: &mut [u8],
) -> TeeResult<usize> {
    let (offs, _size) = get_offs_size(typ, idx, vers)?;
    let size = fd.pread(data, offs)?;
    Ok(size)
}

/// write data to file at offset using rpc
///
/// # Arguments
/// * `fd` - file descriptor
/// * `typ` - type of the file
/// * `idx` - index of the file
/// * `vers` - version of the file
/// * `data` - buffer to store write data
///
/// # Returns
/// * `Ok(usize)` - number of bytes written
pub fn tee_fs_rpc_write_final(
    fd: &FileVariant,
    typ: TeeFsHtreeType,
    idx: usize,
    vers: u8,
    data: &[u8],
) -> TeeResult<usize> {
    let (offs, _size) = get_offs_size(typ, idx, vers)?;
    let size = fd.pwrite(data, offs)?;
    Ok(size)
}

/// init for read rpc
/// no need to do anything in starryos, because we use file operations to read data
pub fn ree_fs_rpc_read_init() -> TeeResult {
    Ok(())
}

/// init for write rpc
/// no need to do anything in starryos, because we use file operations to write data
pub fn ree_fs_rpc_write_init() -> TeeResult {
    Ok(())
}

pub trait TeeFsHtreeStorageOps {
    fn block_size(&self) -> usize;

    fn rpc_read_init(&self) -> TeeResult;

    fn rpc_read_final(
        &self,
        fd: &mut FileVariant,
        typ: TeeFsHtreeType,
        idx: usize,
        vers: u8,
        data: &mut [u8],
    ) -> TeeResult<usize>;

    fn rpc_write_init(&self) -> TeeResult;

    fn rpc_write_final(
        &self,
        fd: &mut FileVariant,
        typ: TeeFsHtreeType,
        idx: usize,
        vers: u8,
        data: &[u8],
    ) -> TeeResult<usize>;
}

/// tee_file_operations is the operations of the tee_pobj
#[derive(Debug)]
pub struct TeeFileOperations {
    pub open: fn(po: &tee_pobj, size: &mut usize) -> TeeResult<Arc<tee_file_handle>>,

    pub create: fn(
        po: &tee_pobj,
        overwrite: bool,
        head: &[u8],
        attr: &[u8],
        data_core: &[u8],
        data_user: &[u8],
        data_size: usize,
    ) -> TeeResult<Arc<tee_file_handle>>,

    pub close: fn(fh: &mut Arc<tee_file_handle>) -> TeeResult,

    pub read: fn(
        fh: &Arc<tee_file_handle>,
        pos: usize,
        buf_core: &mut [u8],
        buf_user: &mut [u8],
        len: &mut usize,
    ) -> TeeResult,

    pub write: fn(
        fh: &Arc<tee_file_handle>,
        pos: usize,
        buf_core: &[u8],
        buf_user: &[u8],
        len: usize,
    ) -> TeeResult,

    pub rename: fn(old_po: &mut tee_pobj, new_po: &mut tee_pobj, overwrite: bool) -> TeeResult,

    pub remove: fn(po: &mut tee_pobj) -> TeeResult,

    pub truncate: fn(fh: &mut Arc<tee_file_handle>, size: usize) -> TeeResult,

    // fn opendir(uuid: &TEE_UUID, d: &mut Arc<tee_fs_dir>) -> TeeResult;

    // fn readdir(d: &mut Arc<tee_fs_dir>, ent: &mut Arc<tee_fs_dirent>) -> TeeResult;

    // fn closedir(d: &mut Arc<tee_fs_dir>) -> TeeResult;
    #[cfg(feature = "tee_test")]
    pub echo: fn() -> String,
}

// Helper functions for REE_FS_OPS
fn ree_fs_open(_po: &tee_pobj, _size: &mut usize) -> TeeResult<Arc<tee_file_handle>> {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_create(
    _po: &tee_pobj,
    _overwrite: bool,
    _head: &[u8],
    _attr: &[u8],
    _data_core: &[u8],
    _data_user: &[u8],
    _data_size: usize,
) -> TeeResult<Arc<tee_file_handle>> {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_close(_fh: &mut Arc<tee_file_handle>) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_read(
    _fh: &Arc<tee_file_handle>,
    _pos: usize,
    _buf_core: &mut [u8],
    _buf_user: &mut [u8],
    _len: &mut usize,
) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_write(
    _fh: &Arc<tee_file_handle>,
    _pos: usize,
    _buf_core: &[u8],
    _buf_user: &[u8],
    _len: usize,
) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_truncate(_fh: &mut Arc<tee_file_handle>, _size: usize) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_rename(_old_po: &mut tee_pobj, _new_po: &mut tee_pobj, _overwrite: bool) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_remove(_po: &mut tee_pobj) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

#[cfg(feature = "tee_test")]
fn ree_fs_echo() -> String {
    "TeeFileOperations->echo".to_string()
}

// global file_ops for REE FS, in starryos REE is  starryos self
pub static REE_FS_OPS: TeeFileOperations = TeeFileOperations {
    open: ree_fs_open,
    create: ree_fs_create,
    close: ree_fs_close,
    read: ree_fs_read,
    write: ree_fs_write,
    truncate: ree_fs_truncate,
    rename: ree_fs_rename,
    remove: ree_fs_remove,
    #[cfg(feature = "tee_test")]
    echo: ree_fs_echo,
};

// Returns the appropriate tee_file_operations for the specified storage ID.
// The value TEE_STORAGE_PRIVATE will select the REE FS if available, otherwise
// RPMB.
//
// only support REE FS now
pub fn tee_svc_storage_file_ops(storage_id: c_uint) -> TeeResult<&'static TeeFileOperations> {
    match storage_id {
        TEE_STORAGE_PRIVATE => Ok(&REE_FS_OPS),
        TEE_STORAGE_PRIVATE_REE => Ok(&REE_FS_OPS),
        TEE_STORAGE_PRIVATE_RPMB => Err(TEE_ERROR_NOT_SUPPORTED),
        _ => Err(TEE_ERROR_BAD_PARAMETERS),
    }
}

/// Trait for file interface operations supplied by user of this interface
///
/// tee_fs_dirfile_operations
pub trait TeeFsDirfileOperations {
    /// Opens a file
    fn open(
        &self,
        create: bool,
        hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
        uuid: Option<&TEE_UUID>,
        dfh: Option<&TeeFsDirfileFileh>,
    ) -> TeeResult<Box<TeeFsFd>>;

    /// Closes a file, changes are discarded unless commit_writes is called before
    fn close(&self, fh: &mut TeeFsFd);

    /// Reads from an open file
    fn read(&self, fh: &mut TeeFsFd, pos: usize, buf: &mut [u8], len: &mut usize) -> TeeResult;

    /// Writes to an open file
    fn write(&self, fh: &mut TeeFsFd, pos: usize, buf: &[u8]) -> TeeResult;

    /// Commits changes since the file was opened
    fn commit_writes(
        &self,
        fh: &mut TeeFsFd,
        hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
    ) -> TeeResult;
}
