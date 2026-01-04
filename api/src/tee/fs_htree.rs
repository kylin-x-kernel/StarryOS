// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, vec, vec::Vec};
use core::ptr::NonNull;

use bytemuck::{Pod, Zeroable};
use mbedtls::{
    cipher::{Authenticated, Cipher, CipherData, Decryption, Encryption, Fresh, Operation, raw},
    hash::Md,
};
use memoffset::offset_of;
use subtle::ConstantTimeEq;
use tee_raw_sys::{
    TEE_ALG_AES_ECB_NOPAD, TEE_ALG_AES_GCM, TEE_ALG_HMAC_SHA256, TEE_ALG_SHA256,
    TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_CORRUPT_OBJECT, TEE_ERROR_GENERIC, TEE_ERROR_MAC_INVALID,
    TEE_ERROR_NOT_SUPPORTED, TEE_ERROR_SECURITY, TEE_ERROR_SHORT_BUFFER, TEE_OperationMode,
    TEE_UUID,
};

use super::utee_defines::{TEE_AES_BLOCK_SIZE, TEE_SHA256_HASH_SIZE};
use crate::tee::{
    TeeResult,
    common::file_ops::FileVariant,
    crypto_temp::crypto_temp::{
        crypto_hash_alloc_ctx, crypto_hash_final, crypto_hash_init, crypto_hash_update,
    },
    tee_fs_key_manager::{TEE_FS_KM_FEK_SIZE, tee_fs_fek_crypt},
    tee_ree_fs::{
        BLOCK_SIZE, TeeFsHtreeStorageOps, crypto_rng_read, ree_fs_rpc_read_init as rpc_read_init,
        ree_fs_rpc_write_init as rpc_write_init, tee_fs_rpc_read_final as rpc_read_final,
        tee_fs_rpc_write_final as rpc_write_final,
    },
    utee_defines::TEE_ALG,
};

pub const TEE_FS_HTREE_IV_SIZE: usize = 16;
pub const TEE_FS_HTREE_HASH_SIZE: usize = TEE_SHA256_HASH_SIZE;
pub const TEE_FS_HTREE_FEK_SIZE: usize = 16;
pub const TEE_FS_HTREE_TAG_SIZE: usize = 16;

pub const TEE_FS_HTREE_CHIP_ID_SIZE: usize = 32;
pub const TEE_FS_HTREE_HASH_ALG: TEE_ALG = TEE_ALG_SHA256;
pub const TEE_FS_HTREE_TSK_SIZE: usize = TEE_FS_HTREE_HASH_SIZE;
pub const TEE_FS_HTREE_ENC_ALG: TEE_ALG = TEE_ALG_AES_ECB_NOPAD;
pub const TEE_FS_HTREE_ENC_SIZE: usize = TEE_AES_BLOCK_SIZE;
pub const TEE_FS_HTREE_SSK_SIZE: usize = TEE_FS_HTREE_HASH_SIZE;

pub const HTREE_NODE_COMMITTED_BLOCK: u32 = 1 << 0; // 即 0x1

pub const TEE_FS_HTREE_AUTH_ENC_ALG: TEE_ALG = TEE_ALG_AES_GCM;
pub const TEE_FS_HTREE_HMAC_ALG: TEE_ALG = TEE_ALG_HMAC_SHA256;

#[inline]
fn block_num_to_node_id(num: usize) -> usize {
    num + 1
}

#[allow(dead_code)]
#[inline]
fn node_id_to_block_num(id: usize) -> usize {
    id - 1
}

#[inline]
pub const fn htree_node_committed_child(n: usize) -> u32 {
    1 << (1 + n)
}

// unsafe impl Zeroable for TeeFsHtreeNodeImage {}
// unsafe impl Pod for TeeFsHtreeNodeImage {}
#[repr(C)]
#[derive(Copy, Debug, Clone, Default, Pod, Zeroable)]
pub struct TeeFsHtreeMeta {
    pub length: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct TeeFsHtreeImeta {
    pub meta: TeeFsHtreeMeta,
    pub max_node_id: u32,
    // pub _padding: [u8; 4],
}

pub const TEE_FS_HTREE_IMETA_SIZE: usize = core::mem::size_of::<TeeFsHtreeImeta>();
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, Pod, Zeroable)]
pub struct TeeFsHtreeImage {
    pub iv: [u8; TEE_FS_HTREE_IV_SIZE],
    pub tag: [u8; TEE_FS_HTREE_TAG_SIZE],
    pub enc_fek: [u8; TEE_FS_HTREE_FEK_SIZE],
    pub imeta: [u8; TEE_FS_HTREE_IMETA_SIZE],
    pub counter: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)] // Derive Clone for easy copying if needed
pub struct TeeFsHtreeNodeImage {
    pub hash: [u8; TEE_FS_HTREE_HASH_SIZE],
    pub iv: [u8; TEE_FS_HTREE_IV_SIZE],
    pub tag: [u8; TEE_FS_HTREE_TAG_SIZE],
    pub flags: u16,
}

pub enum TeeFsHtreeType {
    Head,
    Node,
    Block,
    #[allow(dead_code)]
    UnsupportedType,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct TeeFsHtreeData {
    pub head: TeeFsHtreeImage,
    pub fek: [u8; TEE_FS_HTREE_FEK_SIZE],
    pub imeta: TeeFsHtreeImeta,
    pub uuid: TEE_UUID,
    pub dirty: bool,
    // const struct tee_fs_htree_storage *stor;
    // void *stor_aux;
}

#[derive(Debug, Default)]
pub struct HtreeNode {
    pub id: usize,
    pub dirty: bool,
    pub block_updated: bool,
    pub node: TeeFsHtreeNodeImage,
    // parent 使用 NonNull，因为：
    // 1. root 节点的 parent 为 None
    // 2. 子节点的 parent 指向父节点（非拥有关系）
    // 3. 父节点的生命周期由 tee_fs_htree 保证
    pub parent: Option<NonNull<HtreeNode>>,
    // left/right 使用 Box，因为：
    // 1. 子节点由父节点拥有
    // 2. 释放时通过 Box 自动管理
    // 3. 符合 Rust 的所有权模型
    pub left: Subtree,
    pub right: Subtree,
}

impl HtreeNode {
    pub fn new(id: usize, node_image: TeeFsHtreeNodeImage) -> Self {
        HtreeNode {
            id,
            node: node_image,
            ..Default::default()
        }
    }

    pub fn set_left(current_node: &mut HtreeNode, mut child: HtreeNode) {
        child.parent = NonNull::new(current_node as *mut _);
        current_node.left = Some(Box::new(child));
    }

    pub fn set_right(current_node: &mut HtreeNode, mut child: HtreeNode) {
        child.parent = NonNull::new(current_node as *mut _);
        current_node.right = Some(Box::new(child));
    }

