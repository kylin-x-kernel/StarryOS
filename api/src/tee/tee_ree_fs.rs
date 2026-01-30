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
    vec,
    vec::Vec,
};
use core::{any::Any, ffi::c_uint, fmt::Debug, ptr};

use scope_local::scope_local;
use spin::{Mutex, RwLock};
use tee_raw_sys::{TEE_STORAGE_PRIVATE, *};

use super::{
    TeeResult,
    common::file_ops::{FileVariant, TeeFileLike},
    fs_dirfile::{
        TeeFsDirfileDirh, TeeFsDirfileFileh, tee_fs_dirfile_close, tee_fs_dirfile_commit_writes,
        tee_fs_dirfile_find, tee_fs_dirfile_get_next, tee_fs_dirfile_get_tmp, tee_fs_dirfile_open,
        tee_fs_dirfile_remove, tee_fs_dirfile_rename, tee_fs_dirfile_update_hash,
    },
    fs_htree::{
        TEE_FS_HTREE_HASH_SIZE, TeeFsHtree, TeeFsHtreeImage, TeeFsHtreeNodeImage, TeeFsHtreeType,
        tee_fs_htree_get_meta, tee_fs_htree_meta_set_dirty, tee_fs_htree_open,
        tee_fs_htree_read_block, tee_fs_htree_sync_to_storage, tee_fs_htree_truncate,
        tee_fs_htree_write_block,
    },
    ree_fs_rpc::{
        tee_fs_rpc_close, tee_fs_rpc_create_dfh, tee_fs_rpc_open_dfh, tee_fs_rpc_remove_dfh,
        tee_fs_rpc_truncate,
    },
    tee_api_defines_extensions::{TEE_STORAGE_PRIVATE_REE, TEE_STORAGE_PRIVATE_RPMB},
    tee_fs::tee_fs_dirent,
    tee_pobj::tee_pobj,
    user_access::{copy_from_user, copy_to_user},
    utils::roundup_u,
};
use crate::tee::utils::slice_fmt;

pub type tee_file_handle = TeeFsFd;

static REE_FS_MUTEX: Mutex<()> = Mutex::new(());

pub const BLOCK_SHIFT: usize = 12;
pub const BLOCK_SIZE: usize = 1 << BLOCK_SHIFT;

#[derive(Debug, Default)]
pub struct TeeFsFd {
    pub ht: Box<TeeFsHtree>,
    pub fd: Box<FileVariant>,
    pub dfh: TeeFsDirfileFileh,
    pub uuid: TEE_UUID,
}

// auxiliary struct for TeeFsFd
// In upstream version, tee_fs_htree has member:
//   - stor:  tee_fs_htree_storage for file ops
//   - stor_aux: sturct data as parameter for file ops
// as such code: `res = ht->stor->rpc_read_init(ht->stor_aux, &op, type, idx, vers, &p);`
// In such case, the stor_aux is struct tee_fs_fd, as code in `ree_fs_rpc_read_init`:
// `struct tee_fs_fd *fdp = aux;`
// So we should implement impl TeeFsHtreeStorageOps for TeeFsFd.
// But TeeFsFd has members as TeeFsHtree, it is so complex to implement TeeFsHtreeStorageOps because of
// the life management is too complex.
//
// As we saw in the code, such as `rpc_write`:
// `res = ht->stor->rpc_write_init(ht->stor_aux, &op, type, idx, vers, &p);`
// We put parameter `struct tee_fs_htree *ht,` for `rpc_write`, but the real usage is `ht->stor` and `ht->stor_aux`
// and the stor_aux used in stor is just the `fd`, the file handle.
//
// the FileVariant is a wrapper of the file descriptor, it is actually a index number, the using of fd is guaranteed by the fd table
// with lock, so we can use it copy from other data.
//
// So we use TeeFsFdAux(with only member: fd: FileVariant) to store the file descriptor and the auxiliary struct data.
#[derive(Debug, Default)]
pub struct TeeFsFdAux {
    pub fd: FileVariant,
}

impl TeeFsFdAux {
    pub fn new() -> Self {
        Self {
            fd: FileVariant::default(),
        }
    }
}

#[repr(C)]
pub struct TeeFsDir {
    pub dirh: *mut TeeFsDirfileDirh,
    pub idx: i32,
    pub d: tee_fs_dirent,
    pub uuid: TEE_UUID,
}

