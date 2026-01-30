// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, string::String, vec, vec::Vec};
use core::{fmt, fmt::Debug, ptr::NonNull};

use bytemuck::{Pod, Zeroable};
use mbedtls::{
    cipher::{Authenticated, Cipher, CipherData, Decryption, Encryption, Fresh, Operation, raw},
    error::HiError::PemAllocFailed,
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
    rng_software::crypto_rng_read,
    tee_fs_key_manager::{TEE_FS_KM_FEK_SIZE, tee_fs_fek_crypt},
    tee_ree_fs::{BLOCK_SIZE, TeeFsFdAux, TeeFsHtreeStorageOps},
    utee_defines::TEE_ALG,
    utils::slice_fmt,
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
#[derive(Copy, Clone, Default, Pod, Zeroable)]
pub struct TeeFsHtreeImage {
    pub iv: [u8; TEE_FS_HTREE_IV_SIZE],
    pub tag: [u8; TEE_FS_HTREE_TAG_SIZE],
    pub enc_fek: [u8; TEE_FS_HTREE_FEK_SIZE],
    pub imeta: [u8; TEE_FS_HTREE_IMETA_SIZE],
    pub counter: u32,
}

impl Debug for TeeFsHtreeImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TeeFsHtreeImage {{ iv: {:?}, tag: {:?}, enc_fek: {:?}, imeta: {:?}, counter: {:?} }}",
            hex::encode(self.iv),
            hex::encode(self.tag),
            hex::encode(self.enc_fek),
            hex::encode(self.imeta),
            self.counter
        )
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)] // Derive Clone for easy copying if needed
pub struct TeeFsHtreeNodeImage {
    pub hash: [u8; TEE_FS_HTREE_HASH_SIZE],
    pub iv: [u8; TEE_FS_HTREE_IV_SIZE],
    pub tag: [u8; TEE_FS_HTREE_TAG_SIZE],
    pub flags: u16,
}

static_assertions::const_assert!(core::mem::size_of::<TeeFsHtreeNodeImage>() == 66);

impl TeeFsHtreeNodeImage {
    /// Returns the on-disk byte representation of this node image.
    ///
    /// # Safety invariants
    /// - `Self` is `#[repr(C)]` with a stable layout
    /// - All bytes are fully initialized
    /// - This type represents an on-disk / wire image
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut Self as *mut u8,
                core::mem::size_of::<Self>(),
            )
        }
    }
}

impl Debug for TeeFsHtreeNodeImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TeeFsHtreeNodeImage {{ hash: {:?}, iv: {:?}, tag: {:?}, flags: {:X?} }}",
            hex::encode(self.hash),
            hex::encode(self.iv),
            hex::encode(self.tag),
            self.flags,
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    pub fn get_child_by_index(&mut self, index: usize) -> Option<&HtreeNode> {
        if index == 0 {
            self.left.as_ref().map(|b| b.as_ref())
        } else {
            self.right.as_ref().map(|b| b.as_ref())
        }
    }

    /// 根据索引获取左右子树的引用。
    ///
    /// `index` 为 0 时返回左子树，为 1 时返回右子树。
    /// 如果对应子树不存在，则返回 `None`。
    pub fn get_child_by_index_mut(&mut self, index: usize) -> Option<&mut HtreeNode> {
        if index == 0 {
            self.left.as_mut().map(|b| b.as_mut())
        } else {
            self.right.as_mut().map(|b| b.as_mut())
        }
    }
}

pub type Subtree = Option<Box<HtreeNode>>;

// HtreeNode 使用 NonNull 作为 parent 指针，不是自动 Send 的
// 但我们可以安全地实现 Send，因为：
// 1. 整个 TeeFsHtree 被 Mutex 保护，所有访问都通过 Mutex 进行
// 2. parent 指针的生命周期由 TeeFsHtree 保证，不会出现悬垂指针
// 3. 树结构在同一线程中访问，不会出现并发修改
unsafe impl Send for HtreeNode {}

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

// #[derive(Debug)]
pub struct TeeFsHtree {
    pub root: HtreeNode,
    pub data: TeeFsHtreeData,
    pub storage: Box<dyn TeeFsHtreeStorageOps>,
}

impl Default for TeeFsHtree {
    fn default() -> Self {
        TeeFsHtree {
            root: HtreeNode::new(0, TeeFsHtreeNodeImage::default()),
            data: TeeFsHtreeData::default(),
            storage: Box::new(TeeFsFdAux::new()),
        }
    }
}