    /// 根据索引获取左右子树的引用。
    ///
    /// `index` 为 0 时返回左子树，为 1 时返回右子树。
    /// 如果对应子树不存在，则返回 `None`。
    pub fn get_child_by_index(&mut self, index: usize) -> Option<&mut HtreeNode> {
        if index == 0 {
            self.left.as_mut().map(|b| b.as_mut())
        } else {
            self.right.as_mut().map(|b| b.as_mut())
        }
    }
}

pub type Subtree = Option<Box<HtreeNode>>;

pub trait SubtreeExt {
    fn get_mut(&mut self) -> Option<&mut HtreeNode>;

    fn get_ref(&self) -> Option<&HtreeNode>;
}

impl SubtreeExt for Subtree {
    fn get_mut(&mut self) -> Option<&mut HtreeNode> {
        self.as_deref_mut()
    }

    fn get_ref(&self) -> Option<&HtreeNode> {
        self.as_deref()
    }
}

#[derive(Debug, Default)]
pub struct TeeFsHtree {
    pub root: HtreeNode,
    pub data: TeeFsHtreeData,
}

/// read the data from the storage
///
/// # Arguments
/// * `fd` - the file descriptor
/// * `typ` - the type of the data
/// * `idx` - the index of the data
/// * `vers` - the version of the data
/// * `data` - the data to read
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn rpc_read(
    fd: &mut FileVariant,
    // ht: &mut TeeFsHtree,
    typ: TeeFsHtreeType,
    idx: usize,
    vers: u8,
    data: &mut [u8],
) -> TeeResult {
    let dlen = data.len();
    if dlen == 0 {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    rpc_read_init()?;

    let result = rpc_read_final(fd, typ, idx, vers, data)?;
    if result != dlen {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    Ok(())
}

/// read the head from the storage
///
/// # Arguments
/// * `fd` - the file descriptor
/// * `vers` - the version of the head
/// * `head` - the head of the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn rpc_read_head(
    fd: &mut FileVariant,
    // ht: &mut TeeFsHtree,
    vers: u8,
    head: &mut TeeFsHtreeImage,
) -> TeeResult {
    let data_ptr: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(
            head as *mut TeeFsHtreeImage as *mut u8,
            size_of::<TeeFsHtreeImage>(),
        )
    };
    rpc_read(fd, /* ht, */ TeeFsHtreeType::Head, 0, vers, data_ptr)?;
    Ok(())
}

/// read the node from the storage
///
/// # Arguments
/// * `fd` - the file descriptor
/// * `node_id` - the id of the node
/// * `vers` - the version of the node
/// * `head` - the head of the node
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn rpc_read_node(
    fd: &mut FileVariant,
    // ht: &mut TeeFsHtree,
    node_id: usize,
    vers: u8,
    head: &mut TeeFsHtreeNodeImage,
) -> TeeResult {
    let data_ptr: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(
            head as *mut TeeFsHtreeNodeImage as *mut u8,
            size_of::<TeeFsHtreeNodeImage>(),
        )
    };
    rpc_read(
        fd,
        // ht,
        TeeFsHtreeType::Node,
        node_id - 1,
        vers,
        data_ptr,
    )?;
    Ok(())
}

/// write the data to the storage
///
/// # Arguments
/// * `fd` - the file descriptor
/// * `typ` - the type of the data
/// * `idx` - the index of the data
/// * `vers` - the version of the data
/// * `data` - the data to write
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn rpc_write(
    fd: &FileVariant,
    // ht: &mut TeeFsHtree,
    typ: TeeFsHtreeType,
    idx: usize,
    vers: u8,
    data: &[u8],
) -> TeeResult {
    let dlen = data.len();
    if dlen == 0 {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    rpc_write_init()?;
    let _ = rpc_write_final(fd, typ, idx, vers, data)?;

    Ok(())
}

/// write the head to the storage
///
/// # Arguments
/// * `fd` - the file descriptor
/// * `vers` - the version of the head
/// * `head` - the head of the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn rpc_write_head(
    fd: &FileVariant,
    // ht: &mut TeeFsHtree,
    vers: u8,
    head: &TeeFsHtreeImage,
) -> TeeResult {
    let data_ptr: &[u8] = unsafe {
        core::slice::from_raw_parts(
            head as *const TeeFsHtreeImage as *const u8,
            size_of::<TeeFsHtreeImage>(),
        )
    };
    rpc_write(fd, /* ht, */ TeeFsHtreeType::Head, 0, vers, data_ptr)?;
    Ok(())
}

/// write the node to the storage
///
/// # Arguments
/// * `fd` - the file descriptor
/// * `node_id` - the id of the node
/// * `vers` - the version of the node
/// * `head` - the head of the node
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn rpc_write_node(
    fd: &FileVariant,
    // ht: &mut TeeFsHtree,
    node_id: usize,
    vers: u8,
    head: &TeeFsHtreeNodeImage,
) -> TeeResult {
    let data_ptr: &[u8] = unsafe {
        core::slice::from_raw_parts(
            head as *const TeeFsHtreeNodeImage as *const u8,
            size_of::<TeeFsHtreeNodeImage>(),
        )
    };
    rpc_write(
        fd,
        // ht,
        TeeFsHtreeType::Node,
        node_id - 1,
        vers,
        data_ptr,
    )?;
    Ok(())
}

/// calc the hash of the node
///
/// # Arguments
/// * `node` - the node
/// * `ht_data` - the data of the tree
/// * `fd` - the file descriptor
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn calc_node(
    mut node: &mut HtreeNode,
    ht_data: &TeeFsHtreeData,
    _fd: Option<&mut FileVariant>,
) -> TeeResult {
    let mut digest = [0u8; TEE_FS_HTREE_HASH_SIZE];

    if node.parent.is_some() {
        calc_node_hash_with_type(TEE_FS_HTREE_HASH_ALG, &node, None, &mut digest)?;
    } else {
        calc_node_hash_with_type(
            TEE_FS_HTREE_HASH_ALG,
            &node,
            Some(&ht_data.imeta.meta),
            &mut digest,
        )?;
    }

    node.node.hash.copy_from_slice(&digest);

    Ok(())
}