impl Default for TeeFsDir {
    fn default() -> Self {
        TeeFsDir {
            dirh: ptr::null_mut(),
            idx: 0,
            d: tee_fs_dirent::default(),
            uuid: TEE_UUID::default(),
        }
    }
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
    fd: &FileVariant,
    typ: TeeFsHtreeType,
    idx: usize,
    vers: u8,
    data: &mut [u8],
) -> TeeResult<usize> {
    tee_debug!(
        "tee_fs_rpc_read_final: fd: {:?}, typ: {:?}, idx: {:?}, vers: {:?}, data_len: {:X?}",
        fd,
        typ,
        idx,
        vers,
        data.len()
    );
    let (offs, sz) = get_offs_size(typ, idx, vers)?;

    // alloc data with sz
    let mut data_alloc = vec![0; sz];
    let size = fd.pread(&mut data_alloc, offs)?;

    if (size != data.len()) {
        error!(
            "tee_fs_rpc_read_final: size: {} != data.len(): {}",
            size,
            data.len()
        );
        return Err(TEE_ERROR_CORRUPT_OBJECT);
    }

    data.copy_from_slice(&data_alloc[..data.len()]);
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
    let (offs, sz) = get_offs_size(typ, idx, vers)?;
    // alloc data with sz
    let mut data_alloc = vec![0; sz];

    debug_assert!(data.len() <= sz);
    data_alloc[..data.len()].copy_from_slice(data);

    tee_debug!(
        "tee_fs_rpc_write_final: fd: {:?}, typ: {:?}, idx: {:?}, vers: {:?}, offs: {:?}",
        fd,
        typ,
        idx,
        vers,
        offs
    );
    let size = fd.pwrite(&data_alloc, offs)?;
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

pub trait TeeFsHtreeStorageOps: Debug + Any + Send + Sync {
    fn block_size(&self) -> usize;

    fn rpc_read_init(&self) -> TeeResult;

    fn rpc_read_final(
        &self,
        // fd: &mut FileVariant,
        typ: TeeFsHtreeType,
        idx: usize,
        vers: u8,
        data: &mut [u8],
    ) -> TeeResult<usize>;

    fn rpc_write_init(&self) -> TeeResult;

    fn rpc_write_final(
        &self,
        // fd: &FileVariant,
        typ: TeeFsHtreeType,
        idx: usize,
        vers: u8,
        data: &[u8],
    ) -> TeeResult<usize>;

    fn clone_box(&self) -> Box<dyn TeeFsHtreeStorageOps> {
        unimplemented!()
    }
}

impl TeeFsHtreeStorageOps for TeeFsFdAux {
    fn block_size(&self) -> usize {
        BLOCK_SIZE
    }

    fn rpc_read_init(&self) -> TeeResult {
        ree_fs_rpc_read_init()
    }

    fn rpc_read_final(
        &self,
        // fd: &mut FileVariant,
        typ: TeeFsHtreeType,
        idx: usize,
        vers: u8,
        data: &mut [u8],
    ) -> TeeResult<usize> {
        tee_fs_rpc_read_final(&self.fd, typ, idx, vers, data).inspect_err(|e| {
            error!("rpc_read_final: error: {:X?}", e);
        })
    }

    fn rpc_write_init(&self) -> TeeResult {
        ree_fs_rpc_write_init()
    }

    fn rpc_write_final(
        &self,
        // fd: &FileVariant,
        typ: TeeFsHtreeType,
        idx: usize,
        vers: u8,
        data: &[u8],
    ) -> TeeResult<usize> {
        tee_fs_rpc_write_final(&self.fd, typ, idx, vers, data)
    }
}

/// Open a file, primitive version
///
/// # Arguments
/// * `uuid` - the uuid of the file
/// * `create` - whether to create the file
/// * `hash` - the hash of the file
/// * `dfh` - the dfh to open the file from
///
/// # Returns
/// * `TeeResult<Box<TeeFsFd>>` - the file descriptor
pub fn ree_fs_open_primitive(
    uuid: Option<&TEE_UUID>,
    create: bool,
    hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
    dfh: Option<&TeeFsDirfileFileh>,
) -> TeeResult<Box<TeeFsFd>> {
    tee_debug!(
        "ree_fs_open_primitive: uuid: {:?}, create: {:?}, hash: {:?}, dfh: {:?}",
        uuid,
        create,
        hash,
        dfh
    );

    let mut fdp = Box::new(TeeFsFd::default());
    if let Some(uuid_val) = uuid {
        fdp.uuid = *uuid_val;
    }

    let fd = if create {
        tee_fs_rpc_create_dfh(dfh)
    } else {
        tee_fs_rpc_open_dfh(dfh)
    };

    tee_debug!("ree_fs_open_primitive: fd: {:?}", fd);

    let fd = match fd {
        Ok(fd) => fd,
        Err(e) => return Err(e),
    };

    fdp.fd = Box::new(fd);

    let mut fd_aux = TeeFsFdAux { fd: *fdp.fd };
    let fs_tree = match tee_fs_htree_open(Box::new(fd_aux), create, hash, uuid) {
        Ok(fs_tree) => fs_tree,
        Err(e) => {
            error!("ree_fs_open_primitive: open htree error: {:X?}", e);
            tee_fs_rpc_close(&fdp.fd)?;
            tee_fs_rpc_remove_dfh(dfh)?;
            // drop(fdp);  no need
            return Err(e);
        }
    };

    if let Some(dfh_val) = dfh {
        fdp.dfh = *dfh_val;
    }
    fdp.ht = fs_tree;

    Ok(fdp)
}

fn out_of_place_write(
    fdp: &mut TeeFsFd,
    pos: usize,
    buf_core: &[u8],
    buf_user: &[u8],
    len: usize,
) -> TeeResult {
    tee_debug!(
        "out_of_place_write: fdp.fd: {:?}, pos: {:?}, len: {:?}, buf_core: {:?}, buf_user: {:?}",
        fdp.fd,
        pos,
        len,
        slice_fmt(&buf_core),
        slice_fmt(&buf_user)
    );
    // It doesn't make sense to call this function if nothing is to be
    // written. This also guards against end_block_num getting an
    // unexpected value when pos == 0 and len == 0.
    if len == 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    let start_block_num = pos_to_block_num(pos);
    let end_block_num = pos_to_block_num(pos + len - 1);
    let mut remain_bytes = len;
    let meta = tee_fs_htree_get_meta(&mut fdp.ht);

    let mut block = get_tmp_block().map_err(|_| TEE_ERROR_OUT_OF_MEMORY)?;
    let mut current_pos = pos;
    let mut current_start_block_num = start_block_num;
    let meta_length = meta.length;

    while current_start_block_num <= end_block_num {
        let offset = current_pos % BLOCK_SIZE;
        let mut size_to_write = core::cmp::min(remain_bytes, BLOCK_SIZE);

        if size_to_write + offset > BLOCK_SIZE {
            size_to_write = BLOCK_SIZE - offset;
        }

        // 如果块在现有文件范围内，先读取
        if current_start_block_num * BLOCK_SIZE < roundup_u(meta_length as usize, BLOCK_SIZE) {
            tee_fs_htree_read_block(&mut fdp.ht, current_start_block_num, &mut *block)?;
        } else {
            // 新块，初始化为0
            block.fill(0);
        }

        // 复制数据到块中
        if !buf_core.is_empty() {
            let buf_offset = buf_core.len() - remain_bytes;
            let src_slice = &buf_core[buf_offset..buf_offset + size_to_write];
            let dst_slice = &mut block[offset..offset + size_to_write];
            dst_slice.copy_from_slice(src_slice);
        } else if !buf_user.is_empty() {
            let buf_offset = buf_user.len() - remain_bytes;
            let src_slice = &buf_user[buf_offset..buf_offset + size_to_write];
            let dst_slice = &mut block[offset..offset + size_to_write];
            copy_from_user(dst_slice, src_slice, size_to_write)?;
        } else {
            // 如果buf为空，填充0
            block[offset..offset + size_to_write].fill(0);
        }

        // 写入块
        tee_fs_htree_write_block(
            &mut fdp.ht,
            // &mut fdp.fd,
            current_start_block_num,
            &mut *block,
        )?;

        remain_bytes -= size_to_write;
        current_start_block_num += 1;
        current_pos += size_to_write;
    }

    // 更新文件长度
    let meta = tee_fs_htree_get_meta(&mut fdp.ht);
    if current_pos > meta.length as usize {
        meta.length = current_pos as u64;
        tee_fs_htree_meta_set_dirty(fdp.ht.as_mut());
    }

    put_tmp_block(block);
    Ok(())
}

pub fn ree_fs_read_primitive(
    fh: &mut TeeFsFd,
    pos: usize,
    buf_core: &mut [u8],
    buf_user: &mut [u8],
    len: &mut usize,
) -> TeeResult {
    tee_debug!(
        "ree_fs_read_primitive: fh.fd: {:?}, pos: {:?}, buf_core_len: {:?}, buf_user_len: {:?}, \
         len: {:?}",
        fh.fd,
        pos,
        buf_core.len(),
        buf_user.len(),
        len
    );
    let mut remain_bytes = *len;
    let meta = tee_fs_htree_get_meta(&mut fh.ht);

    // One of buf_core and buf_user must be NULL
    debug_assert!(buf_core.len() > 0 || buf_user.len() > 0);

    tee_debug!("ree_fs_read_primitive: check boundary conditions");
    // 检查边界条件
    if (pos + remain_bytes) < remain_bytes || pos > meta.length as usize {
        remain_bytes = 0;
    } else if pos + remain_bytes > meta.length as usize {
        remain_bytes = meta.length as usize - pos;
    }

    tee_debug!(" pos: {:?}, remain_bytes: {:?}", pos, remain_bytes);
    // 实际读取的数据长度
    *len = remain_bytes;

    if remain_bytes == 0 {
        return Ok(());
    }

    let mut pos = pos;
    let mut start_block_num = pos_to_block_num(pos);
    let end_block_num = pos_to_block_num(pos + remain_bytes - 1);

    tee_debug!(
        "ree_fs_read_primitive: start_block_num: {:?}, end_block_num: {:?}",
        start_block_num,
        end_block_num
    );
    let mut block = get_tmp_block().map_err(|e| {
        error!("Failed to allocate temporary block: {:?}", e);
        TEE_ERROR_OUT_OF_MEMORY
    })?;

    let mut buf_offset = 0;

    while start_block_num <= end_block_num {
        let offset = pos % BLOCK_SIZE;
        let mut size_to_read = core::cmp::min(remain_bytes, BLOCK_SIZE);

        if size_to_read + offset > BLOCK_SIZE {
            size_to_read = BLOCK_SIZE - offset;
        }

        // 读取数据块
        tee_fs_htree_read_block(&mut fh.ht, start_block_num, &mut *block)?;

        // 复制数据到目标缓冲区
        if !buf_core.is_empty() {
            let block_slice = &block[offset..offset + size_to_read];
            let buf_slice = &mut buf_core[buf_offset..buf_offset + size_to_read];
            buf_slice.copy_from_slice(block_slice);
        } else if !buf_user.is_empty() {
            let block_slice = &block[offset..offset + size_to_read];
            let buf_slice = &mut buf_user[buf_offset..buf_offset + size_to_read];
            copy_to_user(buf_slice, block_slice, size_to_read)?;
        }

        remain_bytes -= size_to_read;
        pos += size_to_read;
        buf_offset += size_to_read;
        start_block_num += 1;
    }

    put_tmp_block(block);
    Ok(())
}

pub fn ree_fs_ftruncate_internal(fdp: &mut TeeFsFd, new_file_len: usize) -> TeeResult {
    let meta_length = {
        let meta = tee_fs_htree_get_meta(&mut fdp.ht);
        meta.length
    };

    if new_file_len as u64 > meta_length {
        // 文件扩展路径
        let ext_len = new_file_len - meta_length as usize;
        out_of_place_write(fdp, meta_length as usize, &[], &[], ext_len)?;
    } else {
        // 文件截断路径
        let (offs, sz) = get_offs_size(
            TeeFsHtreeType::Block,
            roundup_u(new_file_len, BLOCK_SIZE) / BLOCK_SIZE,
            1,
        )?;
        tee_fs_htree_truncate(&mut fdp.ht, new_file_len / BLOCK_SIZE)?;
        tee_fs_rpc_truncate(&mut fdp.fd, offs + sz)?;
        let meta = tee_fs_htree_get_meta(&mut fdp.ht);
        meta.length = new_file_len as u64;
        tee_fs_htree_meta_set_dirty(&mut fdp.ht);
    }
    Ok(())
}

pub fn ree_fs_write_primitive(
    fdp: &mut TeeFsFd,
    pos: usize,
    buf_core: &[u8],
    buf_user: &[u8],
    len: usize,
) -> TeeResult {
    debug_assert!(buf_core.len() > 0 || buf_user.len() > 0);

    if len == 0 {
        return Ok(());
    }

    let file_size = tee_fs_htree_get_meta(&mut fdp.ht).length;

    if (pos + len) < len {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    tee_debug!(
        "ree_fs_write_primitive: file_size: {:?}, pos: {:?}",
        file_size,
        pos
    );
    if (file_size as usize) < pos {
        ree_fs_ftruncate_internal(fdp, pos)?;
    }

    out_of_place_write(fdp, pos, buf_core, buf_user, len)
}

pub fn ree_fs_close_primitive(_fdp: &mut TeeFsFd) {
    // tee_fs_htree_close(&mut fdp.ht);
    // tee_fs_rpc_close(&fdp.fd);
    // free(fdp);
}

pub fn ree_dirf_commit_writes(
    fh: &mut TeeFsFd,
    hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
) -> TeeResult {
    tee_fs_htree_sync_to_storage(&mut fh.ht, Some(&mut fh.dfh.hash))?;

    if let Some(h) = hash {
        h.copy_from_slice(&fh.dfh.hash);
    }

    Ok(())
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

/// Implementation of TeeFsDirfileOperations for REE file system
#[derive(Copy, Clone, Debug, Default)]
pub struct ReeDirfOps;

impl TeeFsDirfileOperations for ReeDirfOps {
    fn open(
        &self,
        create: bool,
        hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
        uuid: Option<&TEE_UUID>,
        dfh: Option<&TeeFsDirfileFileh>,
    ) -> TeeResult<Box<TeeFsFd>> {
        tee_debug!(
            "ReeDirfOps::open: create: {}, hash: {:?}, uuid: {:?}, dfh: {:?}",
            create,
            hash,
            uuid,
            dfh
        );
        ree_fs_open_primitive(uuid, create, hash, dfh)
    }

    fn close(&self, fh: &mut TeeFsFd) {
        ree_fs_close_primitive(fh);
    }

    fn read(&self, fh: &mut TeeFsFd, pos: usize, buf: &mut [u8], len: &mut usize) -> TeeResult {
        ree_fs_read_primitive(fh, pos, buf, &mut [], len).inspect_err(|e| {
            error!("ReeDirfOps::read: error: {:X?}", e);
        })
    }

    fn write(&self, fh: &mut TeeFsFd, pos: usize, buf: &[u8]) -> TeeResult {
        ree_fs_write_primitive(fh, pos, buf, &[], buf.len())
    }

    fn commit_writes(
        &self,
        fh: &mut TeeFsFd,
        hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
    ) -> TeeResult {
        ree_dirf_commit_writes(fh, hash)
    }
}

/// tee_file_operations is the operations of the tee_pobj
#[derive(Debug)]
pub struct TeeFileOperations {
    pub name: &'static str,

    pub open: fn(po: &mut tee_pobj, size: Option<&mut usize>) -> TeeResult<Box<TeeFsFd>>,

    pub create: fn(
        po: &mut tee_pobj,
        overwrite: bool,
        head: &[u8],
        attr: &[u8],
        data_core: &[u8],
        data_user: &[u8],
        data_size: usize,
    ) -> TeeResult<Box<TeeFsFd>>,

    pub close: fn(fh: &mut Option<Box<TeeFsFd>>),

    pub read: fn(
        fh: &mut TeeFsFd,
        pos: usize,
        buf_core: &mut [u8],
        buf_user: &mut [u8],
        len: &mut usize,
    ) -> TeeResult,

    pub write:
        fn(fh: &mut TeeFsFd, pos: usize, buf_core: &[u8], buf_user: &[u8], len: usize) -> TeeResult,

    pub rename: fn(old: &mut tee_pobj, new: &tee_pobj, overwrite: bool) -> TeeResult,

    pub remove: fn(po: &tee_pobj) -> TeeResult,

    pub truncate: fn(fh: &mut TeeFsFd, size: usize) -> TeeResult,

    pub opendir: fn(uuid: &TEE_UUID) -> TeeResult<Box<TeeFsDir>>,

    pub readdir: fn(d: &mut TeeFsDir, ent: &mut tee_fs_dirent) -> TeeResult,

    pub closedir: fn(d: &mut TeeFsDir) -> TeeResult,
    #[cfg(feature = "tee_test")]
    pub echo: fn() -> String,
}

pub fn ree_fs_open(po: &mut tee_pobj, size: Option<&mut usize>) -> TeeResult<Box<TeeFsFd>> {
    tee_debug!("ree_fs_open: po: {:?}", po);
    // lock, for thread (process) safety
    let _guard = REE_FS_MUTEX.lock();

    let dirh_ptr = get_dirh()?;
    tee_debug!("ree_fs_open: dirh_ptr: {:?}", dirh_ptr);
    let dirh = unsafe {
        assert!(!dirh_ptr.is_null());
        &mut *dirh_ptr
    };

    let fd = (|| -> TeeResult<Box<TeeFsFd>> {
        // let dfh_opt = tee_fs_dirfile_find(dirh, &po.uuid, po.obj_id)?;
        let mut dfh = tee_fs_dirfile_find(dirh, &po.uuid, &po.obj_id)?;

        let mut hash_buf = dfh.hash;
        let mut fdp = ree_fs_open_primitive(Some(&po.uuid), false, Some(&mut hash_buf), Some(&dfh))
            .map_err(|err| {
                if err == TEE_ERROR_ITEM_NOT_FOUND {
                    TEE_ERROR_CORRUPT_OBJECT
                } else {
                    err
                }
            })?;
        dfh.hash.copy_from_slice(&hash_buf);

        if let Some(size_ref) = size {
            let meta = tee_fs_htree_get_meta(&mut fdp.ht);
            *size_ref = meta.length as usize;
        }

        Ok(fdp)
    })()
    .map_err(|err| {
        put_dirh(&dirh, true);
        err
    })?;

    Ok(fd)
}

/// 为安全文件（fdp 对应的文件）在目录文件（dirh）中设置一个名字 po.obj_id，并同步目录、清理旧文件。
/// 若同名文件已存在，是否允许覆盖由 overwrite 参数决定。
///
/// dirh: 目录文件（directory file）句柄，代表当前 TEE 内部文件系统目录。
/// fdp: 文件描述符（file descriptor），对应正在创建或修改的文件。
/// po: TEE 持久化对象（Persistent Object），包含 UUID、对象 ID（文件名）、长度等。
/// overwrite: 布尔值，指示是否允许覆盖同名文件。
fn set_name(
    dirh: &mut TeeFsDirfileDirh,
    fdp: &mut TeeFsFd,
    po: &tee_pobj,
    overwrite: bool,
) -> TeeResult {
    tee_debug!(
        "set_name: dirh: {:?}, fdp: {:?}, po: {:?}, overwrite: {}",
        dirh,
        fdp,
        po,
        overwrite
    );
    let mut have_old_dfh = false;

    let old_dfh = tee_fs_dirfile_find(dirh, &po.uuid, &po.obj_id);

    tee_debug!("set_name: old_dfh: {:X?}", old_dfh);

    // find old dfh, if not overwrite, return error
    if !overwrite && old_dfh.is_ok() {
        return Err(TEE_ERROR_ACCESS_CONFLICT);
    }

    // if old dfh is found, set have_old_dfh to true
    if old_dfh.is_ok() {
        have_old_dfh = true;
    }

    let mut old_dfh = old_dfh.unwrap_or(TeeFsDirfileFileh {
        idx: -1,
        hash: [0; TEE_FS_HTREE_HASH_SIZE],
        file_number: 0,
    });

    // If old_dfh wasn't found, the idx will be -1 and
    // tee_fs_dirfile_rename() will allocate a new index.
    fdp.dfh.idx = old_dfh.idx;
    old_dfh.idx = -1;
    tee_fs_dirfile_rename(dirh, &po.uuid, &mut fdp.dfh, &po.obj_id)?;
    commit_dirh_writes(dirh)?;

    if have_old_dfh {
        tee_debug!("set_name: remove old_dfh: {:?}", old_dfh);
        tee_fs_rpc_remove_dfh(Some(&old_dfh))?;
    }

    Ok(())
}

fn ree_fs_create(
    po: &mut tee_pobj,
    overwrite: bool,
    head: &[u8],
    attr: &[u8],
    data_core: &[u8],
    data_user: &[u8],
    data_size: usize,
) -> TeeResult<Box<TeeFsFd>> {
    // One of data_core and data_user must be NULL
    debug_assert!(data_core.is_empty() || data_user.is_empty());

    let _guard = REE_FS_MUTEX.lock();

    let mut dfh = TeeFsDirfileFileh::default();
    let mut fdp: Option<Box<TeeFsFd>> = None;
    let dirh_ptr = get_dirh()?;
    let dirh = unsafe {
        assert!(!dirh_ptr.is_null());
        &mut *dirh_ptr
    };

    (|| -> TeeResult<Box<TeeFsFd>> {
        tee_fs_dirfile_get_tmp(dirh, &mut dfh)?;

        let mut hash_buf = dfh.hash;
        let opened_fdp =
            ree_fs_open_primitive(Some(&po.uuid), true, Some(&mut hash_buf), Some(&dfh))?;
        dfh.hash.copy_from_slice(&hash_buf);

        // 标记 fdp 已成功打开，对应 C 版本中的 *fh != NULL
        fdp = Some(opened_fdp);
        let fdp_val = fdp.as_mut().ok_or(TEE_ERROR_GENERIC)?;

        let mut pos = 0;

        if !head.is_empty() {
            ree_fs_write_primitive(fdp_val, pos, head, &[], head.len())?;
            pos += head.len();
        }

        if !attr.is_empty() {
            ree_fs_write_primitive(fdp_val, pos, attr, &[], attr.len())?;
            pos += attr.len();
        }

        if (!data_core.is_empty() || !data_user.is_empty()) && data_size > 0 {
            ree_fs_write_primitive(fdp_val, pos, data_core, data_user, data_size)?;
        }

        tee_debug!("ree_fs_create: sync to storage");
        tee_fs_htree_sync_to_storage(&mut fdp_val.ht, Some(&mut fdp_val.dfh.hash))?;

        tee_debug!("ree_fs_create: set name");
        set_name(dirh, fdp_val, po, overwrite).inspect_err(|e| {
            error!("ree_fs_create: set name failed: {:X?}", e);
        })?;

        // 成功时取出 fdp，使用 take() 避免移动后无法在 map_err 中使用
        Ok(fdp.take().ok_or(TEE_ERROR_GENERIC)?)
    })()
    .map_err(|err| {
        // 错误处理：对应 C 版本中的 if (*fh) 检查
        put_dirh(&dirh, true);
        if let Some(ref mut fdp_val) = fdp {
            ree_fs_close_primitive(fdp_val);
            tee_fs_rpc_remove_dfh(Some(&dfh)).ok();
        }
        err
    })
}

pub fn ree_fs_close(fh: &mut Option<Box<TeeFsFd>>) {
    if let Some(mut fdp) = fh.take() {
        // 获取互斥锁，guard 在作用域结束时自动释放
        let _guard = REE_FS_MUTEX.lock();
        let _ = put_dirh_primitive(false);
        ree_fs_close_primitive(&mut fdp);
        // fh 已经在 take() 时设置为 None，无需手动设置
        // TODO: fdp (Box<TeeFsFd>) 此时未受_guard 保护，是否需要额外处理（drop fdp）？
    }
}

pub fn ree_fs_read(
    fh: &mut TeeFsFd,
    pos: usize,
    buf_core: &mut [u8],
    buf_user: &mut [u8],
    len: &mut usize,
) -> TeeResult {
    let _guard = REE_FS_MUTEX.lock();
    ree_fs_read_primitive(fh, pos, buf_core, buf_user, len)
}

pub fn ree_fs_write(
    fh: &mut TeeFsFd,
    pos: usize,
    buf_core: &[u8],
    buf_user: &[u8],
    len: usize,
) -> TeeResult {
    debug_assert!(!buf_core.is_empty() || !buf_user.is_empty());
    tee_debug!("ree_fs_write: fh: {:?}, pos: {:?}, len: {:?}", fh, pos, len);

    let _guard = REE_FS_MUTEX.lock();
    let dirh_ptr = get_dirh()?;
    let dirh = unsafe {
        assert!(!dirh_ptr.is_null());
        &mut *dirh_ptr
    };
    tee_debug!("ree_fs_write: dirh: {:?}", dirh_ptr);

    let ret = (|| -> TeeResult {
        ree_fs_write_primitive(fh, pos, buf_core, buf_user, len).inspect_err(|e| {
            error!("ree_fs_write: write primitive failed: {:#010X}", e);
        })?;
        tee_fs_htree_sync_to_storage(&mut fh.ht, Some(&mut fh.dfh.hash)).inspect_err(|e| {
            error!("ree_fs_write: sync to storage failed: {:#010X}", e);
        })?;
        tee_fs_dirfile_update_hash(dirh, &mut fh.dfh).inspect_err(|e| {
            error!("ree_fs_write: update hash failed: {:#010X}", e);
        })?;
        commit_dirh_writes(dirh).inspect_err(|e| {
            error!("ree_fs_write: commit writes failed: {:#010X}", e);
        })?;

        Ok(())
    })();

    put_dirh(&dirh, ret.is_err());
    ret
}

pub fn ree_fs_truncate(fh: &mut TeeFsFd, len: usize) -> TeeResult {
    let _guard = REE_FS_MUTEX.lock();
    let dirh_ptr = get_dirh()?;
    let dirh = unsafe {
        assert!(!dirh_ptr.is_null());
        &mut *dirh_ptr
    };

    let ret = (|| -> TeeResult {
        ree_fs_ftruncate_internal(fh, len)?;

        tee_fs_htree_sync_to_storage(&mut fh.ht, Some(&mut fh.dfh.hash))?;

        tee_fs_dirfile_update_hash(dirh, &mut fh.dfh)?;
        commit_dirh_writes(dirh)?;

        Ok(())
    })();

    put_dirh(&dirh, ret.is_err());
    ret
}

pub fn ree_fs_rename(old: &mut tee_pobj, new: &tee_pobj, overwrite: bool) -> TeeResult {
    let _guard = REE_FS_MUTEX.lock();
    let dirh_ptr = get_dirh()?;
    let dirh = unsafe {
        assert!(!dirh_ptr.is_null());
        &mut *dirh_ptr
    };

    let mut remove_dfh = TeeFsDirfileFileh {
        idx: -1,
        hash: [0; TEE_FS_HTREE_HASH_SIZE],
        file_number: 0,
    };

    let ret = (|| -> TeeResult {
        let new_dfh = tee_fs_dirfile_find(dirh, &new.uuid, &new.obj_id);

        if new_dfh.is_ok() && !overwrite {
            return Err(TEE_ERROR_ACCESS_CONFLICT);
        }

        let mut dfh = tee_fs_dirfile_find(dirh, &old.uuid, &old.obj_id)?;

        tee_fs_dirfile_rename(dirh, &new.uuid, &mut dfh, &new.obj_id)?;

        if remove_dfh.idx != -1 {
            tee_fs_dirfile_remove(dirh, &remove_dfh)?;
        }

        commit_dirh_writes(dirh)?;

        if remove_dfh.idx != -1 {
            tee_fs_rpc_remove_dfh(Some(&remove_dfh))?;
        }

        Ok(())
    })();

    put_dirh(&dirh, ret.is_err());
    ret
}

pub fn ree_fs_remove(po: &tee_pobj) -> TeeResult {
    tee_debug!("ree_fs_remove: po: {:?}", po);

    let _guard = REE_FS_MUTEX.lock();
    let dirh_ptr = get_dirh()?;
    let dirh = unsafe {
        assert!(!dirh_ptr.is_null());
        &mut *dirh_ptr
    };

    let ret = (|| -> TeeResult {
        let mut dfh = tee_fs_dirfile_find(dirh, &po.uuid, &po.obj_id)?;

        tee_fs_dirfile_remove(dirh, &dfh)?;

        commit_dirh_writes(dirh)?;

        tee_fs_rpc_remove_dfh(Some(&dfh))?;

        Ok(())
    })();

    put_dirh(&dirh, ret.is_err());
    ret
}

#[cfg(feature = "tee_test")]
fn ree_fs_echo() -> String {
    "TeeFileOperations->echo".to_string()
}

// global file_ops for REE FS, in starryos REE is  starryos self
pub static REE_FS_OPS: TeeFileOperations = TeeFileOperations {
    name: "REE_FS_OPS",
    open: ree_fs_open,
    create: ree_fs_create,
    close: ree_fs_close,
    read: ree_fs_read,
    write: ree_fs_write,
    truncate: ree_fs_truncate,
    rename: ree_fs_rename,
    remove: ree_fs_remove,
    opendir: ree_fs_opendir_rpc,
    closedir: ree_fs_closedir_rpc,
    readdir: ree_fs_readdir_rpc,
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

/// 打开目录句柄
fn open_dirh() -> TeeResult<Box<TeeFsDirfileDirh>> {
    let ree_dir_ops = ReeDirfOps;
    match tee_fs_dirfile_open(false, None, &ree_dir_ops) {
        Ok(dirh) => Ok(Box::new(*dirh)),
        Err(TEE_ERROR_ITEM_NOT_FOUND) => {
            tee_debug!("open_dirh: TEE_ERROR_ITEM_NOT_FOUND, create new dirh");
            let dirh = tee_fs_dirfile_open(true, None, &ree_dir_ops)?;
            tee_debug!("open_dirh: create new dirh: {:?}", dirh);
            Ok(Box::new(*dirh))
        }
        Err(e) => Err(e),
    }
}

fn close_dirh(dirh: &mut Box<TeeFsDirfileDirh>) -> TeeResult {
    // 关闭目录句柄
    tee_fs_dirfile_close(&mut **dirh)
    //*dirh = Arc::new(TeeFsDirfileDirh::default());
}

fn commit_dirh_writes(dirh: &mut TeeFsDirfileDirh) -> TeeResult {
    tee_fs_dirfile_commit_writes(dirh, None)
}

/// Process level directory handle cache
/// Using scope_local! to implement, the directory handle will be automatically cleaned up when the process exits, solving the fd invalid problem
///
/// Different from OP-TEE:
/// - OP-TEE: ree_fs_dirh is a global variable in the TEE kernel, shared by multiple TAs
/// - StarryOS: DIR_HANDLE_MANAGER is a process level variable, each process is independent
///
/// This design has the following advantages:
/// 1. The fd and cache life cycle are consistent, and the cache will be automatically cleaned up when the process exits
/// 2. No need to manually call reset
/// 3. Code is more concise
///
/// TODO: multi-process support must be implemented
pub struct ReeFsDirh {
    /// Directory handle cache
    handle: Option<Box<TeeFsDirfileDirh>>,
    /// Reference count (for compatibility with the put_dirh semantic of the C version)
    refcount: usize,
}

impl Default for ReeFsDirh {
    fn default() -> Self {
        Self::new()
    }
}

impl ReeFsDirh {
    pub const fn new() -> Self {
        Self {
            handle: None,
            refcount: 0,
        }
    }

    /// Get directory handle, if not exist, open it
    pub fn get_dirh(&mut self) -> TeeResult<*mut TeeFsDirfileDirh> {
        if self.handle.is_none() {
            let dirh = open_dirh()?;
            self.handle = Some(dirh);
        }
        self.refcount += 1;

        let h = self.handle.as_mut().unwrap().as_mut() as *mut TeeFsDirfileDirh;
        tee_debug!(
            "get_dirh: h with refcount: {:?} and h: {:?}",
            self.refcount,
            h
        );
        Ok(h)
    }

    /// Release directory handle reference
    pub fn put_dirh_primitive(&mut self, close: bool) -> TeeResult {
        if self.refcount == 0 {
            warn!("put_dirh_primitive: refcount already 0 (double free or logic error)");
            return Ok(());
        }

        self.refcount -= 1;

        // Only these two cases need to be released:
        // 1) Normal reference count reaches zero
        // 2) close == true (force release)
        if self.refcount == 0 || close {
            if let Some(mut dirh) = self.handle.take() {
                close_dirh(&mut dirh)?;
            }
            self.refcount = 0;
        }

        Ok(())
    }
}

impl Drop for ReeFsDirh {
    fn drop(&mut self) {
        // 进程退出时自动清理
        if let Some(mut dirh) = self.handle.take() {
            tee_debug!("ReeFsDirh::drop: cleaning up cached dirh");
            let _ = close_dirh(&mut dirh);
        }
    }
}

// 进程级目录句柄管理器
// 使用 scope_local! 实现进程级存储，进程退出时自动清理
// 使用 Arc<Mutex<...>> 与 TEE_FD_TABLE 的设计保持一致
scope_local! {
    /// 进程级目录句柄缓存
    /// 使用 Arc<Mutex<...>> 保证线程安全，与 TEE_FD_TABLE 设计一致
    pub static DIR_HANDLE_MANAGER: Arc<Mutex<ReeFsDirh>> = Arc::new(Mutex::new(ReeFsDirh::new()));
}

/// 获取目录句柄
pub fn get_dirh() -> TeeResult<*mut TeeFsDirfileDirh> {
    DIR_HANDLE_MANAGER.lock().get_dirh()
}

/// 释放目录句柄引用
pub fn put_dirh_primitive(close: bool) -> TeeResult {
    DIR_HANDLE_MANAGER.lock().put_dirh_primitive(close)
}

/// 释放目录句柄
pub fn put_dirh(_dirh: &TeeFsDirfileDirh, close: bool) {
    let _ = put_dirh_primitive(close);
}

pub fn ree_fs_opendir_rpc(uuid: &TEE_UUID) -> TeeResult<Box<TeeFsDir>> {
    let mut d = Box::new(TeeFsDir::default());

    // d->uuid = uuid; (如果 uuid 是 NULL，则使用默认值)
    d.uuid = *uuid;

    let _guard = REE_FS_MUTEX.lock();

    // res = get_dirh(&d->dirh);
    let dirh_ptr = get_dirh()?;
    d.dirh = dirh_ptr;

    let dirh = unsafe {
        assert!(!dirh_ptr.is_null());
        &mut *dirh_ptr
    };

    // See that there's at least one file
    // d->idx = -1;
    d.idx = -1;
    d.d.oid_len = d.d.oid.len() as u32;
    // 在 Rust 中，oidlen 是返回值，不需要预先设置

    // res = tee_fs_dirfile_get_next(d->dirh, d->uuid, &d->idx, d->d.oid, &d->d.oidlen);
    let res = tee_fs_dirfile_get_next(dirh, &d.uuid, &mut d.idx, &mut d.d.oid);

    d.idx = -1;

    // 处理结果
    match res {
        Ok(oid_len) => {
            d.d.oid_len = oid_len as u32;
            // 成功：*dir = d;
            Ok(d)
        }
        Err(e) => {
            // 失败：put_dirh(d->dirh, false); free(d);
            put_dirh(dirh, false);
            Err(e)
        }
    }
}

pub fn ree_fs_closedir_rpc(d: &mut TeeFsDir) -> TeeResult {
    let _guard = REE_FS_MUTEX.lock();
    let dirh_ptr = d.dirh;
    if !dirh_ptr.is_null() {
        let dirh = unsafe {
            assert!(!dirh_ptr.is_null());
            &mut *dirh_ptr
        };
        put_dirh(dirh, false);
    }

    Ok(())
}

pub fn ree_fs_readdir_rpc(d: &mut TeeFsDir, ent: &mut tee_fs_dirent) -> TeeResult {
    let _guard = REE_FS_MUTEX.lock();
    let dirh_ptr = d.dirh;
    let dirh = unsafe {
        assert!(!dirh_ptr.is_null());
        &mut *dirh_ptr
    };

    ent.oid_len = ent.oid.len() as u32;
    let res = tee_fs_dirfile_get_next(dirh, &d.uuid, &mut d.idx, &mut d.d.oid);

    match res {
        Ok(oid_len) => {
            ent.oid_len = oid_len as u32;
            ent.oid[..oid_len].copy_from_slice(&d.d.oid[..oid_len]);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[cfg(feature = "tee_test")]
pub mod tests_tee_ree_fs {
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{TestDescriptor, TestResult},
        test_fn, tests, tests_name,
    };

    const NODE_SIZE_TEST: usize = size_of::<super::TeeFsHtreeNodeImage>(); // 66
    const HTREE_IMAGE_SIZE_TEST: usize = size_of::<super::TeeFsHtreeImage>(); // 256
    const BLOCK_NODES_TEST: usize = BLOCK_SIZE / (NODE_SIZE_TEST * 2); // 4096 / (66 * 2) = 31

    test_fn! {
        using TestResult;
        fn test_get_offs_size_head() {
            // Case 1: Head type, version 0
            let result = get_offs_size(TeeFsHtreeType::Head, 0, 0);
            assert_eq!(result.unwrap(), (0 as usize, HTREE_IMAGE_SIZE_TEST)); // (0, 256)

            // Case 2: Head type, version 1
            let result = get_offs_size(TeeFsHtreeType::Head, 0, 1);
            assert_eq!(result.unwrap(), (HTREE_IMAGE_SIZE_TEST, HTREE_IMAGE_SIZE_TEST)); // (256, 256)

            // Case 3: Head type, arbitrary idx (should be ignored for Head)
            let result = get_offs_size(TeeFsHtreeType::Head, 100, 0);
            assert_eq!(result.unwrap(), (0, HTREE_IMAGE_SIZE_TEST)); // (0, 256)
        }
    }

    test_fn! {
        using TestResult;
        fn test_get_offs_size_node() {
            // Case 4: Node type, first node in first block_nodes group (idx 0), version 0
            // pbn = 1 + ((0 / 31) * 31 * 2) = 1
            // offs = 1 * 4096 + 2 * 66 * (0 % 31) + 66 * 0 = 4096 + 0 + 0 = 4096
            let result = get_offs_size(TeeFsHtreeType::Node, 0, 0);
            assert_eq!(result.unwrap(), (4096, NODE_SIZE_TEST));

            // Case 5: Node type, first node in first block_nodes group (idx 0), version 1
            // offs = 1 * 4096 + 2 * 66 * 0 + 66 * 1 = 4096 + 66 = 4162
            let result = get_offs_size(TeeFsHtreeType::Node, 0, 1);
            assert_eq!(result.unwrap(), (4162, NODE_SIZE_TEST));

            // Case 6: Node type, last node in first block_nodes group (idx 30), version 0
            // pbn = 1 + ((30 / 31) * 31 * 2) = 1
            // offs = 1 * 4096 + 2 * 66 * (30 % 31) + 66 * 0 = 4096 + 3960 = 8056
            let result = get_offs_size(TeeFsHtreeType::Node, 30, 0);
            assert_eq!(result.unwrap(), (8056, NODE_SIZE_TEST));

            // Case 7: Node type, last node in first block_nodes group (idx 30), version 1
            // offs = 1 * 4096 + 2 * 66 * 30 + 66 * 1 = 4096 + 3960 + 66 = 8122
            let result = get_offs_size(TeeFsHtreeType::Node, 30, 1);
            assert_eq!(result.unwrap(), (8122, NODE_SIZE_TEST));

            // Case 8: Node type, first node in second block_nodes group (idx 31), version 0
            // pbn = 1 + ((31 / 31) * 31 * 2) = 1 + (1 * 62) = 63
            // offs = 63 * 4096 + 2 * 66 * (31 % 31) + 66 * 0 = 25792 + 0 + 0 = 25792
            let result = get_offs_size(TeeFsHtreeType::Node, 31, 0);
            assert_eq!(result.unwrap(), (258048, NODE_SIZE_TEST));

            // Case 9: Node type, first node in second block_nodes group (idx 31), version 1
            // offs = 63 * 4096 + 2 * 66 * 0 + 66 * 1 = 25792 + 66 = 25858
            let result = get_offs_size(TeeFsHtreeType::Node, 31, 1);
            assert_eq!(result.unwrap(), (258114, NODE_SIZE_TEST));
        }
    }

    test_fn! {
        using TestResult;
        fn test_get_offs_size_block() {
            let _block_nodes_x2_minus_1 = BLOCK_NODES_TEST * 2 - 1; // 31 * 2 - 1 = 61

            // Case 10: Block type, first data block (idx 0), version 0
            // bidx = 2 * 0 + 0 = 0
            // pbn = 2 + 0 + 0 / 61 = 2
            // offs = 2 * 4096 = 8192
            let result = get_offs_size(TeeFsHtreeType::Block, 0, 0);
            assert_eq!(result.unwrap(), (8192, BLOCK_SIZE));

            // Case 11: Block type, first data block (idx 0), version 1
            // bidx = 2 * 0 + 1 = 1
            // pbn = 2 + 1 + 1 / 61 = 3
            // offs = 3 * 4096 = 12288
            let result = get_offs_size(TeeFsHtreeType::Block, 0, 1);
            assert_eq!(result.unwrap(), (12288, BLOCK_SIZE));

            // Case 12: Block type, last data block in first block_nodes group (idx 30), version 0
            // bidx = 2 * 30 + 0 = 60
            // pbn = 2 + 60 + 60 / 61 = 62
            // offs = 62 * 4096 = 253952
            let result = get_offs_size(TeeFsHtreeType::Block, 30, 0);
            assert_eq!(result.unwrap(), (253952, BLOCK_SIZE));

            // Case 13: Block type, last data block in first block_nodes group (idx 30), version 1
            // bidx = 2 * 30 + 1 = 61
            // pbn = 2 + 61 + 61 / 61 = 63 + 1 = 64  (Note: this is where the code's logic differs from comments)
            // offs = 64 * 4096 = 262144
            let result = get_offs_size(TeeFsHtreeType::Block, 30, 1);
            assert_eq!(result.unwrap(), (262144, BLOCK_SIZE));

            // Case 14: Block type, first data block in second block_nodes group (idx 31), version 0
            // bidx = 2 * 31 + 0 = 62
            // pbn = 2 + 62 + 62 / 61 = 64 + 1 = 65
            // offs = 65 * 4096 = 266240
            let result = get_offs_size(TeeFsHtreeType::Block, 31, 0);
            assert_eq!(result.unwrap(), (266240, BLOCK_SIZE));

            // Case 15: Block type, first data block in second block_nodes group (idx 31), version 1
            // bidx = 2 * 31 + 1 = 63
            // pbn = 2 + 63 + 63 / 61 = 65 + 1 = 66
            // offs = 66 * 4096 = 270336
            let result = get_offs_size(TeeFsHtreeType::Block, 31, 1);
            assert_eq!(result.unwrap(), (270336, BLOCK_SIZE));
        }
    }

    test_fn! {
        using TestResult;
        fn test_get_offs_size_unsupported_type() {
            // Case 16: Unsupported type should return an error
            let result = get_offs_size(TeeFsHtreeType::UnsupportedType, 0, 0);
            assert_eq!(result.unwrap_err(), TEE_ERROR_GENERIC);
        }
    }

    test_fn! {
        using TestResult;
        fn test_ree_fs_primitive_operations() {
            // use crate::fs_dirfile::TeeFsDirfileFileh;
            // use crate::fs_htree::TEE_FS_HTREE_HASH_SIZE;
            // use crate::tee_api_types::TeeUuid;
            // use crate::tee_ree_fs::{ree_fs_open_primitive, ree_fs_read_primitive, ree_fs_write_primitive};

            // 创建测试用的 UUID 和 DFH
            let uuid = TEE_UUID {
                timeLow: 0x12345678,
                timeMid: 0x1234,
                timeHiAndVersion: 0x5678,
                clockSeqAndNode: [0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78],
            };

            let mut dfh = TeeFsDirfileFileh::default();
            dfh.file_number = 0x12345678;
            dfh.hash = [0x01; TEE_FS_HTREE_HASH_SIZE];
            dfh.idx = 1;

            let mut hash = [0u8; TEE_FS_HTREE_HASH_SIZE];

            // 1. 使用 create = true 调用 ree_fs_open_primitive 创建文件
            let mut fdp = ree_fs_open_primitive(Some(&uuid), true, Some(&mut hash), Some(&dfh)).unwrap();

            // 验证文件创建成功
            assert_eq!(fdp.uuid, uuid);
            assert_eq!(fdp.dfh.file_number, dfh.file_number);

            // 2. 调用 ree_fs_write_primitive 写入数据
            let test_data = b"Hello, TEE World! This is a test message for ree_fs_primitive operations.";
            let write_result = ree_fs_write_primitive(&mut fdp, 0, test_data, &[], test_data.len());
            assert!(write_result.is_ok());

            // 3. 调用 ree_fs_read_primitive 读取数据
            let mut read_buffer = vec![0u8; test_data.len()];
            let mut read_len = read_buffer.len();
            let read_result = ree_fs_read_primitive(&mut fdp, 0, &mut read_buffer, &mut [], &mut read_len);
            assert!(read_result.is_ok());

            // 验证读取的数据长度
            assert_eq!(read_len, test_data.len());

            // 验证读取的数据内容
            assert_eq!(&read_buffer[..read_len], test_data);

            // 4. 测试部分读取
            let mut partial_buffer = vec![0u8; 10];
            let mut partial_len = partial_buffer.len();
            let partial_read_result =
                ree_fs_read_primitive(&mut fdp, 0, &mut partial_buffer, &mut [], &mut partial_len);
            assert!(partial_read_result.is_ok());
            assert_eq!(partial_len, 10);
            assert_eq!(&partial_buffer[..partial_len], &test_data[..10]);

            // 5. 测试从中间位置读取
            let mut mid_buffer = vec![0u8; 15];
            let mut mid_len = mid_buffer.len();
            let mid_read_result = ree_fs_read_primitive(&mut fdp, 10, &mut mid_buffer, &mut [], &mut mid_len);
            assert!(mid_read_result.is_ok());
            assert_eq!(mid_len, 15);
            assert_eq!(&mid_buffer[..mid_len], &test_data[10..25]);

            // 6. 测试写入更多数据
            let additional_data = b" Additional data appended to the file.";
            let additional_write_result =
                ree_fs_write_primitive(&mut fdp, test_data.len(), additional_data, &[], additional_data.len());
            assert!(additional_write_result.is_ok());

            // 7. 读取完整数据（包括追加的数据）
            let total_len = test_data.len() + additional_data.len();
            let mut full_buffer = vec![0u8; total_len];
            let mut full_len = full_buffer.len();
            let full_read_result = ree_fs_read_primitive(&mut fdp, 0, &mut full_buffer, &mut [], &mut full_len);
            assert!(full_read_result.is_ok());
            assert_eq!(full_len, total_len);

            // 验证完整数据
            let mut expected_full_data = Vec::new();
            expected_full_data.extend_from_slice(test_data);
            expected_full_data.extend_from_slice(additional_data);
            assert_eq!(&full_buffer[..full_len], expected_full_data.as_slice());
        }
    }

    tests_name! {
        TEST_TEE_REE_FS;
        //------------------------
        test_get_offs_size_head,
        test_get_offs_size_node,
        test_get_offs_size_block,
        test_get_offs_size_unsupported_type,
        test_ree_fs_primitive_operations,
    }
}