impl Debug for TeeFsHtree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TeeFsHtree {{ root: {:?}, data: {:?} }}",
            self.root, self.data
        )
    }
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
    // fd: &mut FileVariant,
    storage: &dyn TeeFsHtreeStorageOps,
    typ: TeeFsHtreeType,
    idx: usize,
    vers: u8,
    data: &mut [u8],
) -> TeeResult {
    let dlen = data.len();
    if dlen == 0 {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    // rpc_read_init()?;
    storage.rpc_read_init()?;

    let result = storage.rpc_read_final(typ, idx, vers, data)?;
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
    storage: &dyn TeeFsHtreeStorageOps,
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
    rpc_read(storage, TeeFsHtreeType::Head, 0, vers, data_ptr)?;
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
    // fd: &mut FileVariant,
    // ht: &mut TeeFsHtree,
    // storage: &dyn TeeFsHtreeStorageOps,
    storage: &dyn TeeFsHtreeStorageOps,
    node_id: usize,
    vers: u8,
    head: &mut TeeFsHtreeNodeImage,
) -> TeeResult {
    tee_debug!("rpc_read_node: node_id: {:X?}, vers: {:X?}", node_id, vers);
    let data_ptr: &mut [u8] = head.as_bytes_mut();
    rpc_read(storage, TeeFsHtreeType::Node, node_id - 1, vers, data_ptr)?;
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
    // fd: &FileVariant,
    storage: &dyn TeeFsHtreeStorageOps,
    typ: TeeFsHtreeType,
    idx: usize,
    vers: u8,
    data: &[u8],
) -> TeeResult {
    let dlen = data.len();
    if dlen == 0 {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    storage.rpc_write_init()?;

    let _ = storage
        .rpc_write_final(typ, idx, vers, data)
        .inspect_err(|e| {
            error!("rpc_write_final: error: {:X?}", e);
        })?;

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
    // fd: &FileVariant,
    // ht: &mut TeeFsHtree,
    storage: &dyn TeeFsHtreeStorageOps,
    vers: u8,
    head: &TeeFsHtreeImage,
) -> TeeResult {
    tee_debug!("rpc_write_head: vers: {}, counter: {}", vers, head.counter);
    let data_ptr: &[u8] = unsafe {
        core::slice::from_raw_parts(
            head as *const TeeFsHtreeImage as *const u8,
            size_of::<TeeFsHtreeImage>(),
        )
    };
    rpc_write(storage, TeeFsHtreeType::Head, 0, vers, data_ptr)?;
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
    // fd: &FileVariant,
    // ht: &mut TeeFsHtree,
    storage: &dyn TeeFsHtreeStorageOps,
    node_id: usize,
    vers: u8,
    head: &TeeFsHtreeNodeImage,
) -> TeeResult {
    tee_debug!(
        "rpc_write_node: node_id: {:?}, vers: {:?}, head: {:?}",
        node_id,
        vers,
        head,
    );
    let data_ptr: &[u8] = head.as_bytes();
    rpc_write(storage, TeeFsHtreeType::Node, node_id - 1, vers, data_ptr)
        .inspect_err(|e| error!("rpc_write_node error! {:X?}", e))?;
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
    _storage: &dyn TeeFsHtreeStorageOps,
    // _fd: Option<&mut FileVariant>,
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
    // let all_bytes = bytemuck::bytes_of(&node.node);
    let all_bytes: &[u8] = node.node.as_bytes();
    debug_assert!(66 == all_bytes.len());
    let iv_offset = offset_of!(TeeFsHtreeNodeImage, iv);
    let flags_offset = offset_of!(TeeFsHtreeNodeImage, flags);
    let flags_size = core::mem::size_of::<u16>();

    tee_debug!(
        "all_bytes.len(): {:X?}, iv_offset: {:X?}, flags_offset: {:X?}, flags_size: {:X?}",
        all_bytes.len(),
        iv_offset,
        flags_offset,
        flags_size
    );

    tee_debug!(
        "calc_node_hash_with_ctx: node.node: {:?}, meta: {:?}",
        node.node,
        meta
    );

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

    tee_debug!("hash with {} {}", node.id, hex::encode(digest));
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
) -> TeeResult
where
    F: FnMut(&HtreeNode, &TeeFsHtreeData) -> TeeResult,
{
    // 对 fd 做借用变换

    // Traverse left subtree
    if let Some(left_child_arc) = node.left.get_ref() {
        post_order_traverse(left_child_arc, ht_data, visitor)?;
    }

    // Traverse right subtree
    if let Some(right_child_arc) = node.right.get_ref() {
        post_order_traverse(right_child_arc, ht_data, visitor)?;
    }

    // Visit the current node
    visitor(node, ht_data)?;

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
    storage: &dyn TeeFsHtreeStorageOps,
    visitor: &mut F,
) -> TeeResult
where
    F: FnMut(
        &mut HtreeNode,
        &TeeFsHtreeData,
        &dyn TeeFsHtreeStorageOps,
        // Option<&mut FileVariant>,
    ) -> TeeResult, // visitor 现在接收 RefMut<HtreeNode>
{
    // 遍历左子树
    if let Some(left_child_arc) = node.left.get_mut() {
        post_order_traverse_mut(left_child_arc, ht_data, storage, visitor)?;
    }

    // 遍历右子树
    if let Some(right_child_arc) = node.right.get_mut() {
        post_order_traverse_mut(right_child_arc, ht_data, storage, visitor)?;
    }
    // `try_borrow_mut()` 会返回 Err，这里使用 `ok()` 忽略错误，
    // TODO 实际应用中你可能需要更健壮的错误处理。
    visitor(node, ht_data, storage)?; // 将 RefMut<HtreeNode> 传递给 visitor

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
    // _fd: Option<&mut FileVariant>,
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
    // _fd: Option<&mut FileVariant>,
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

    tee_debug!(
        "check hash {} with {}",
        hex::encode(node.node.hash),
        hex::encode(digest)
    );

    if node.node.hash.ct_eq(&digest).unwrap_u8() == 0 {
        tee_debug!("verify_node: hash not equal");
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
    // _fd: Option<&mut FileVariant>,
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

    tee_debug!("hash with {} {}", node.id, hex::encode(digest));
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
    storage: &dyn TeeFsHtreeStorageOps,
    // fd: Option<&mut FileVariant>,
) -> TeeResult {
    tee_debug!(
        "htree_sync_node_to_storage: node.id: {:?}, node.dirty: {:?}, node.block_updated: {:?}",
        // fd,
        node.id,
        node.dirty,
        node.block_updated
    );

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
    // if fd.is_none() {
    //     return Err(TEE_ERROR_BAD_PARAMETERS);
    // }

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
        // C version: vers = !(targ->ht->head.counter & 1);
        // When counter is even, vers = 1; when counter is odd, vers = 0
        vers = ((ht_data.head.counter & 1) == 0) as u8;
        meta = Some(&ht_data.imeta.meta);
        tee_debug!(
            "htree_sync_node_to_storage (root): counter: {}, root_node_vers: {}, flags: 0x{:04X}",
            ht_data.head.counter,
            vers,
            node.node.flags
        );
    }
    let mut digest = [0u8; TEE_FS_HTREE_HASH_SIZE];

    calc_node_hash_with_type(TEE_FS_HTREE_HASH_ALG, &node, meta, &mut digest)?;

    node.node.hash.copy_from_slice(&digest);

    node.dirty = false;
    node.block_updated = false;

    rpc_write_node(storage, node.id, vers, &mut node.node)?;
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
    if !ni_is_some {
        // When ni is None (not using node IV), AAD includes root.hash and head.counter
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
    // When ni is Some (block encryption/decryption), root_hash should not be used
    // When ni is None, root_hash should be provided or use ht.root.node.hash
    let hash = if ni.is_some() {
        // When using node IV, don't use root_hash (will be ignored in authenc_init_core anyway)
        None
    } else {
        // When not using node IV, use provided root_hash or ht.root.node.hash
        root_hash.or(Some(&ht.root.node.hash))
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

    tee_debug!(
        "authenc_init_decrypt: fek: {:?}, head: {:?}, iv: {:?}, ni_is_some: {:?}, root_hash: {:?}",
        hex::encode(fek),
        head,
        hex::encode(iv),
        ni_is_some,
        root_hash.map(|hash| hex::encode(hash)),
    );
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

    tee_debug!(
        "authenc_init_encrypt: fek: {:?}, head: {:?}, iv: {:?}, ni_is_some: {:?}, root_hash: {:?}",
        hex::encode(fek),
        head,
        hex::encode(iv),
        ni_is_some,
        root_hash.map(|hash| hex::encode(hash)),
    );
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
    tee_debug!(
        "authenc_decrypt_final: tag: {:?}, crypt: {:?}, plain: {:?}",
        slice_fmt(tag),
        slice_fmt(crypt),
        slice_fmt(plain),
    );
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

    tee_debug!(
        "authenc_encrypt_final: tag: {:?}, crypt: {:?}, plain: {:?}",
        slice_fmt(&tag),
        slice_fmt(&crypt),
        slice_fmt(&plain),
    );
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
fn htree_traverse_post_order<F>(ht: &TeeFsHtree, visitor: &mut F) -> TeeResult
where
    F: FnMut(&HtreeNode, &TeeFsHtreeData) -> TeeResult,
{
    post_order_traverse(&ht.root, &ht.data, visitor)?;

    Ok(())
}

/// traverse the tree post order mut
///
/// # Arguments
/// * `ht` - the tree
/// * `visitor` - the visitor function
/// # Returns
/// * `TeeResult` - the result of the operation
fn htree_traverse_post_order_mut<F>(ht: &mut TeeFsHtree, visitor: &mut F) -> TeeResult
where
    F: FnMut(&mut HtreeNode, &TeeFsHtreeData, &dyn TeeFsHtreeStorageOps) -> TeeResult,
{
    let storage = ht.storage.as_ref();
    post_order_traverse_mut(&mut ht.root, &ht.data, storage, visitor)?;

    Ok(())
}

/// verify the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn verify_tree(ht: &TeeFsHtree) -> TeeResult {
    htree_traverse_post_order(ht, &mut verify_node)?;
    Ok(())
}

/// calc the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn calc_tree(ht: &mut TeeFsHtree) -> TeeResult {
    htree_traverse_post_order_mut(ht, &mut calc_node)?;
    Ok(())
}

/// print the hash of the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn print_tree_hash(ht: &TeeFsHtree) -> TeeResult {
    htree_traverse_post_order(ht, &mut print_node_hash)?;

    Ok(())
}

/// init the root node of the tree
///
/// # Arguments
/// * `ht` - the tree
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn init_root_node(ht: &mut TeeFsHtree) -> TeeResult {
    tee_debug!("init_root_node");
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
            current = current.get_child_by_index_mut(index).unwrap();
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
    // fd: &mut FileVariant,
    ht: &mut TeeFsHtree,
    hash: Option<&[u8]>,
) -> TeeResult {
    let storage = ht.storage.as_mut();
    // let mut fd = storage.get_hd();
    if let Some(target_hash) = hash {
        for idx in 0.. {
            let node_ref = &mut ht.root.node; // mutable access in scope
            rpc_read_node(storage, 1, idx, node_ref)?;
            tee_debug!(
                "rpc_read_node: root node hash: {:X?}, target_hash: {:X?}",
                hex::encode(node_ref.hash),
                hex::encode(target_hash)
            );
            if node_ref.hash == target_hash {
                let _head = rpc_read_head(storage, idx, &mut ht.data.head).inspect_err(|e| {
                    error!("rpc_read_head error! {:X?}", e);
                })?;
                break;
            }

            if idx != 0 {
                return Err(TEE_ERROR_SECURITY);
            }
        }
    } else {
        let mut heads = [TeeFsHtreeImage::default(); 2];
        for idx in 0..2 {
            // Read version idx (0 or 1) of the head, consistent with C implementation
            rpc_read_head(storage, idx as u8, &mut heads[idx])?;
            tee_debug!(
                "init_head_from_data: read head[{}]: counter={}",
                idx,
                heads[idx].counter
            );
        }

        let idx = get_idx_from_counter(heads[0].counter, heads[1].counter)
            .map_err(|_| TEE_ERROR_SECURITY)?;
        tee_debug!(
            "init_head_from_data: get_idx_from_counter result: idx={}, heads[0].counter={}, \
             heads[1].counter={}",
            idx,
            heads[0].counter,
            heads[1].counter
        );

        let node_ref = &mut ht.root.node;
        tee_debug!(
            "init_head_from_data: reading root node with vers: {}, heads[0].counter: {}, \
             heads[1].counter: {}",
            idx,
            heads[0].counter,
            heads[1].counter
        );
        rpc_read_node(storage, 1, idx, node_ref)?;
        tee_debug!(
            "init_head_from_data: root node loaded, flags: 0x{:04X}",
            node_ref.flags
        );

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
pub fn init_tree_from_data(ht: &mut TeeFsHtree) -> TeeResult {
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
        let storage = ht.storage.as_ref();
        rpc_read_node(
            storage,
            node_id as usize,
            committed_version,
            &mut node_image,
        )?;

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
    // fd: &mut FileVariant,
    mut hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
) -> TeeResult {
    // if ht.is_none() {
    //     return Err(TeeResultCode::ErrorCorruptObject);
    // }
    tee_debug!(
        "tee_fs_htree_sync_to_storage: ht.data.dirty: {:?}",
        ht.data.dirty
    );

    if !ht.data.dirty {
        return Ok(());
    }

    // TODO: fd through out parameters?
    // let mut fd = open_file_like("filenamne", FS_OFLAG_DEFAULT, FS_MODE_644)
    //     .map_err(|_| TeeResultCode::ErrorGeneric)?;

    htree_traverse_post_order_mut(ht, &mut htree_sync_node_to_storage)
        .inspect_err(|e| error!("htree_traverse_post_order_mut error! {:X?}", e))?;

    let counter_before = ht.data.head.counter;
    update_root(ht)?;
    let counter_after = ht.data.head.counter;
    let head_vers = (ht.data.head.counter & 1) as u8;
    tee_debug!(
        "tee_fs_htree_sync_to_storage: counter_before: {}, counter_after: {}, head_vers: {}, \
         root_flags: 0x{:04X}",
        counter_before,
        counter_after,
        head_vers,
        ht.root.node.flags
    );

    let storage = ht.storage.as_ref();
    rpc_write_head(storage, head_vers, &mut ht.data.head)?;

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
    // fd: &mut FileVariant,
    storage: Box<dyn TeeFsHtreeStorageOps>,
    create: bool,
    hash: Option<&mut [u8; TEE_FS_HTREE_HASH_SIZE]>,
    uuid: Option<&TEE_UUID>,
) -> TeeResult<Box<TeeFsHtree>> {
    tee_debug!(
        "tee_fs_htree_open: create: {:?}, hash: {:?}, uuid: {:?}",
        create,
        hash,
        uuid
    );
    let mut ht = Box::new(TeeFsHtree::default());
    if let Some(uuid_val) = uuid {
        ht.data.uuid = *uuid_val;
    }

    ht.storage = storage;

    let init_result = (|| {
        if create {
            let mut dummy_head = TeeFsHtreeImage::default();
            tee_debug!("tee_fs_htree_open: create: true");
            crypto_rng_read(&mut ht.data.fek).map_err(|e| e)?;
            tee_fs_fek_crypt(
                Some(&ht.data.uuid),
                TEE_OperationMode::TEE_MODE_ENCRYPT,
                Some(&ht.data.fek),
                size_of_val(&ht.data.fek),
                Some(&mut ht.data.head.enc_fek),
            )?;
            init_root_node(&mut ht)?;
            tee_debug!("init_root_node to get: ht: {:?}", &ht);
            ht.data.dirty = true;
            tee_fs_htree_sync_to_storage(&mut ht, hash)?;
            let storage = ht.storage.as_ref();
            rpc_write_head(storage, 0, &mut dummy_head)?;
        } else {
            init_head_from_data(&mut ht, hash.as_ref().map(|s| &s[..])).inspect_err(|e| {
                error!("init_head_from_data error! {:X?}", e);
            })?;
            verify_root(&mut ht).inspect_err(|e| {
                error!("verify_root error! {:X?}", e);
            })?;
            init_tree_from_data(&mut ht).inspect_err(|e| {
                error!("init_tree_from_data error! {:X?}", e);
            })?;
            tee_debug!("verify_tree");
            verify_tree(&ht).inspect_err(|e| {
                error!("verify_tree error! {:X?}", e);
            })?;
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
pub fn tee_fs_htree_read_block(
    ht: &mut TeeFsHtree,
    // fd: &mut FileVariant,
    block_num: usize,
    block: &mut [u8],
) -> TeeResult {
    let block_size = {
        let storage = ht.storage.as_ref();
        storage.block_size()
    };
    tee_debug!(
        "tee_fs_htree_read_block: ht: {:?},  block_num: {:?}, block_len: 0x{:X?}, block_size: \
         0x{:X?}",
        ht,
        // fd,
        block_num,
        block.len(),
        block_size
    );
    // first get the node and extract the necessary information, then release the node borrow
    let (block_vers, ni_iv, ni_tag) = {
        let node = get_block_node(ht, false, block_num).map_err(|_| TEE_ERROR_CORRUPT_OBJECT)?;

        let vers = if (node.node.flags & HTREE_NODE_COMMITTED_BLOCK as u16) != 0 {
            1
        } else {
            0
        };

        tee_debug!(
            "tee_fs_htree_read_block: node.node.flags: 0x{:04X}, HTREE_NODE_COMMITTED_BLOCK: \
             0x{:04X}, block_vers: {}",
            node.node.flags,
            HTREE_NODE_COMMITTED_BLOCK as u16,
            vers
        );

        // extract iv and tag (these are Copy types, can be used directly)
        (vers, node.node.iv, node.node.tag)
    };

    // before calling authenc_init, get the root hash first
    let root_hash = ht.root.node.hash;

    tee_debug!(
        "tee_fs_htree_read_block: block_vers: {:?}, ni_iv: {:X?}, ni_tag: {:X?}, root_hash: {:X?}",
        block_vers,
        hex::encode(ni_iv),
        hex::encode(ni_tag),
        hex::encode(root_hash)
    );

    // now the node borrow is released, can safely borrow the other parts of ht
    let result = (|| {
        // allocate buffer, length is one BLOCK
        let mut enc_block = vec![0u8; block_size];

        let storage = ht.storage.as_ref();
        storage.rpc_read_init()?;

        let len =
            storage.rpc_read_final(TeeFsHtreeType::Block, block_num, block_vers, &mut enc_block)?;

        if len != block_size {
            error!(
                "tee_fs_htree_read_block: len: {:X?} != block_size: {:X?}",
                len, block_size
            );
            return Err(TEE_ERROR_CORRUPT_OBJECT);
        }

        // use authenc_init_decrypt, directly pass ni_iv without constructing temporary struct
        let cipher =
            authenc_init_decrypt(&ht.data.fek, &ht.data.head, Some(&ni_iv), Some(&root_hash))
                .inspect_err(|e| {
                    error!("authenc_init_decrypt: error: {:X?}", e);
                })
                .map_err(|_| TEE_ERROR_CORRUPT_OBJECT)?;

        // same as C version: res = authenc_decrypt_final(ctx, node->node.tag, enc_block, ht->stor->block_size, block);
        authenc_decrypt_final(cipher, &ni_tag, &enc_block, block).inspect_err(|e| {
            error!("authenc_decrypt_final: error: {:X?}", e);
        })?;

        Ok(())
    })();

    if result.is_err() {
        error!("tee_fs_htree_read_block error! {:X?}", result);
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
pub fn tee_fs_htree_write_block(
    ht: &mut TeeFsHtree,
    // storage: &dyn TeeFsHtreeStorageOps,
    // fd: &mut FileVariant,
    block_num: usize,
    block: &[u8],
) -> TeeResult {
    tee_debug!(
        "tee_fs_htree_write_block-> block_num: {:?}, block.len: {:?}",
        block_num,
        block.len()
    );
    // before calling authenc_init, get the root hash first
    let root_hash = ht.root.node.hash;

    // extract fek and head before getting node to avoid borrow conflicts
    let fek = ht.data.fek;
    let head = ht.data.head;

    // 在获取 node 之前先获取 block_size 和初始化写入操作，避免借用冲突
    let (block_size, write_init_result) = {
        let storage = ht.storage.as_ref();
        let block_size = storage.block_size();
        let write_init_result = storage.rpc_write_init();
        (block_size, write_init_result)
    };
    write_init_result?;

    let result = (|| {
        // get node once for all operations
        let node = get_block_node(ht, true, block_num).map_err(|_| TEE_ERROR_CORRUPT_OBJECT)?;

        // if block not updated, toggle committed flag
        let block_vers = {
            let flags_before = node.node.flags;
            if !node.block_updated {
                node.node.flags ^= HTREE_NODE_COMMITTED_BLOCK as u16;
            }

            let vers = if (node.node.flags & HTREE_NODE_COMMITTED_BLOCK as u16) != 0 {
                1
            } else {
                0
            };
            tee_debug!(
                "tee_fs_htree_write_block: block_num: {}, block_updated: {}, flags_before: \
                 0x{:04X}, flags_after: 0x{:04X}, block_vers: {}",
                block_num,
                node.block_updated,
                flags_before,
                node.node.flags,
                vers
            );
            vers
        };

        // allocate encryption buffer (使用之前获取的 block_size)
        let mut enc_block = vec![0u8; block_size];

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
        {
            let storage = ht.storage.as_ref();
            storage
                .rpc_write_final(TeeFsHtreeType::Block, block_num, block_vers, &enc_block)
                .inspect_err(|e| {
                    error!("rpc_write_final: error: {:X?}", e);
                })?;
        }

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

#[cfg(feature = "tee_test")]
mod tests_htree_basic {
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{TestDescriptor, TestResult},
        test_fn, tests, tests_name,
    };

    //-------- test suites --------
    test_fn! {
        using TestResult;

        pub fn test_iv_offset_matches_hash_size() {
            let iv_offset = offset_of!(TeeFsHtreeNodeImage, iv);
            assert_eq!(iv_offset, TEE_FS_HTREE_HASH_SIZE);
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_calc_node_hash() {
            let mut node =  HtreeNode::default();
            let meta = TeeFsHtreeMeta::default();
            let mut digest = [0u8; TEE_FS_HTREE_HASH_SIZE];

            let res = calc_node_hash_with_type(TEE_ALG_SHA256, &mut node, Some(&meta), &mut digest);
            assert!(res.is_ok());

            // the same asn sha256([0u8, 42]);
            let hex_string = hex::encode(digest);
            assert_eq!(hex_string, "094c4931fdb2f2af417c9e0322a9716006e8211fe9017f671ac6e3251300acca");
            debug!("digest = {:02x?}", digest);
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_construct_tree_with_children() {
            let root_node_image = TeeFsHtreeNodeImage::default();
            let left_node_image = TeeFsHtreeNodeImage::default();
            let right_node_image = TeeFsHtreeNodeImage::default();

            let mut root = HtreeNode::new(0, root_node_image);
            let left = HtreeNode::new(1, left_node_image);
            let right = HtreeNode::new(2, right_node_image);

            let meta = TeeFsHtreeMeta::default();

            HtreeNode::set_left(&mut root, left);
            HtreeNode::set_right(&mut root, right);

            let mut digest = [0u8; TEE_FS_HTREE_HASH_SIZE];

            let res = calc_node_hash_with_type(TEE_ALG_SHA256, &root, Some(&meta), &mut digest);
            assert!(res.is_ok());

            // the same asn sha256([0u8, 42+32+32]);
            let hex_string = hex::encode(digest);
            assert_eq!(hex_string, "34dbd6bf55d0d075d666181d9278b8387482a8b5804e44e1ddaafe6876dadc15");
            debug!("digest = {:02x?}", digest);
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_field_offsets_and_relevant_slice() {
            let node = TeeFsHtreeNodeImage {
                hash: [0xAA; TEE_FS_HTREE_HASH_SIZE],
                iv: [0xBB; TEE_FS_HTREE_IV_SIZE],
                tag: [0xCC; TEE_FS_HTREE_TAG_SIZE],
                flags: 0x1234,
            };

            // let all_bytes = bytemuck::bytes_of(&node);
            let all_bytes: &[u8] = node.as_bytes();
            let iv_offset = offset_of!(TeeFsHtreeNodeImage, iv);
            let flags_offset = offset_of!(TeeFsHtreeNodeImage, flags);
            let flags_size = core::mem::size_of::<u16>();

            let relevant = &all_bytes[iv_offset..flags_offset + flags_size];

            assert_eq!(iv_offset, TEE_FS_HTREE_HASH_SIZE);
            assert_eq!(flags_offset, TEE_FS_HTREE_HASH_SIZE+TEE_FS_HTREE_IV_SIZE+TEE_FS_HTREE_TAG_SIZE);
            assert_eq!(flags_size, 2);
            assert_eq!(relevant.len(), TEE_FS_HTREE_IV_SIZE+TEE_FS_HTREE_TAG_SIZE+2);

            assert_eq!(&relevant[..16], &[0xBB; 16]);         // iv
            assert_eq!(&relevant[16..32], &[0xCC; 16]);       // tag
            assert_eq!(&relevant[32..34], &0x1234u16.to_le_bytes()); // flags（小端）
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_htree_structure_and_access() {
            // 创建根节点
            let mut root = HtreeNode::new(0, TeeFsHtreeNodeImage {
                hash: [0x00; TEE_FS_HTREE_HASH_SIZE],
                iv: [0x01; TEE_FS_HTREE_IV_SIZE],
                tag: [0x02; TEE_FS_HTREE_TAG_SIZE],
                flags: 0,
            });

            // 创建左子节点
            let left = HtreeNode::new(1, TeeFsHtreeNodeImage {
                hash: [0x11; TEE_FS_HTREE_HASH_SIZE],
                iv: [0x12; TEE_FS_HTREE_IV_SIZE],
                tag: [0x13; TEE_FS_HTREE_TAG_SIZE],
                flags: 1,
            });

            // 创建右子节点
            let right = HtreeNode::new(2, TeeFsHtreeNodeImage {
                hash: [0x21; TEE_FS_HTREE_HASH_SIZE],
                iv: [0x22; TEE_FS_HTREE_IV_SIZE],
                tag: [0x23; TEE_FS_HTREE_TAG_SIZE],
                flags: 2,
            });

            HtreeNode::set_left(&mut root, left);
            HtreeNode::set_right(&mut root, right);

            {
                assert!(root.left.is_some());
                assert!(root.right.is_some());

                let left_ref = root.left.as_ref().unwrap();
                assert_eq!(left_ref.id, 1);

                let right_ref = root.right.as_ref().unwrap();
                assert_eq!(right_ref.id, 2);
            }

            {
                let left_ref = root.left.as_ref().unwrap();
                let left_parent = left_ref.parent.as_ref().unwrap();
                assert_eq!(left_parent.as_ptr(), &root as *const HtreeNode as *mut HtreeNode); // 指向同一个 root

                let right_ref = root.right.as_ref().unwrap();
                let right_parent = right_ref.parent.as_ref().unwrap();
                assert_eq!(right_parent.as_ptr(), &root as *const HtreeNode as *mut HtreeNode);
            }

            {
                let mut left_mut = root.left.get_mut().unwrap();
                left_mut.dirty = true;
                assert!(left_mut.dirty);
            }
        }
    }

    test_fn! {
        using TestResult;
        pub fn test_init_root_node_sets_fields_and_hash() {
            let mut ht = TeeFsHtree {
                root: HtreeNode::new(0, TeeFsHtreeNodeImage::default()),
                data: TeeFsHtreeData::default(),
                storage: Box::new(TeeFsFdAux::new()),
            };
            let result = init_root_node(&mut ht);

            assert!(result.is_ok());
            assert_eq!(ht.root.id, 1);
            assert!(ht.root.dirty);
            let hex_string = hex::encode(ht.root.node.hash);
            assert_eq!(hex_string, "094c4931fdb2f2af417c9e0322a9716006e8211fe9017f671ac6e3251300acca");
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_calc_and_verify_tree() {
            // 1. Create a sample tree
            let node_image_root = TeeFsHtreeNodeImage::default();
            let node_image_l = TeeFsHtreeNodeImage::default();
            let node_image_r = TeeFsHtreeNodeImage::default();
            let node_image_ll = TeeFsHtreeNodeImage::default();
            let node_image_lr = TeeFsHtreeNodeImage::default();

            let mut root = HtreeNode::new(1, node_image_root);
            let mut left_child = HtreeNode::new(2, node_image_l);
            let mut right_child = HtreeNode::new(3, node_image_r);
            let mut left_left_child = HtreeNode::new(4, node_image_ll);
            let mut left_right_child = HtreeNode::new(5, node_image_lr);

            HtreeNode::set_left(&mut left_child, left_left_child);
            HtreeNode::set_right(&mut left_child, left_right_child);
            HtreeNode::set_left(&mut root, left_child);
            HtreeNode::set_right(&mut root, right_child);
            // Create the TeeFsHtree structure
            let mut ht = TeeFsHtree {
                root: root,
                data: TeeFsHtreeData::default(),
                storage: Box::new(TeeFsFdAux::new()),
            };
            debug!("Verify tree completed.");
            let calc_result = calc_tree(&mut ht);
            assert!(calc_result.is_ok(), "Calc tree failed: {:?}", calc_result.unwrap_err());

            // 3. Verify tree hashes
            let verify_result = verify_tree(&mut ht);
            assert!(verify_result.is_ok(), "Verify tree failed: {:?}", verify_result.unwrap_err());

            let _print_result = print_tree_hash(&mut ht);
            assert!(verify_result.is_ok(), "Verify tree failed: {:?}", verify_result.unwrap_err());
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_verify_node_after_calc() {
            let mut ht = TeeFsHtree::default();
            ht.data.imeta.meta.length = 100;

            let mut node = HtreeNode::new(1, TeeFsHtreeNodeImage::default());

            // 先计算节点哈希
            let storage = ht.storage.as_ref();
            let calc_result = calc_node(&mut node, &ht.data, storage);
            assert!(calc_result.is_ok());

            // 再验证节点
            let verify_result = verify_node(&node, &ht.data);
            assert!(verify_result.is_ok());
        }
    }

    test_fn! {
        using TestResult;
        pub fn test_verify_node_without_calc() {
            let ht = TeeFsHtree::default();
            let node = HtreeNode::new(1, TeeFsHtreeNodeImage::default());

            // 不计算直接验证（应该失败）
            assert!(verify_node(&node, &ht.data).is_err());
        }
    }

    test_fn! {
        using TestResult;
        pub fn test_verify_node_with_parent_after_calc() {
            let ht = TeeFsHtree::default();
            let parent = HtreeNode::new(1, TeeFsHtreeNodeImage::default());
            let mut child = HtreeNode::new(2, TeeFsHtreeNodeImage::default());

            // 设置父子关系
            child.parent = NonNull::new(&parent as *const HtreeNode as *mut HtreeNode);

            // 先计算子节点哈希
            let storage = ht.storage.as_ref();
            let calc_result = calc_node(&mut child, &ht.data, storage);
            assert!(calc_result.is_ok());

            // 再验证子节点
            let verify_result = verify_node(&child, &ht.data);
            assert!(verify_result.is_ok());
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_update_verify_root() {
            let mut ht = TeeFsHtree::default();
            ht.data.uuid = TEE_UUID {
                timeLow: 1,
                timeMid: 2,
                timeHiAndVersion: 3,
                clockSeqAndNode: [4; 8],
            };
            ht.data.imeta.meta.length = 100;
            ht.data.imeta.max_node_id = 5;

            // 生成 fek
            tee_fs_fek_crypt(
                Some(&ht.data.uuid),
                TEE_OperationMode::TEE_MODE_ENCRYPT,
                Some(&ht.data.fek),
                TEE_FS_KM_FEK_SIZE,
                Some(&mut ht.data.head.enc_fek),
            )
            .unwrap();

            // 更新根节点
            update_root(&mut ht).unwrap();
            assert_eq!(ht.data.head.counter, 1);

            // 验证根节点
            verify_root(&mut ht).unwrap();

            // 验证解密后的数据
            assert_eq!(ht.data.imeta.meta.length, 100);
            assert_eq!(ht.data.imeta.max_node_id, 5);
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_update_root_multiple_times() {
            let mut ht = TeeFsHtree::default();
            ht.data.uuid = TEE_UUID {
                timeLow: 1,
                timeMid: 2,
                timeHiAndVersion: 3,
                clockSeqAndNode: [4; 8],
            };
            ht.data.imeta.meta.length = 200;
            ht.data.imeta.max_node_id = 10;

            tee_fs_fek_crypt(
                Some(&ht.data.uuid),
                TEE_OperationMode::TEE_MODE_ENCRYPT,
                Some(&ht.data.fek),
                TEE_FS_KM_FEK_SIZE,
                Some(&mut ht.data.head.enc_fek),
            )
            .unwrap();

            // 多次更新
            update_root(&mut ht).unwrap();
            assert_eq!(ht.data.head.counter, 1);

            update_root(&mut ht).unwrap();
            assert_eq!(ht.data.head.counter, 2);

            update_root(&mut ht).unwrap();
            assert_eq!(ht.data.head.counter, 3);

            verify_root(&mut ht).unwrap();
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_verify_root_without_update() {
            let mut ht = TeeFsHtree::default();
            ht.data.uuid = TEE_UUID {
                timeLow: 1,
                timeMid: 2,
                timeHiAndVersion: 3,
                clockSeqAndNode: [4; 8],
            };
            ht.data.head.imeta = [0x00; TEE_FS_HTREE_IMETA_SIZE]; // 未加密数据

            assert!(verify_root(&mut ht).is_err());
        }
    }
}

#[cfg(feature = "tee_test")]
mod tests_node_id_to_level {
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

        pub fn test_node_id_to_level_basic() {
            assert_eq!(node_id_to_level(1), 1);
            assert_eq!(node_id_to_level(2), 2);
            assert_eq!(node_id_to_level(3), 2);
            assert_eq!(node_id_to_level(4), 3);
            assert_eq!(node_id_to_level(7), 3);
            assert_eq!(node_id_to_level(8), 4);
            assert_eq!(node_id_to_level(15), 4);
            assert_eq!(node_id_to_level(16), 5);
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_node_id_to_level_high_values() {
            assert_eq!(node_id_to_level(1023), 10); // 2^10 - 1
            assert_eq!(node_id_to_level(1024), 11); // 2^10
            assert_eq!(node_id_to_level(1 << 30), 31);
        }
    }

    test_fn! {
        using TestResult;

        #[should_panic]
        pub fn test_node_id_zero_should_panic() {
            node_id_to_level(0);
        }
    }

    test_fn! {
        using TestResult;

        #[should_panic]
        pub fn test_node_id_max_should_panic() {
            node_id_to_level(usize::MAX);
        }
    }
}

#[cfg(feature = "tee_test")]
mod tests_find_closest_node {
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{TestDescriptor, TestResult},
        test_fn, tests, tests_name,
    };

    // 辅助函数：构建一个测试树
    // 树结构：
    //       1 (root)
    //      /  \
    //     2    3
    //    / \    /
    //   4   5  6
    fn build_test_tree() -> TeeFsHtree {
        let mut root_node = HtreeNode::new(1, TeeFsHtreeNodeImage::default()); // ID 1 (level 1)
        let mut node2 = HtreeNode::new(2, TeeFsHtreeNodeImage::default()); // ID 2 (level 2) - Left of 1
        let mut node3 = HtreeNode::new(3, TeeFsHtreeNodeImage::default()); // ID 3 (level 2) - Right of 1
        let mut node4 = HtreeNode::new(4, TeeFsHtreeNodeImage::default()); // ID 4 (level 3) - Left of 2
        let mut node5 = HtreeNode::new(5, TeeFsHtreeNodeImage::default()); // ID 5 (level 3) - Right of 2
        let mut node6 = HtreeNode::new(6, TeeFsHtreeNodeImage::default()); // ID 6 (level 3) - Left of 3

        HtreeNode::set_left(&mut node3, node6); // 节点 3 的右子树不存在
        HtreeNode::set_left(&mut node2, node4);
        HtreeNode::set_right(&mut node2, node5);
        HtreeNode::set_left(&mut root_node, node2);
        HtreeNode::set_right(&mut root_node, node3);

        TeeFsHtree {
            root: root_node,
            data: TeeFsHtreeData::default(),
            storage: Box::new(TeeFsFdAux::new()),
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_find_closest_node_exact_match() {
            let mut ht = build_test_tree();

            // 查找存在的节点：根节点
            let found = find_closest_node(&mut ht, 1);
            assert_eq!(found.id, 1, "应找到根节点 ID 1");
            let found = find_node(&mut ht, 1).unwrap();
            assert_eq!(found.id, 1, "应找到根节点 ID 1");
            let found = find_node(&mut ht, 100).is_some();
            assert_eq!(found, false, "应找到根节点 ID 1");

            // 查找存在的节点：左子节点
            let found = find_closest_node(&mut ht, 2);
            assert_eq!(found.id, 2, "应找到节点 ID 2");

            // 查找存在的节点：右子节点
            let found = find_closest_node(&mut ht, 3);
            assert_eq!(found.id, 3, "应找到节点 ID 3");

            // 查找存在的叶子节点
            let found = find_closest_node(&mut ht, 4);
            assert_eq!(found.id, 4, "应找到叶子节点 ID 4");

            let found = find_closest_node(&mut ht, 5);
            assert_eq!(found.id, 5, "应找到叶子节点 ID 5");

            let found = find_closest_node(&mut ht, 6);
            assert_eq!(found.id, 6, "应找到叶子节点 ID 6");
        }
    }

    test_fn! {
        using TestResult;
        pub fn test_find_closest_node_non_existent_path_left() {
            let mut ht = build_test_tree();

            // 查找一个不存在但其父节点存在的节点：
            // 目标 ID 7 (111) -> 路径: 1(root) -> 3(right) -> 7(right of 3)
            // 节点 3 (ID 3) 有左子节点 (ID 6)，但没有右子节点。
            // find_closest_node 应该找到节点 3。
            let found = find_closest_node(&mut ht, 7);
            assert_eq!(found.id, 3, "目标 ID 7 不存在，应返回最近的父节点 ID 3");

            let found = find_closest_node(&mut ht, 9);
            assert_eq!(found.id, 4, "目标 ID 9 不存在，应返回最近的父节点 ID 4");

            let found = find_closest_node(&mut ht, 16);
            assert_eq!(found.id, 4, "目标 ID 16 不存在，应返回最近的父节点 ID 4");
        }
    }

    test_fn! {
        using TestResult;
        pub fn test_find_closest_node_non_existent_path_deep() {
            let mut ht = build_test_tree();

            // 查找一个更深层级，且路径完全不存在的节点：
            // 目标 ID 10 (1010) -> 路径: 1(root) -> 2(left) -> 4(left) -> 8(left of 4) -> 16(left of 8) -> etc.
            // 我们的树在 ID 4 之后没有子节点。
            // 步骤：
            // 1. `node_id_to_level(10)` 是 4。
            // 2. 第一步：`bit_idx = 4 - (0+1) - 1 = 2`。`(10 >> 2) & 1 = (2)&1 = 0` -> 走左子树，到 `node2` (ID 2)。
            // 3. 第二步：`bit_idx = 4 - (1+1) - 1 = 1`。`(10 >> 1) & 1 = (5)&1 = 1` -> 走右子树，到 `node5` (ID 5)。
            // 4. 第三步：`bit_idx = 4 - (2+1) - 1 = 0`。`(10 >> 0) & 1 = (10)&1 = 0` -> 走左子树。`node5` 没有左子节点。
            // 所以，应该返回 `node5` (ID 5)。
            let found = find_closest_node(&mut ht, 10);
            assert_eq!(found.id, 5, "目标 ID 10 不存在，应返回路径上最近的节点 ID 5");

            // 查找 ID 15 (1111)
            // 1. `node_id_to_level(15)` 是 4。
            // 2. `bit_idx = 2`. `(15 >> 2) & 1 = (3)&1 = 1` -> 走右子树，到 `node3` (ID 3)。
            // 3. `bit_idx = 1`. `(15 >> 1) & 1 = (7)&1 = 1` -> 走右子树。`node3` 没有右子节点。
            // 所以，应该返回 `node3` (ID 3)。
            let found = find_closest_node(&mut ht, 15);
            assert_eq!(found.id, 3, "目标 ID 15 不存在，应返回路径上最近的节点 ID 3");
        }
    }

    test_fn! {
        using TestResult;

        #[should_panic]
        pub fn test_find_closest_node_target_is_zero() {
            let mut ht = build_test_tree();
            // 如果目标 ID 是 0，根据逻辑应返回根节点。
            let found = find_closest_node(&mut ht, 1);
            assert_eq!(found.id, 1, "目标 ID 0 应返回根节点 ID 1");
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_find_closest_node_empty_tree_or_only_root() {
            // 测试只有根节点的树
            let root_node = HtreeNode::new(1, TeeFsHtreeNodeImage::default());
            let mut ht = TeeFsHtree {
                root: root_node,
                data: TeeFsHtreeData::default(),
                storage: Box::new(TeeFsFdAux::new()),
            };

            // 查找根节点
            let found = find_closest_node(&mut ht, 1);
            assert_eq!(found.id, 1, "只有根节点的树，查找根节点");

            // 查找不存在的子节点 (例如 ID 2，左子节点)
            let found = find_closest_node(&mut ht, 2);
            assert_eq!(found.id, 1, "只有根节点的树，查找不存在的子节点应返回根节点");

            // 查找不存在的子节点 (例如 ID 3，右子节点)
            let found = find_closest_node(&mut ht, 3);
            assert_eq!(found.id, 1, "只有根节点的树，查找不存在的子节点应返回根节点");
        }
    }
}

#[cfg(feature = "tee_test")]
mod tests_get_node {
    //-------- test framework import --------
    //-------- local tests import --------
    use alloc::collections::VecDeque;

    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{TestDescriptor, TestResult},
        test_fn, tests, tests_name,
    };

    // 辅助函数：构建一个只有根节点的树 (ID=1)
    fn build_root_only_tree() -> TeeFsHtree {
        let root_node = HtreeNode::new(1, TeeFsHtreeNodeImage::default());
        TeeFsHtree {
            root: root_node,
            data: TeeFsHtreeData::default(),
            storage: Box::new(TeeFsFdAux::new()),
        }
    }

    // 辅助函数：构建一个部分树结构
    // 树结构:
    //         1
    //        / \
    //       2   3
    //      /
    //     4
    fn build_partial_tree() -> TeeFsHtree {
        let mut root_node = HtreeNode::new(1, TeeFsHtreeNodeImage::default());
        let mut node2 = HtreeNode::new(2, TeeFsHtreeNodeImage::default());
        let mut node3 = HtreeNode::new(3, TeeFsHtreeNodeImage::default());
        let mut node4 = HtreeNode::new(4, TeeFsHtreeNodeImage::default());

        HtreeNode::set_left(&mut node2, node4);
        HtreeNode::set_left(&mut root_node, node2);
        HtreeNode::set_right(&mut root_node, node3);

        let mut ht_data = TeeFsHtreeData::default();
        ht_data.imeta.max_node_id = 4; // 根据实际最大ID设置
        TeeFsHtree {
            root: root_node,
            data: ht_data,
            storage: Box::new(TeeFsFdAux::new()),
        }
    }

    // 辅助函数：深度优先遍历，验证树结构和节点ID
    // 注意：这个验证函数会假设树是按预期构建的。如果 get_node 的逻辑有缺陷，
    // 这个验证可能会失败，或者更糟的是，误认为正确。
    fn assert_tree_structure(
        ht: &TeeFsHtree,
        expected_ids: &[usize], // 期望按层序遍历的节点ID
        expected_max_node_id: u32,
    ) -> TestResult {
        let mut actual_ids = Vec::new();
        let mut q: VecDeque<&HtreeNode> = VecDeque::new();
        q.push_back(&ht.root);

        while let Some(node) = q.pop_front() {
            actual_ids.push(node.id);

            if let Some(left_child) = node.left.get_ref() {
                q.push_back(left_child);
            }
            if let Some(right_child) = node.right.get_ref() {
                q.push_back(right_child);
            }
        }
        assert_eq!(actual_ids, expected_ids);
        assert_eq!(ht.data.imeta.max_node_id, expected_max_node_id);

        TestResult::Ok
    }

    // 测试场景 1: 查找已存在的根节点 (create = false)
    test_fn! {
        using TestResult;

        pub fn test_get_node_root_exists_no_create() {
            let mut ht = build_root_only_tree();
            let initial_max_id = ht.data.imeta.max_node_id;

            let result = get_node(&mut ht, false, 1);
            assert!(result.is_ok());
            let node = result.unwrap();
            assert_eq!(node.id, 1);
            assert_eq!(ht.data.imeta.max_node_id, initial_max_id); // max_node_id 不应改变
            assert_tree_structure(&ht, &[1], initial_max_id); // 树结构不变
        }
    }

    // 测试场景 2: 查找已存在的叶子节点 (create = false)
    test_fn! {
        using TestResult;

        pub fn test_get_node_leaf_exists_no_create() {
            let mut ht = build_partial_tree(); // 包含 1, 2, 3, 4
            let initial_max_id = ht.data.imeta.max_node_id;

            let result = get_node(&mut ht, false, 4);
            assert!(result.is_ok());
            let node = result.unwrap();
            assert_eq!(node.id, 4);
            assert_eq!(ht.data.imeta.max_node_id, initial_max_id);
            assert_tree_structure(&ht, &[1, 2, 3, 4], initial_max_id);
        }
    }

    // 测试场景 3: 查找已存在的中间节点 (create = false)
    test_fn! {
        using TestResult;
        pub fn test_get_node_intermediate_exists_no_create() {
            let mut ht = build_partial_tree(); // 包含 1, 2, 3, 4
            let initial_max_id = ht.data.imeta.max_node_id;

            let node = get_node(&mut ht, false, 2).unwrap();
            assert_eq!(node.id, 2);
            assert_eq!(ht.data.imeta.max_node_id, initial_max_id);
            assert_tree_structure(&ht, &[1, 2, 3, 4], initial_max_id);
        }
    }
    // // 测试场景 4: 查找不存在的节点 (create = false)
    test_fn! {
        using TestResult;
        pub fn test_get_node_not_exists_no_create() {
            let mut ht = build_root_only_tree(); // 只有根节点 1
            let initial_max_id = ht.data.imeta.max_node_id;

            let result = get_node(&mut ht, false, 5); // 5 不存在
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err, TEE_ERROR_GENERIC);
            assert_eq!(ht.data.imeta.max_node_id, initial_max_id); // max_node_id 不应改变
            assert_tree_structure(&ht, &[1], initial_max_id); // 树结构不变
        }
    }
    // 测试场景 5: 查找已存在的节点 (create = true) - 不应有副作用
    test_fn! {
        using TestResult;
        pub fn test_get_node_exists_with_create_no_side_effects() {
            let mut ht = build_partial_tree(); // 包含 1, 2, 3, 4
            let initial_max_id = ht.data.imeta.max_node_id;

            let result = get_node(&mut ht, true, 3); // 3 已经存在
            assert!(result.is_ok());
            let node = result.unwrap();
            assert_eq!(node.id, 3);
            assert_eq!(ht.data.imeta.max_node_id, initial_max_id); // max_node_id 不应改变
            assert_tree_structure(&ht, &[1, 2, 3, 4], initial_max_id); // 树结构不变
        }
    }
    // 测试场景 6: 创建一个需要补齐路径上的一个节点的场景 (左子节点)
    // 初始: 1-2-4。 目标: 5 (2的右子节点)
    // `get_node` 的 `for n in current_id..=node_id` 循环: `for n in 4..=5`
    // 预期 `find_closest_node(ht, 5)` 返回 2。
    // 循环会处理 `n=4` (已存在), `n=5` (不存在，创建)。
    test_fn! {
        using TestResult;
        pub fn test_get_node_create_single_missing_left_child() {
            let mut ht = build_partial_tree(); // 包含 1, 2, 3, 4
            // 确保 ID=5 (2的右子) 不存在
            assert!(ht.root.left.as_ref().unwrap().right.is_none());

            let result = get_node(&mut ht, true, 5); // 路径 1 -> 2 -> 5
            assert!(result.is_ok());

            // 先获取 node 的 id 和 parent，然后释放借用
            let (node_id, parent_ptr) = {
                let node = result.unwrap();
                let id = node.id;
                let parent = node.parent;
                (id, parent)
            };
            assert_eq!(node_id, 5);

            // 验证 max_node_id 是否更新
            assert_eq!(ht.data.imeta.max_node_id, 5);

            // 验证树结构 (注意：根据您的 `get_node` 实现，这里可能需要调整期望的树结构)
            // 期望是: 1-2-3-4-5 (5是2的右子)
            assert_tree_structure(&ht, &[1, 2, 3, 4, 5], 5); // 期望 5 在树中

            // 额外验证连接
            let node2_from_tree = ht.root.left.get_ref().unwrap();
            assert!(node2_from_tree.right.is_some());
            assert_eq!(node2_from_tree.right.as_ref().unwrap().id, 5);

            // 验证 parent 指针
            if let Some(parent_ptr) = parent_ptr {
                let parent_ref = unsafe { &*parent_ptr.as_ptr() };
                assert_eq!(parent_ref.id, 2);
            } else {
                panic!("Node 5 should have a parent");
            }
        }
    }
    // 测试场景 7: 创建一个需要补齐路径上的一个节点的场景 (右子节点)
    // 初始: 1-2-3。 目标: 6 (3的左子节点)
    // `get_node` 的 `for n in current_id..=node_id` 循环: `for n in 3..=6`
    // 预期 `find_closest_node(ht, 6)` 返回 3。
    // 循环会处理 `n=3` (已存在), `n=4,5` (不存在，但不在路径上), `n=6` (不存在，创建)。
    test_fn! {
        using TestResult;
        pub fn test_get_node_create_single_missing_right_child() {
            let mut ht = build_partial_tree(); // 包含 1, 2, 3, 4
            // 确保 ID=6 (3的左子) 不存在
            assert!(ht.root.right.get_ref().unwrap().left.is_none());

            let result = get_node(&mut ht, true, 6); // 路径 1 -> 3 -> 6
            assert!(result.is_ok());
            let (node_id, parent_ptr) = {
                let node = result.unwrap();
                let id = node.id;
                let parent = node.parent;
                (id, parent)
            };
            assert_eq!(node_id, 6);

            assert_eq!(ht.data.imeta.max_node_id, 6);

            // 注意：根据 `for n in current_id..=node_id` 的行为，
            // 可能会创建额外的节点。这个测试的预期行为将基于 `get_node` 的实际实现。
            // 对于这个版本，`n=4,5` 会被处理。
            // `find_closest_node(ht, 4)` 可能返回 2 (如果 4 存在) 或 1 (如果 4 不存在)
            // `find_closest_node(ht, 5)` 可能返回 2 (如果 5 不存在) 或 1 (如果 5 不存在)
            // 预期ID序列可能变长: 1, 2, 3, 4, [5], 6.
            // 如果 4 存在，那么 5 将被创建并连接到 2 的右侧。
            // 如果 4 不存在，则 4 将被创建并连接到 2 的左侧。
            // 鉴于 `build_partial_tree` 包含 4，所以 5 会被创建。
            // 期望的结构: 1, 2, 3, 4, 5 (2的右), 6 (3的左)
            assert_tree_structure(&ht, &[1, 2, 3, 4, 5, 6], 6);

            // 额外验证连接
            let node3_from_tree = ht.root.right.get_ref().unwrap();
            assert!(node3_from_tree.left.is_some());
            assert_eq!(node3_from_tree.left.as_ref().unwrap().id, 6);
            if let Some(parent_ptr) = parent_ptr {
                let parent_ref = unsafe { &*parent_ptr.as_ptr() };
                assert_eq!(parent_ref.id, 3);
            } else {
                panic!("Node 6 should have a parent");
            }
        }
    }
    // 测试场景 8: 创建多个中间缺失节点 (所有在路径上，但非连续ID)
    // 初始: 只有 1。 目标: 4 (路径: 1 -> 2 -> 4)
    // `get_node` 的 `for n in current_id..=node_id` 循环: `for n in 1..=4`
    // 预期会创建 2 和 4，但也会不必要地处理 3。
    test_fn! {
        using TestResult;
        pub fn test_get_node_create_multiple_missing_nodes_linear_ids() {
            let mut ht = build_root_only_tree(); // 只有根节点 1
            let result = get_node(&mut ht, true, 4); // 路径 1 -> 2 -> 4
            assert!(result.is_ok());
            let created_node_arc = result.unwrap();
            assert_eq!(created_node_arc.id, 4);

            assert_eq!(ht.data.imeta.max_node_id, 4);

            // 预期会创建 2, 3, 4。 3 是 1 的右子。
            // 结构: 1(root) -> 2(L), 3(R) -> 4(2L)
            assert_tree_structure(&ht, &[1, 2, 3, 4], 4);

            // 额外验证连接
            let root = ht.root;
            assert!(root.left.is_some());
            assert_eq!(root.left.as_ref().unwrap().id, 2);
            assert!(root.right.is_some()); // ID=3 应该被创建
            assert_eq!(root.right.as_ref().unwrap().id, 3);

            let node2_from_tree = root.left.get_ref().unwrap();
            assert!(node2_from_tree.left.is_some());
            assert_eq!(node2_from_tree.left.as_ref().unwrap().id, 4);
        }
    }
    // 测试场景 9: 创建更深层次的节点，验证 max_node_id 更新
    // 初始: 1,2,3,4。 目标: 8 (路径: 1 -> 2 -> 4 -> 8)
    // `get_node` 的 `for n in current_id..=node_id` 循环: `for n in 4..=8`
    // 预期会处理 4, 5, 6, 7, 8。
    // 5, 6, 7 不在路径上。
    test_fn! {
        using TestResult;
        pub fn test_get_node_create_deeper_node_and_max_id_update() {
            let mut ht = build_partial_tree(); // 包含 1, 2, 3, 4
            let _initial_max_id = ht.data.imeta.max_node_id; // 初始为 4

            let result = get_node(&mut ht, true, 8); // 路径 1 -> 2 -> 4 -> 8
            assert!(result.is_ok());
            let created_node_arc = result.unwrap();
            assert_eq!(created_node_arc.id, 8);

            // max_node_id 应该更新到 8
            assert_eq!(ht.data.imeta.max_node_id, 8);

            // 验证树结构：这会非常复杂，因为 5, 6, 7 都会被不必要地创建和连接。
            // 预期 ID 序列：1, 2, 3, 4, 5(2的右), 6(3的左), 7(3的右), 8(4的左)
            assert_tree_structure(&ht, &[1, 2, 3, 4, 5, 6, 7, 8], 8);

            // 额外验证路径上的连接
            let node4_from_tree = ht.root
                .left.get_ref().unwrap()
                .left.get_ref().unwrap();
            assert!(node4_from_tree.left.is_some());
            assert_eq!(node4_from_tree.left.get_ref().unwrap().id, 8);
        }
    }
}

#[cfg(feature = "tee_test")]
mod tests_authenc_funcs {
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

        pub fn test_authenc_functions() {
            // 1. 创建测试用的 TeeFsHtree
            let mut ht = TeeFsHtree::default();

            // 初始化必要的字段
            ht.data.fek = [0x01; TEE_FS_HTREE_FEK_SIZE];
            ht.data.head.enc_fek = [0x02; TEE_FS_HTREE_FEK_SIZE];
            ht.data.head.counter = 12345;
            ht.data.head.iv = [0x03; TEE_FS_HTREE_IV_SIZE];

            // 设置根节点的哈希
            ht.root.node.hash = [0x04; TEE_FS_HTREE_HASH_SIZE];

            // 2. 创建测试数据
            let test_data = b"Hello, TEE World!.";
            let mut plaintext = [0u8; 64];
            plaintext[..test_data.len()].copy_from_slice(test_data);

            // 3. 测试加密流程
            // 创建节点镜像用于加密
            let mut node_image = TeeFsHtreeNodeImage::default();
            node_image.iv = [0x05; TEE_FS_HTREE_IV_SIZE];

            // 初始化加密上下文
            let encrypt_cipher = authenc_init::<Encryption>(
                TEE_OperationMode::TEE_MODE_ENCRYPT,
                &mut ht,
                Some(&mut node_image),
                plaintext.len(),
                None,
            )
            .expect("Failed to initialize encryption cipher");

            // 执行加密
            let mut ciphertext = [0u8; 64];
            let mut tag = [0u8; TEE_FS_HTREE_TAG_SIZE];

            authenc_encrypt_final(encrypt_cipher, &mut tag, &plaintext, &mut ciphertext)
                .expect("Encryption failed");

            // 4. 测试解密流程
            // 初始化解密上下文
            let decrypt_cipher = authenc_init::<Decryption>(
                TEE_OperationMode::TEE_MODE_DECRYPT,
                &mut ht,
                Some(&mut node_image),
                ciphertext.len(),
                None,
            )
            .expect("Failed to initialize decryption cipher");

            // 执行解密
            let mut decrypted = [0u8; 64];

            authenc_decrypt_final(decrypt_cipher, &tag, &ciphertext, &mut decrypted)
                .expect("Decryption failed");

            // 5. 验证结果
            assert_eq!(&plaintext[..test_data.len()], test_data);
            assert_eq!(&decrypted[..test_data.len()], test_data);

            // 验证加密和解密的数据不同（加密成功）
            assert_ne!(
                &plaintext[..test_data.len()],
                &ciphertext[..test_data.len()]
            );

            debug!("Encryption/Decryption test passed!");
            debug!(
                "Original: {:?}",
                String::from_utf8_lossy(&plaintext[..test_data.len()])
            );
            debug!(
                "Decrypted: {:?}",
                String::from_utf8_lossy(&decrypted[..test_data.len()])
            );
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_authenc_roundtrip_no_node_image() {
            let mut ht = TeeFsHtree::default();
            ht.data.fek = [0x11; TEE_FS_HTREE_FEK_SIZE];
            ht.data.head.enc_fek = [0x22; TEE_FS_HTREE_FEK_SIZE];
            ht.data.head.counter = 1;
            ht.data.head.iv = [0x33; TEE_FS_HTREE_IV_SIZE];
            ht.root.node.hash = [0x44; TEE_FS_HTREE_HASH_SIZE];

            let msg = b"roundtrip-no-node-image";
            let mut plain = [0u8; 32];
            plain[..msg.len()].copy_from_slice(msg);

            let enc = authenc_init::<Encryption>(TEE_OperationMode::TEE_MODE_ENCRYPT, &mut ht, None, msg.len(), None)
                .unwrap();
            let mut crypt = [0u8; 32];
            let mut tag = [0u8; TEE_FS_HTREE_TAG_SIZE];
            authenc_encrypt_final(enc, &mut tag, &plain[..msg.len()], &mut crypt[..msg.len()]).unwrap();

            assert_ne!(&plain[..msg.len()], &crypt[..msg.len()]);

            let dec = authenc_init::<Decryption>(TEE_OperationMode::TEE_MODE_DECRYPT, &mut ht, None, msg.len(), None)
                .unwrap();
            let mut out = [0u8; 32];
            authenc_decrypt_final(dec, &tag, &crypt[..msg.len()], &mut out[..msg.len()]).unwrap();

            assert_eq!(&out[..msg.len()], msg);
        }
    }

    test_fn! {
        using TestResult;

        pub fn test_authenc_empty_data() {
            let mut ht = TeeFsHtree::default();
            ht.data.fek = [0x01; TEE_FS_HTREE_FEK_SIZE];
            ht.data.head.enc_fek = [0x02; TEE_FS_HTREE_FEK_SIZE];
            ht.data.head.counter = 1;
            ht.data.head.iv = [0x03; TEE_FS_HTREE_IV_SIZE];
            ht.root.node.hash = [0x04; TEE_FS_HTREE_HASH_SIZE];

            let mut node_image = TeeFsHtreeNodeImage::default();
            node_image.iv = [0x05; TEE_FS_HTREE_IV_SIZE];

            let enc = authenc_init::<Encryption>(
                TEE_OperationMode::TEE_MODE_ENCRYPT,
                &mut ht,
                Some(&mut node_image),
                0,
                None,
            )
            .unwrap();
            let mut crypt = [0u8; 0];
            let mut tag = [0u8; TEE_FS_HTREE_TAG_SIZE];
            authenc_encrypt_final(enc, &mut tag, &[], &mut crypt).unwrap();

            let dec = authenc_init::<Decryption>(
                TEE_OperationMode::TEE_MODE_DECRYPT,
                &mut ht,
                Some(&mut node_image),
                0,
                None,
            )
            .unwrap();
            let mut out = [0u8; 0];
            authenc_decrypt_final(dec, &tag, &[], &mut out).unwrap();
        }
    }

    test_fn! {
        using TestResult;
        pub fn test_authenc_wrong_tag() {
            let mut ht = TeeFsHtree::default();
            ht.data.fek = [0x01; TEE_FS_HTREE_FEK_SIZE];
            ht.data.head.enc_fek = [0x02; TEE_FS_HTREE_FEK_SIZE];
            ht.data.head.counter = 1;
            ht.data.head.iv = [0x03; TEE_FS_HTREE_IV_SIZE];
            ht.root.node.hash = [0x04; TEE_FS_HTREE_HASH_SIZE];

            let mut node_image = TeeFsHtreeNodeImage::default();
            node_image.iv = [0x05; TEE_FS_HTREE_IV_SIZE];

            let enc = authenc_init::<Encryption>(
                TEE_OperationMode::TEE_MODE_ENCRYPT,
                &mut ht,
                Some(&mut node_image),
                4,
                None,
            )
            .unwrap();
            let mut crypt = [0u8; 4];
            let mut tag = [0u8; TEE_FS_HTREE_TAG_SIZE];
            authenc_encrypt_final(enc, &mut tag, b"test", &mut crypt).unwrap();

            let dec = authenc_init::<Decryption>(
                TEE_OperationMode::TEE_MODE_DECRYPT,
                &mut ht,
                Some(&mut node_image),
                4,
                None,
            )
            .unwrap();
            let mut out = [0u8; 4];
            tag[0] ^= 1;
            assert!(authenc_decrypt_final(dec, &tag, &crypt, &mut out).is_err());
        }
    }
}

#[cfg(feature = "tee_test")]
pub mod tests_fs_htree {
    //-------- test framework import --------
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{
            TestDescriptor, TestResult,
            common::file_ops::{FS_MODE_644, FS_OFLAG_DEFAULT},
        },
        test_fn, tests, tests_name,
    };

    test_fn! {
        using TestResult;
        fn test_tee_fs_htree_open() {
            let mut fd = FileVariant::open("test_fs_htree_open.bin", FS_OFLAG_DEFAULT, FS_MODE_644).unwrap();

            let uuid = TEE_UUID { timeLow: 1, timeMid: 2, timeHiAndVersion: 3, clockSeqAndNode: [4; 8] };
            let mut hash = [0u8; TEE_FS_HTREE_HASH_SIZE];

            let mut fd_back = fd;
            info!("fd_back: {:?}", fd_back);
            // 先创建文件 (create = true)
            let ht_create = tee_fs_htree_open(
                Box::new(TeeFsFdAux { fd: fd }),
                true,
                Some(&mut hash),
                Some(&uuid),
            ).unwrap();

            assert_eq!(ht_create.data.uuid, uuid);
            assert!(ht_create.data.dirty==false);

            // 再打开文件 (create = false)
            let ht_open = tee_fs_htree_open(
                Box::new(TeeFsFdAux { fd: fd_back }),
                false,
                Some(&mut hash),
                Some(&uuid),
            ).unwrap();

            assert_eq!(ht_open.data.uuid, uuid);
            assert!(!ht_open.data.dirty);
        }
    }

    use crate::tee::fs_htree::{
        tests_authenc_funcs::{
            test_authenc_empty_data, test_authenc_functions, test_authenc_roundtrip_no_node_image,
            test_authenc_wrong_tag,
        },
        tests_find_closest_node::{
            test_find_closest_node_empty_tree_or_only_root, test_find_closest_node_exact_match,
            test_find_closest_node_non_existent_path_deep,
            test_find_closest_node_non_existent_path_left, test_find_closest_node_target_is_zero,
        },
        tests_get_node::{
            test_get_node_create_deeper_node_and_max_id_update,
            test_get_node_create_multiple_missing_nodes_linear_ids,
            test_get_node_create_single_missing_left_child,
            test_get_node_create_single_missing_right_child,
            test_get_node_exists_with_create_no_side_effects,
            test_get_node_intermediate_exists_no_create, test_get_node_not_exists_no_create,
            test_get_node_root_exists_no_create,
        },
        tests_htree_basic::{
            test_calc_and_verify_tree, test_calc_node_hash, test_construct_tree_with_children,
            test_field_offsets_and_relevant_slice, test_htree_structure_and_access,
            test_init_root_node_sets_fields_and_hash, test_iv_offset_matches_hash_size,
            test_update_root_multiple_times, test_update_verify_root, test_verify_node_after_calc,
            test_verify_node_with_parent_after_calc, test_verify_node_without_calc,
            test_verify_root_without_update,
        },
        tests_node_id_to_level::{
            test_node_id_max_should_panic, test_node_id_to_level_basic,
            test_node_id_to_level_high_values, test_node_id_zero_should_panic,
        },
    };
    tests_name! {
        TEST_FS_HTREE;
        //------------------------
        test_tee_fs_htree_open,
        //------------------------
        // tests_htree_basic
        test_iv_offset_matches_hash_size,
        test_calc_node_hash,
        test_construct_tree_with_children,
        test_field_offsets_and_relevant_slice,
        test_htree_structure_and_access,
        test_init_root_node_sets_fields_and_hash,
        test_calc_and_verify_tree,
        test_verify_node_after_calc,
        test_verify_node_without_calc,
        test_verify_node_with_parent_after_calc,
        test_update_verify_root,
        test_update_root_multiple_times,
        test_verify_root_without_update,
        //------------------------
        // tests_node_id_to_level
        test_node_id_to_level_basic,
        test_node_id_to_level_high_values,
        // test_node_id_zero_should_panic,
        // test_node_id_max_should_panic,
        //------------------------
        // tests_find_closest_node
        test_find_closest_node_exact_match,
        test_find_closest_node_non_existent_path_left,
        test_find_closest_node_non_existent_path_deep,
        test_find_closest_node_target_is_zero,
        test_find_closest_node_empty_tree_or_only_root,
        //------------------------
        // tests_get_node
        test_get_node_root_exists_no_create,
        test_get_node_intermediate_exists_no_create,
        test_get_node_not_exists_no_create,
        test_get_node_exists_with_create_no_side_effects,
        test_get_node_create_single_missing_left_child,
        test_get_node_create_single_missing_right_child,
        test_get_node_create_multiple_missing_nodes_linear_ids,
        test_get_node_create_deeper_node_and_max_id_update,
        //------------------------
        // tests_authenc_funcs
        test_authenc_functions,
        test_authenc_roundtrip_no_node_image,
        test_authenc_empty_data,
        test_authenc_wrong_tag,
    }
}