/// calc the hash of the node with context
///
/// # Arguments
/// * `md` - the hash context
/// * `node` - the node
/// * `meta` - the meta of the tree
/// * `digest` - the digest of the hash
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn calc_node_hash_with_ctx(
    mut md: Md,
    node: &HtreeNode,
    meta: Option<&TeeFsHtreeMeta>,
    digest: &mut [u8; TEE_FS_HTREE_HASH_SIZE],
) -> TeeResult {
    let all_bytes = bytemuck::bytes_of(&node.node);
    let iv_offset = offset_of!(TeeFsHtreeNodeImage, iv);
    let flags_offset = offset_of!(TeeFsHtreeNodeImage, flags);
    let flags_size = core::mem::size_of_val(&node.node.flags);

    crypto_hash_init(&mut md)?;
    crypto_hash_update(&mut md, &all_bytes[iv_offset..flags_offset + flags_size])?;
    if let Some(meta) = meta {
        crypto_hash_update(&mut md, bytemuck::bytes_of(meta))?;
    }

    if let Some(left) = node.left.get_ref() {
        crypto_hash_update(&mut md, &left.node.hash)?;
    }

    if let Some(right) = node.right.get_ref() {
        crypto_hash_update(&mut md, &right.node.hash)?;
    }
    crypto_hash_final(md, digest)?;

    Ok(())
}

/// calc the hash of the node with type
///
/// # Arguments
/// * `t` - the type of the hash
/// * `node` - the node
/// * `meta` - the meta of the tree
/// * `digest` - the digest of the hash
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn calc_node_hash_with_type(
    t: TEE_ALG,
    node: &HtreeNode,
    meta: Option<&TeeFsHtreeMeta>,
    digest: &mut [u8; TEE_FS_HTREE_HASH_SIZE],
) -> TeeResult {
    let md = crypto_hash_alloc_ctx(t)?;
    calc_node_hash_with_ctx(md, node, meta, digest)?;

    Ok(())
}

/// calc the hash of the node
///
/// # Arguments
/// * `node` - the node
/// * `meta` - the meta of the tree
/// * `digest` - the digest of the hash
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn calc_node_hash(
    node: &HtreeNode,
    meta: &TeeFsHtreeMeta,
    digest: &mut [u8; TEE_FS_HTREE_HASH_SIZE],
) -> TeeResult {
    calc_node_hash_with_type(TEE_ALG_SHA256, node, Some(meta), digest)
}

/// traverse the tree post order
///
/// # Arguments
/// * `cb` - the callback function
/// * `node` - the node
/// * `tee_fs_htree` - the tree
/// * `fd` - the file descriptor
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn traverse_post_order<F>(
    mut cb: F,
    node: &mut HtreeNode,
    tee_fs_htree: &mut TeeFsHtree,
    mut fd: Option<&mut FileVariant>,
) -> TeeResult
where
    F: FnMut(&mut TeeFsHtree, &mut HtreeNode, Option<&mut FileVariant>) -> TeeResult,
{
    if let Some(left) = node.left.get_mut() {
        traverse_post_order(&mut cb, left, tee_fs_htree, fd.as_deref_mut())?;
    }

    if let Some(right) = node.right.get_mut() {
        traverse_post_order(&mut cb, right, tee_fs_htree, fd.as_deref_mut())?;
    }

    // 回调当前节点
    let _res = cb(tee_fs_htree, node, fd.as_deref_mut());

    Ok(())
}

/// traverse the tree post order
///
/// # Arguments
/// * `node` - the node
/// * `ht_data` - the data of the tree
/// * `visitor` - the visitor function
/// * `fd` - the file descriptor
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn post_order_traverse<F>(
    node: &HtreeNode,
    ht_data: &TeeFsHtreeData,
    visitor: &mut F,
    mut fd: Option<&mut FileVariant>,
) -> TeeResult
where
    F: FnMut(&HtreeNode, &TeeFsHtreeData, Option<&mut FileVariant>) -> TeeResult,
{
    // 对 fd 做借用变换

    // Traverse left subtree
    if let Some(left_child_arc) = node.left.get_ref() {
        post_order_traverse(left_child_arc, ht_data, visitor, fd.as_deref_mut())?;
    }

    // Traverse right subtree
    if let Some(right_child_arc) = node.right.get_ref() {
        post_order_traverse(right_child_arc, ht_data, visitor, fd.as_deref_mut())?;
    }

    // Visit the current node
    let _ = visitor(node, ht_data, fd.as_deref_mut());

    Ok(())
}

/// traverse the tree post order mut
///
/// # Arguments
/// * `node` - the node
/// * `ht_data` - the data of the tree
/// * `visitor` - the visitor function
/// * `fd` - the file descriptor
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn post_order_traverse_mut<F>(
    node: &mut HtreeNode,
    ht_data: &TeeFsHtreeData,
    visitor: &mut F,
    mut fd: Option<&mut FileVariant>,
) -> TeeResult
where
    F: FnMut(&mut HtreeNode, &TeeFsHtreeData, Option<&mut FileVariant>) -> TeeResult, /* visitor 现在接收 RefMut<HtreeNode> */
{
    // 遍历左子树
    if let Some(left_child_arc) = node.left.get_mut() {
        post_order_traverse_mut(left_child_arc, ht_data, visitor, fd.as_deref_mut())?;
    }

    // 遍历右子树
    if let Some(right_child_arc) = node.right.get_mut() {
        post_order_traverse_mut(right_child_arc, ht_data, visitor, fd.as_deref_mut())?;
    }
    // `try_borrow_mut()` 会返回 Err，这里使用 `ok()` 忽略错误，
    // TODO 实际应用中你可能需要更健壮的错误处理。
    visitor(node, ht_data, fd.as_deref_mut())?; // 将 RefMut<HtreeNode> 传递给 visitor

    Ok(())
}

/// free the node
///
/// # Arguments
/// * `node` - the node
/// * `ht_data` - the data of the tree
/// * `fd` - the file descriptor
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn free_node(
    _node: &HtreeNode,
    _ht_data: &TeeFsHtreeData,
    _fd: Option<&mut FileVariant>,
) -> TeeResult {
    Ok(())
}

/// verify the node
///
/// # Arguments
/// * `node` - the node
/// * `ht_data` - the data of the tree
/// * `fd` - the file descriptor
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn verify_node(
    node: &HtreeNode,
    ht_data: &TeeFsHtreeData,
    _fd: Option<&mut FileVariant>,
) -> TeeResult {
    let mut digest = [0u8; TEE_FS_HTREE_HASH_SIZE];

    if node.parent.is_some() {
        calc_node_hash_with_type(TEE_FS_HTREE_HASH_ALG, node, None, &mut digest)?;
    } else {
        calc_node_hash_with_type(
            TEE_FS_HTREE_HASH_ALG,
            node,
            Some(&ht_data.imeta.meta),
            &mut digest,
        )?;
    }

    debug!(
        "check hash {} with {}",
        hex::encode(node.node.hash),
        hex::encode(digest)
    );

    if node.node.hash.ct_eq(&digest).unwrap_u8() == 0 {
        return Err(TEE_ERROR_CORRUPT_OBJECT);
    }

    Ok(())
}

/// print the hash of the node
///
/// # Arguments
/// * `node` - the node
/// * `ht_data` - the data of the tree
/// * `fd` - the file descriptor
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn print_node_hash(
    node: &HtreeNode,
    ht_data: &TeeFsHtreeData,
    _fd: Option<&mut FileVariant>,
) -> TeeResult {
    let mut digest = [0u8; TEE_FS_HTREE_HASH_SIZE];

    if node.parent.is_some() {
        calc_node_hash_with_type(TEE_FS_HTREE_HASH_ALG, node, None, &mut digest)?;
    } else {
        calc_node_hash_with_type(
            TEE_FS_HTREE_HASH_ALG,
            node,
            Some(&ht_data.imeta.meta),
            &mut digest,
        )?;
    }

    info!("hash with {} {}", node.id, hex::encode(digest));
    Ok(())
}

/// sync the node to the storage
///
/// # Arguments
/// * `node` - the node
/// * `ht_data` - the data of the tree
/// * `fd` - the file descriptor
/// # Returns
/// * `TeeResult` - the result of the operation
fn htree_sync_node_to_storage(
    mut node: &mut HtreeNode,
    ht_data: &TeeFsHtreeData,
    fd: Option<&mut FileVariant>,
) -> TeeResult {
    #[allow(unused_assignments)]
    let mut vers: u8 = 0;
    let mut meta: Option<&TeeFsHtreeMeta> = None;
    // The node can be dirty while the block isn't updated due to
    // updated children, but if block is updated the node has to be
    // dirty.
    assert!(node.dirty >= node.block_updated);

    if !node.dirty {
        return Ok(());
    }
    if fd.is_none() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    if let Some(parent_ptr) = node.parent {
        // parent 是 NonNull<HtreeNode>，可以直接解引用访问父节点
        // 安全性：parent 指针的生命周期由 tee_fs_htree 保证，在节点存在期间始终有效
        let parent_node = unsafe { &mut *parent_ptr.as_ptr() };

        // 计算 flags 并设置
        let f = htree_node_committed_child(node.id & 1);

        parent_node.dirty = true;
        parent_node.node.flags ^= f as u16;
        vers = ((parent_node.node.flags & f as u16) != 0) as u8;
    } else {
        // Counter isn't updated yet, it's increased just before
        // writing the header.
        vers = ((ht_data.head.counter & 1) == 0) as u8;
        meta = Some(&ht_data.imeta.meta);
    }
    let mut digest = [0u8; TEE_FS_HTREE_HASH_SIZE];

    calc_node_hash_with_type(TEE_FS_HTREE_HASH_ALG, &node, meta, &mut digest)?;

    node.node.hash.copy_from_slice(&digest);

    node.dirty = false;
    node.block_updated = false;

    rpc_write_node(fd.unwrap(), node.id, vers, &mut node.node)?;
    Ok(())
}

/// create cipher for encrypt or decrypt
///
/// # Arguments
/// * `alg` - the algorithm of the cipher
/// * `key_bytes` - the length of the key
/// # Returns
/// * `TeeResult<Cipher<M, Authenticated, Fresh>>` - the cipher for encrypt or decrypt
fn create_cipher<M: Operation>(
    alg: TEE_ALG,
    key_bytes: usize,
) -> TeeResult<Cipher<M, Authenticated, Fresh>> {
    let key_bits = key_bytes * 8;
    match alg {
        TEE_ALG_AES_GCM => Cipher::<M, Authenticated, Fresh>::new(
            raw::CipherId::Aes,
            raw::CipherMode::GCM,
            key_bits as u32,
        )
        .map_err(|_| TEE_ERROR_NOT_SUPPORTED),
        _ => return Err(TEE_ERROR_NOT_SUPPORTED),
    }
}

/// init cipher for encrypt or decrypt, internal function,
/// using separated parameters to avoid borrow conflicts
///
/// # Arguments
/// * `fek` - the key for encrypt or decrypt
/// * `head` - the head of the tree
/// * `iv` - the iv for encrypt or decrypt
/// * `ni_is_some` - if the node is some
/// * `root_hash` - the hash of the root
/// # Returns
/// * `Cipher<M, Authenticated, CipherData>` - the cipher for encrypt or decrypt
fn authenc_init_core<M: Operation>(
    fek: &[u8; TEE_FS_HTREE_FEK_SIZE],
    head: &TeeFsHtreeImage,
    iv: &[u8; TEE_FS_HTREE_IV_SIZE],
    ni_is_some: bool,
    root_hash: Option<&[u8; TEE_FS_HTREE_HASH_SIZE]>,
) -> TeeResult<Cipher<M, Authenticated, CipherData>> {
    const ALG: TEE_ALG = TEE_FS_HTREE_AUTH_ENC_ALG;
    let mut aad_len = TEE_FS_HTREE_FEK_SIZE + TEE_FS_HTREE_IV_SIZE;

    if !ni_is_some {
        aad_len += TEE_FS_HTREE_HASH_SIZE + core::mem::size_of_val(&head.counter);
    }

    let cipher = create_cipher::<M>(ALG, TEE_FS_HTREE_FEK_SIZE)?;
    let cipher_k = cipher.set_key_iv(fek, iv).map_err(|_| TEE_ERROR_GENERIC)?;

    let mut ad: Vec<u8> = Vec::with_capacity(aad_len);
    if ni_is_some {
        if let Some(hash) = root_hash {
            ad.extend_from_slice(hash);
        }
        ad.extend_from_slice(bytemuck::bytes_of(&head.counter));
    }

    ad.extend_from_slice(bytemuck::bytes_of(&head.enc_fek));
    ad.extend_from_slice(iv);

    let cipher_d = cipher_k.set_ad(ad.as_slice());

    cipher_d.map_err(|_| TEE_ERROR_GENERIC)
}

/// init cipher for encrypt or decrypt
///
/// # Arguments
/// * `mode` - the mode of the operation
/// * `ht` - the tree
/// * `ni` - the node
/// * `_payload_len` - the length of the payload
/// * `root_hash` - the hash of the root
/// # Returns
/// * `Cipher<M, Authenticated, CipherData>` - the cipher for encrypt or decrypt
pub fn authenc_init<M: Operation>(
    mode: TEE_OperationMode,
    ht: &mut TeeFsHtree,
    ni: Option<&mut TeeFsHtreeNodeImage>,
    _payload_len: usize,
    root_hash: Option<&[u8; TEE_FS_HTREE_HASH_SIZE]>,
) -> TeeResult<Cipher<M, Authenticated, CipherData>> {
    // if ni is some and root_hash is none, use ht.root.node.hash
    let hash = if ni.is_some() && root_hash.is_none() {
        Some(&ht.root.node.hash)
    } else {
        root_hash
    };

    let (iv, ni_is_some) = if let Some(ni) = ni {
        if mode == TEE_OperationMode::TEE_MODE_ENCRYPT {
            crypto_rng_read(&mut ni.iv)?;
        }
        (&ni.iv, true)
    } else {
        if mode == TEE_OperationMode::TEE_MODE_ENCRYPT {
            crypto_rng_read(&mut ht.data.head.iv)?;
        }
        (&ht.data.head.iv, false)
    };

    authenc_init_core(&ht.data.fek, &ht.data.head, iv, ni_is_some, hash)
}

/// special version for decrypt, using separated parameters to avoid borrow conflicts
///
/// # Arguments
/// * `fek` - the key for decrypt
/// * `head` - the head of the tree
/// * `ni_iv` - the iv from the node, if None use head.iv
/// * `root_hash` - the hash of the root
/// # Returns
/// * `Cipher<Decryption, Authenticated, CipherData>` - the cipher for decrypt
fn authenc_init_decrypt(
    fek: &[u8; TEE_FS_HTREE_FEK_SIZE],
    head: &TeeFsHtreeImage,
    ni_iv: Option<&[u8; TEE_FS_HTREE_IV_SIZE]>,
    root_hash: Option<&[u8; TEE_FS_HTREE_HASH_SIZE]>,
) -> TeeResult<Cipher<Decryption, Authenticated, CipherData>> {
    let (iv, ni_is_some) = if let Some(ni_iv) = ni_iv {
        (ni_iv, true)
    } else {
        (&head.iv, false)
    };

    authenc_init_core(fek, head, iv, ni_is_some, root_hash)
}

/// special version for encrypt, using separated parameters to avoid borrow conflicts
///
/// # Arguments
/// * `fek` - the key for encrypt
/// * `head` - the head of the tree (only needs to be mutable if ni_iv is None)
/// * `ni_iv` - the iv from the node (will be filled with random data), if None use head.iv
/// * `root_hash` - the hash of the root
/// # Returns
/// * `Cipher<Encryption, Authenticated, CipherData>` - the cipher for encrypt
fn authenc_init_encrypt(
    fek: &[u8; TEE_FS_HTREE_FEK_SIZE],
    head: &TeeFsHtreeImage,
    ni_iv: Option<&mut [u8; TEE_FS_HTREE_IV_SIZE]>,
    root_hash: Option<&[u8; TEE_FS_HTREE_HASH_SIZE]>,
) -> TeeResult<Cipher<Encryption, Authenticated, CipherData>> {
    let (iv, ni_is_some) = if let Some(ni_iv) = ni_iv {
        crypto_rng_read(ni_iv)?;
        (ni_iv as &[u8; TEE_FS_HTREE_IV_SIZE], true)
    } else {
        // This case should not happen in tee_fs_htree_write_block
        // as we always pass Some(&mut node.node.iv)
        // But we keep it for completeness
        return Err(TEE_ERROR_GENERIC);
    };

    authenc_init_core(fek, head, iv, ni_is_some, root_hash)
}

/// final for decrypt, using separated parameters to avoid borrow conflicts
///
/// # Arguments
/// * `cipher` - the cipher for decrypt
/// * `tag` - the tag for decrypt
/// * `crypt` - the crypt for decrypt
/// * `plain` - the plain for decrypt
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn authenc_decrypt_final(
    cipher: Cipher<Decryption, Authenticated, CipherData>,
    tag: &[u8],
    crypt: &[u8],
    plain: &mut [u8],
) -> TeeResult {
    let mut plain_with_add_block = vec![0u8; crypt.len() + cipher.block_size()];

    let (len1, cipher_d) = cipher
        .update(crypt, plain_with_add_block.as_mut_slice())
        .map_err(|_| TEE_ERROR_GENERIC)?;

    // plain[len1..] 是 finish 写入的位置
    let (len2, cipher_t) = cipher_d
        .finish(&mut plain_with_add_block.as_mut_slice()[len1..])
        .map_err(|_| TEE_ERROR_GENERIC)?;

    cipher_t.check_tag(tag).map_err(|_| TEE_ERROR_MAC_INVALID)?;

    if len1 + len2 != crypt.len() {
        return Err(TEE_ERROR_GENERIC);
    }

    plain.copy_from_slice(&plain_with_add_block.as_slice()[..crypt.len()]);
    Ok(())
}

/// final for encrypt, using separated parameters to avoid borrow conflicts
///
/// # Arguments
/// * `cipher` - the cipher for encrypt
/// * `tag` - the tag for encrypt
/// * `plain` - the plain for encrypt
/// * `crypt` - the crypt for encrypt
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn authenc_encrypt_final(
    cipher: Cipher<Encryption, Authenticated, CipherData>,
    tag: &mut [u8],
    plain: &[u8],
    crypt: &mut [u8],
) -> TeeResult {
    let mut crypt_with_add_block = vec![0u8; plain.len() + cipher.block_size()];

    let (len1, cipher_d) = cipher
        .update(plain, crypt_with_add_block.as_mut_slice())
        .map_err(|_| TEE_ERROR_GENERIC)?;

    // crypt[len1..] 是 finish 写入的位置
    let (len2, cipher_t) = cipher_d
        .finish(&mut crypt_with_add_block.as_mut_slice()[len1..])
        .map_err(|_| TEE_ERROR_GENERIC)?;

    cipher_t.write_tag(tag).map_err(|_| TEE_ERROR_GENERIC)?;

    if len1 + len2 != plain.len() {
        return Err(TEE_ERROR_GENERIC);
    }

    crypt.copy_from_slice(&crypt_with_add_block.as_slice()[..plain.len()]);

    Ok(())
}

/// update the root of the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn update_root(ht: &mut TeeFsHtree) -> TeeResult {
    ht.data.head.counter += 1;

    let cipher = authenc_init(
        TEE_OperationMode::TEE_MODE_ENCRYPT,
        ht,
        None,
        size_of_val(&ht.data.imeta),
        None,
    )?;

    let ptr = &mut ht.data.imeta as *mut _ as *mut u8;
    unsafe {
        let slice = core::slice::from_raw_parts_mut(ptr, size_of_val(&mut ht.data.imeta));
        authenc_encrypt_final(
            cipher,
            &mut ht.data.head.tag,
            slice,
            &mut ht.data.head.imeta,
        )?;
    }

    Ok(())
}

/// traverse the tree post order
///
/// # Arguments
/// * `ht` - the tree
/// * `visitor` - the visitor function
/// * `fd` - the file descriptor
/// # Returns
/// * `TeeResult` - the result of the operation
fn htree_traverse_post_order<F>(
    ht: &TeeFsHtree,
    visitor: &mut F,
    fd: Option<&mut FileVariant>,
) -> TeeResult
where
    F: FnMut(&HtreeNode, &TeeFsHtreeData, Option<&mut FileVariant>) -> TeeResult,
{
    post_order_traverse(&ht.root, &ht.data, visitor, fd)?;

    Ok(())
}

/// traverse the tree post order mut
///
/// # Arguments
/// * `ht` - the tree
/// * `visitor` - the visitor function
/// * `fd` - the file descriptor
/// # Returns
/// * `TeeResult` - the result of the operation
fn htree_traverse_post_order_mut<F>(
    ht: &mut TeeFsHtree,
    visitor: &mut F,
    fd: Option<&mut FileVariant>,
) -> TeeResult
where
    F: FnMut(&mut HtreeNode, &TeeFsHtreeData, Option<&mut FileVariant>) -> TeeResult,
{
    post_order_traverse_mut(&mut ht.root, &ht.data, visitor, fd)?;

    Ok(())
}

/// verify the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn verify_tree(ht: &TeeFsHtree) -> TeeResult {
    htree_traverse_post_order(ht, &mut verify_node, None)?;
    Ok(())
}

/// calc the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn calc_tree(ht: &mut TeeFsHtree) -> TeeResult {
    htree_traverse_post_order_mut(ht, &mut calc_node, None)?;
    Ok(())
}

/// print the hash of the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn print_tree_hash(ht: &TeeFsHtree) -> TeeResult {
    htree_traverse_post_order(ht, &mut print_node_hash, None)?;

    Ok(())
}

/// init the root node of the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn init_root_node(ht: &mut TeeFsHtree) -> TeeResult {
    let _hash = crypto_hash_alloc_ctx(TEE_ALG_SHA256)?;

    ht.root.id = 1;
    ht.root.dirty = true;

    // TODO: 需要优化，以去掉搬运过程
    let mut digest = [0u8; TEE_FS_HTREE_HASH_SIZE];
    calc_node_hash(&ht.root, &ht.data.imeta.meta, &mut digest)?;
    ht.root.node.hash.copy_from_slice(&digest);

    Ok(())
}

/// convert the node id to the level
///
/// # Arguments
/// * `node_id` - the node id
/// # Returns
/// * `usize` - the level of the node
pub fn node_id_to_level(node_id: usize) -> usize {
    assert!(node_id > 0 && node_id < usize::MAX);
    (usize::BITS - node_id.leading_zeros()) as usize
}

/// find the closest node of the tree
///
/// # Arguments
/// * `ht` - the tree
/// * `node_id` - the node id
/// # Returns
/// * `&mut HtreeNode` - the closest node
pub fn find_closest_node(ht: &mut TeeFsHtree, node_id: usize) -> &mut HtreeNode {
    let target_level = node_id_to_level(node_id);

    // 记录访问路径（索引序列），避免在循环中的借用冲突
    let mut path = Vec::new();
    for n in 1..target_level {
        let bit_idx = target_level - n - 1;
        path.push((node_id >> bit_idx) & 1);
    }

    // 通过路径逐步访问节点，每次只借用一次
    let mut current = &mut ht.root;
    for &index in &path {
        // 检查子节点是否存在
        let child_exists = {
            let child_opt = current.get_child_by_index(index);
            child_opt.is_some()
        };

        if child_exists {
            // 重新获取子节点引用，因为之前的引用已经释放
            current = current.get_child_by_index(index).unwrap();
        } else {
            // 子节点不存在，返回当前节点
            return current;
        }
    }

    current
}

/// find the node of the tree
///
/// # Arguments
/// * `ht` - the tree
/// * `node_id` - the node id
/// # Returns
/// * `Option<&mut HtreeNode>` - the node
pub fn find_node(ht: &mut TeeFsHtree, node_id: usize) -> Option<&mut HtreeNode> {
    let node = find_closest_node(ht, node_id);
    if node.id == node_id { Some(node) } else { None }
}

/// ensure the node exists, if not create it
/// internal function, not return reference, to avoid borrow conflicts
///
/// # Arguments
/// * `ht` - the tree
/// * `create` - if create the node
/// * `node_id` - the node id
/// # Returns
/// * `TeeResult` - the result of the operation
fn ensure_node_exists(ht: &mut TeeFsHtree, create: bool, node_id: usize) -> TeeResult {
    let current_node = find_closest_node(ht, node_id);
    let current_id = current_node.id;

    if current_id == node_id {
        return Ok(()); // node exists
    }

    if !create {
        return Err(TEE_ERROR_GENERIC);
    }

    // Add missing nodes, some nodes may already be there.
    for n in current_id..=node_id {
        let node = find_closest_node(ht, n);
        if node.id == n {
            continue;
        }
        // Node id n should be a child of node
        debug_assert_eq!((n >> 1), node.id);
        debug_assert!(node.get_child_by_index(n & 1).is_none());

        let new_node = HtreeNode::new(n, TeeFsHtreeNodeImage::default());

        if (n & 1) == 0 {
            HtreeNode::set_left(node, new_node);
        } else {
            HtreeNode::set_right(node, new_node);
        }
    }

    // update max_node_id
    if node_id > ht.data.imeta.max_node_id as usize {
        ht.data.imeta.max_node_id = node_id as u32;
    }

    Ok(())
}

/// get the node of the tree
///
/// # Arguments
/// * `ht` - the tree
/// * `create` - if create the node
/// * `node_id` - the node id
/// # Returns
/// * `TeeResult<&mut HtreeNode>` - the node
pub fn get_node(ht: &mut TeeFsHtree, create: bool, node_id: usize) -> TeeResult<&mut HtreeNode> {
    // first ensure the node exists (create the required nodes)
    ensure_node_exists(ht, create, node_id)?;

    // then find and return the node
    Ok(find_closest_node(ht, node_id))
}

/// get the index from the counter
///
/// # Arguments
/// * `counter0` - the counter0
/// * `counter1` - the counter1
/// # Returns
/// * `Result<u8, ()>` - the index
fn get_idx_from_counter(counter0: u32, counter1: u32) -> Result<u8, ()> {
    if (counter0 & 1) == 0 {
        // Equivalent to !(counter0 & 1)
        if (counter1 & 1) == 0 {
            // Equivalent to !(counter1 & 1)
            return Ok(0);
        }
        if counter0 > counter1 {
            return Ok(0);
        } else {
            return Ok(1);
        }
    }

    if (counter1 & 1) != 0 {
        // Equivalent to (counter1 & 1)
        Ok(1)
    } else {
        Err(())
    }
}

/// init the head from the data
///
/// # Arguments
/// * `fd` - the file descriptor
/// * `ht` - the tree
/// * `hash` - the hash of the target node
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn init_head_from_data(
    fd: &mut FileVariant,
    ht: &mut TeeFsHtree,
    hash: Option<&[u8]>,
) -> TeeResult {
    if let Some(target_hash) = hash {
        for idx in 0.. {
            let node_ref = &mut ht.root.node; // mutable access in scope
            rpc_read_node(fd, 1, idx, node_ref)?;
            if node_ref.hash == target_hash {
                let _head = rpc_read_head(fd, idx, &mut ht.data.head)?;
                break;
            }

            if idx != 0 {
                return Err(TEE_ERROR_SECURITY);
            }
        }
    } else {
        let mut heads = [TeeFsHtreeImage::default(); 2];
        for idx in 0..2 {
            rpc_read_head(fd, 0, &mut heads[idx])?;
        }

        let idx = get_idx_from_counter(heads[0].counter, heads[1].counter)
            .map_err(|_| TEE_ERROR_SECURITY)?;

        let node_ref = &mut ht.root.node;
        rpc_read_node(fd, 1, idx, node_ref)?;

        ht.data.head = heads[idx as usize];
    }

    ht.root.id = 1;
    Ok(())
}

/// init the tree from the data
///
/// # Arguments
/// * `fd` - the file descriptor
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn init_tree_from_data(fd: &mut FileVariant, ht: &mut TeeFsHtree) -> TeeResult {
    let mut node_image = TeeFsHtreeNodeImage::default();
    let mut node_id = 2;

    while node_id <= ht.data.imeta.max_node_id {
        // find the parent node (node_id >> 1)
        let parent_id = node_id >> 1;
        let parent_node = find_node(ht, parent_id as usize).ok_or(TEE_ERROR_GENERIC)?; // htree not find parent node, return error

        let committed_version = (parent_node.node.flags
            & htree_node_committed_child((node_id & 1) as usize) as u16
            != 0) as u8;

        // read the node from the storage
        rpc_read_node(fd, node_id as usize, committed_version, &mut node_image)?;

        // create node or get the existing node reference
        let nc = get_node(ht, true, node_id as usize)?;

        // set the content
        nc.node = node_image;

        node_id += 1;
    }

    Ok(())
}

/// verify the root of the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn verify_root(ht: &mut TeeFsHtree) -> TeeResult {
    let mut fek = [0u8; TEE_FS_HTREE_FEK_SIZE];
    tee_fs_fek_crypt(
        Some(&ht.data.uuid),
        TEE_OperationMode::TEE_MODE_DECRYPT,
        Some(&ht.data.head.enc_fek),
        TEE_FS_KM_FEK_SIZE,
        Some(&mut fek),
    )?;
    ht.data.fek.copy_from_slice(&fek);

    let cipher = authenc_init(
        TEE_OperationMode::TEE_MODE_DECRYPT,
        ht,
        None,
        size_of_val(&ht.data.imeta),
        None,
    )?;

    let ptr = &mut ht.data.imeta as *mut _ as *mut u8;
    unsafe {
        let slice = core::slice::from_raw_parts_mut(ptr, size_of_val(&mut ht.data.imeta));
        authenc_decrypt_final(cipher, &ht.data.head.tag, &ht.data.head.imeta, slice)?;
    }

    Ok(())
}

/// sync the tree to the storage
///
/// # Arguments
/// * `ht` - the tree
/// * `fd` - the file descriptor
/// * `hash` - the hash of the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn tee_fs_htree_sync_to_storage(
    ht: &mut TeeFsHtree,
    fd: &mut FileVariant,
    mut hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
) -> TeeResult {
    // if ht.is_none() {
    //     return Err(TeeResultCode::ErrorCorruptObject);
    // }

    if !ht.data.dirty {
        return Ok(());
    }

    // TODO: fd through out parameters?
    // let mut fd = open_file_like("filenamne", FS_OFLAG_DEFAULT, FS_MODE_644)
    //     .map_err(|_| TeeResultCode::ErrorGeneric)?;

    htree_traverse_post_order_mut(ht, &mut htree_sync_node_to_storage, Some(fd))?;

    update_root(ht)?;

    rpc_write_head(
        fd,
        // ht,
        (ht.data.head.counter & 1) as u8,
        &mut ht.data.head,
    )?;

    ht.data.dirty = false;

    if let Some(slice) = hash.as_deref_mut() {
        slice.copy_from_slice(&ht.root.node.hash);
    }

    // TODO:
    // tee_fs_htree_close(ht_arg);
    Ok(())
}

/// open the tree
///
/// # Arguments
/// * `fd` - the file descriptor
/// * `create` - if create the tree
/// * `hash` - the hash of the tree
/// * `uuid` - the uuid of the tree
/// # Returns
/// * `TeeResult<Box<TeeFsHtree>>` - the tree
pub fn tee_fs_htree_open(
    fd: &mut FileVariant,
    create: bool,
    hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
    uuid: Option<&TEE_UUID>,
) -> TeeResult<Box<TeeFsHtree>> {
    let mut ht = Box::new(TeeFsHtree::default());
    if let Some(uuid_val) = uuid {
        ht.data.uuid = *uuid_val;
    }

    let init_result = (|| {
        if create {
            let mut dummy_head = TeeFsHtreeImage::default();
            crypto_rng_read(&mut ht.data.fek).map_err(|e| e)?;
            tee_fs_fek_crypt(
                Some(&ht.data.uuid),
                TEE_OperationMode::TEE_MODE_ENCRYPT,
                Some(&ht.data.fek),
                size_of_val(&ht.data.fek),
                Some(&mut ht.data.head.enc_fek),
            )?;
            init_root_node(&mut ht)?;
            ht.data.dirty = true;
            tee_fs_htree_sync_to_storage(&mut ht, fd, hash)?;
            rpc_write_head(fd, /* &mut ht, */ 0, &mut dummy_head)?;
        } else {
            init_head_from_data(fd, &mut ht, hash.as_ref().map(|s| &s[..]))?;
            verify_root(&mut ht)?;
            init_tree_from_data(fd, &mut ht)?;
            verify_tree(&ht)?;
        }

        Ok(())
    })();
    match init_result {
        Ok(_) => {
            // if init success, return ht ownership
            Ok(ht)
        }
        Err(e) => {
            // if init failed, call tee_fs_htree_close to clean ht
            if let Err(close_err) = tee_fs_htree_close(ht) {
                error!("tee_fs_htree_close error! {:?}", close_err);
            }
            Err(e)
        }
    }
}

/// close the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn tee_fs_htree_close(_ht: Box<TeeFsHtree>) -> TeeResult {
    // TODO: check if no need to free nodes manually??? rust will free them automatically???
    // htree_traverse_post_order(&ht, &mut free_node, None)?;

    Ok(())
    // ht free after leave scope
}

/// get the meta of the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `&mut TeeFsHtreeMeta` - the meta of the tree
pub fn tee_fs_htree_get_meta(ht: &mut TeeFsHtree) -> &mut TeeFsHtreeMeta {
    &mut ht.data.imeta.meta
}

/// set the dirty of the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn tee_fs_htree_meta_set_dirty(ht: &mut TeeFsHtree) {
    ht.data.dirty = true;
    ht.root.dirty = true;
}

/// get the block node of the tree
///
/// # Arguments
/// * `ht` - the tree
/// * `create` - if create the block node
/// * `block_num` - the block number
/// # Returns
/// * `TeeResult<&mut HtreeNode>` - the block node
fn get_block_node(
    ht: &mut TeeFsHtree,
    create: bool,
    block_num: usize,
) -> TeeResult<&mut HtreeNode> {
    let node_id = block_num_to_node_id(block_num);
    get_node(ht, create, node_id)
}

/// read the block of the tree
///
/// # Arguments
/// * `ht` - the tree
/// * `storage` - the storage
/// * `fd` - the file descriptor
/// * `block_num` - the block number
/// * `block` - the block
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn tee_fs_htree_read_block<S: TeeFsHtreeStorageOps>(
    ht: &mut TeeFsHtree,
    storage: &S,
    fd: &mut FileVariant,
    block_num: usize,
    block: &mut [u8; BLOCK_SIZE],
) -> TeeResult {
    // first get the node and extract the necessary information, then release the node borrow
    let (block_vers, ni_iv, ni_tag) = {
        let node = get_block_node(ht, false, block_num).map_err(|_| TEE_ERROR_CORRUPT_OBJECT)?;

        let vers = if (node.node.flags & HTREE_NODE_COMMITTED_BLOCK as u16) != 0 {
            1
        } else {
            0
        };

        // extract iv and tag (these are Copy types, can be used directly)
        (vers, node.node.iv, node.node.tag)
    };

    // before calling authenc_init, get the root hash first
    let root_hash = ht.root.node.hash;

    // now the node borrow is released, can safely borrow the other parts of ht
    let result = (|| {
        // allocate buffer, length is one BLOCK
        let mut enc_block = vec![0u8; storage.block_size()];

        storage.rpc_read_init()?;

        let len = storage.rpc_read_final(
            fd,
            TeeFsHtreeType::Block,
            block_num,
            block_vers,
            &mut enc_block,
        )?;

        if len != storage.block_size() {
            return Err(TEE_ERROR_CORRUPT_OBJECT);
        }

        // use authenc_init_decrypt, directly pass ni_iv without constructing temporary struct
        let cipher =
            authenc_init_decrypt(&ht.data.fek, &ht.data.head, Some(&ni_iv), Some(&root_hash))
                .map_err(|_| TEE_ERROR_CORRUPT_OBJECT)?;

        // same as C version: res = authenc_decrypt_final(ctx, node->node.tag, enc_block, ht->stor->block_size, block);
        authenc_decrypt_final(cipher, &ni_tag, &enc_block, block)?;

        Ok(())
    })();

    if result.is_err() {
        error!("tee_fs_htree_read_block error! {:?}", result);
        // tee_fs_htree_close(ht)?;
    }

    result
}

/// write the block of the tree
///
/// # Arguments
/// * `ht` - the tree
/// * `storage` - the storage
/// * `fd` - the file descriptor
/// * `block_num` - the block number
/// * `block` - the block
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn tee_fs_htree_write_block<S: TeeFsHtreeStorageOps>(
    ht: &mut TeeFsHtree,
    storage: &S,
    fd: &mut FileVariant,
    block_num: usize,
    block: &[u8; BLOCK_SIZE],
) -> TeeResult {
    // before calling authenc_init, get the root hash first
    let root_hash = ht.root.node.hash;

    // extract fek and head before getting node to avoid borrow conflicts
    let fek = ht.data.fek;
    let head = ht.data.head;

    let result = (|| {
        // get node once for all operations
        let node = get_block_node(ht, true, block_num).map_err(|_| TEE_ERROR_CORRUPT_OBJECT)?;

        // if block not updated, toggle committed flag
        let block_vers = {
            if !node.block_updated {
                node.node.flags ^= HTREE_NODE_COMMITTED_BLOCK as u16;
            }

            if (node.node.flags & HTREE_NODE_COMMITTED_BLOCK as u16) != 0 {
                1
            } else {
                0
            }
        };

        // allocate encryption buffer
        let mut enc_block = vec![0u8; storage.block_size()];

        // initialize write operation
        storage.rpc_write_init()?;

        // use authenc_init_encrypt, directly use extracted fek and head (immutable) since ni_iv is Some
        // authenc_init_encrypt will generate random IV for node.node.iv
        let cipher = authenc_init_encrypt(&fek, &head, Some(&mut node.node.iv), Some(&root_hash))
            .map_err(|_| TEE_ERROR_CORRUPT_OBJECT)?;

        // encrypt data block
        authenc_encrypt_final(cipher, &mut node.node.tag, block, &mut enc_block)?;

        // mark node as updated and dirty
        node.block_updated = true;
        node.dirty = true;

        // node borrow will be released when going out of scope
        // write encrypted data
        storage.rpc_write_final(fd, TeeFsHtreeType::Block, block_num, block_vers, &enc_block)?;

        // mark tree as dirty
        ht.data.dirty = true;

        Ok(())
    })();

    if result.is_err() {
        error!("tee_fs_htree_write_block error! {:?}", result);
        // tee_fs_htree_close(ht)?;
    }

    result
}

/// truncate the tree
///
/// # Arguments
/// * `ht` - the tree
/// * `block_num` - the block number
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn tee_fs_htree_truncate(ht: &mut TeeFsHtree, block_num: usize) -> TeeResult {
    let node_id = block_num_to_node_id(block_num);

    while node_id < ht.data.imeta.max_node_id as usize {
        let current_max_node_id = ht.data.imeta.max_node_id as usize;
        let node = find_closest_node(ht, current_max_node_id);
        assert!(node.id == current_max_node_id);
        assert!(node.get_child_by_index(0).is_none() && node.get_child_by_index(1).is_none());
        assert!(node.parent.is_some());

        // Get the parent node pointer and child node index, then release the node reference
        let (parent_ptr, child_index) = if let Some(parent) = node.parent {
            (parent, node.id & 1)
        } else {
            unreachable!() // already ensured by assert that parent exists
        };

        // node reference will be released automatically when scope ends, here explicitly mark it as not used
        let _ = node;

        // Use unsafe to get the mutable reference of the parent node from NonNull
        // Safety: parent pointer lifetime is guaranteed by tee_fs_htree, valid during node existence
        let parent_node = unsafe { &mut *parent_ptr.as_ptr() };

        // Set the corresponding child tree of the parent node to None
        if child_index == 0 {
            parent_node.left = None;
        } else {
            parent_node.right = None;
        }
        ht.data.imeta.max_node_id -= 1;
        ht.data.dirty = true;
    }
    Ok(())
}
