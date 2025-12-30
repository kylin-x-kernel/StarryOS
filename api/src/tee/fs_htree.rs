// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, vec::Vec, vec};
use core::ptr::NonNull;

use bytemuck::{Pod, Zeroable};
use mbedtls::{
    cipher::{
        Authenticated, CipherData, Decryption, Encryption, Fresh, raw, Cipher, Operation,
    },
    hash::{Md},
};
use memoffset::offset_of;
use subtle::ConstantTimeEq;
use tee_raw_sys::{
    TEE_ALG_AES_ECB_NOPAD, TEE_ALG_SHA256, TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_CORRUPT_OBJECT,
    TEE_ERROR_GENERIC, TEE_ERROR_MAC_INVALID, TEE_ERROR_NOT_SUPPORTED, TEE_ERROR_SHORT_BUFFER,
    TEE_OperationMode, TEE_UUID, TEE_ALG_AES_GCM, TEE_ALG_HMAC_SHA256, 
};

use super::utee_defines::{TEE_AES_BLOCK_SIZE, TEE_SHA256_HASH_SIZE, TeeResultCode};
use crate::tee::{
    TeeResult,
    common::file_ops::FileVariant,
    crypto_temp::crypto_temp::{
        crypto_hash_alloc_ctx, crypto_hash_final, crypto_hash_init, crypto_hash_update,
    },
    tee_ree_fs::{
        crypto_rng_read, ree_fs_rpc_read_init as rpc_read_init,
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

pub fn calc_node_hash(
    node: &HtreeNode,
    meta: &TeeFsHtreeMeta,
    digest: &mut [u8; TEE_FS_HTREE_HASH_SIZE],
) -> TeeResult {
    calc_node_hash_with_type(TEE_ALG_SHA256, node, Some(meta), digest)
}

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
pub fn free_node(
    _node: &HtreeNode,
    _ht_data: &TeeFsHtreeData,
    _fd: Option<&mut FileVariant>,
) -> TeeResult {
    Ok(())
}

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

fn create_cipher<M: Operation>(
    alg: TEE_ALG,
    key_bytes: usize,
) -> Result<Cipher<M, Authenticated, Fresh>, TeeResultCode> {
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

pub fn authenc_init<M: Operation>(
    mode: TEE_OperationMode,
    ht: &mut TeeFsHtree,
    mut ni: Option<&mut TeeFsHtreeNodeImage>,
    _payload_len: usize,
    root_hash: Option<&[u8; TEE_FS_HTREE_HASH_SIZE]>,
) -> Result<Cipher<M, Authenticated, CipherData>, TeeResultCode> {
    const ALG: TEE_ALG = TEE_FS_HTREE_AUTH_ENC_ALG;
    let mut ni_is_some = false;
    let mut aad_len = TEE_FS_HTREE_FEK_SIZE + TEE_FS_HTREE_IV_SIZE;

    // 可变引用指向 iv，避免 unsafe
    let iv_ref: &mut [u8; TEE_FS_HTREE_IV_SIZE] = if let Some(ref mut ni) = ni {
        ni_is_some = true;
        &mut ni.iv
    } else {
        aad_len += TEE_FS_HTREE_HASH_SIZE + size_of_val(&ht.data.head.counter);
        &mut ht.data.head.iv
    };

    if mode == TEE_OperationMode::TEE_MODE_ENCRYPT {
        crypto_rng_read(iv_ref)?;
    }

    let cipher = create_cipher::<M>(ALG, TEE_FS_HTREE_FEK_SIZE)?;
    let cipher_k = cipher
        .set_key_iv(&ht.data.fek, iv_ref)
        .map_err(|_| TEE_ERROR_GENERIC)?;

    let mut ad: Vec<u8> = Vec::with_capacity(aad_len);
    if ni_is_some {
        if let Some(hash) = root_hash {
            ad.extend_from_slice(hash);
        } else {
            ad.extend_from_slice(&ht.root.node.hash);
        }
        ad.extend_from_slice(bytemuck::bytes_of(&ht.data.head.counter));
    }

    ad.extend_from_slice(bytemuck::bytes_of(&ht.data.head.enc_fek));
    ad.extend_from_slice(iv_ref);

    let cipher_d = cipher_k.set_ad(ad.as_slice());

    cipher_d.map_err(|_| TEE_ERROR_GENERIC)
}
#[allow(dead_code)]
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
#[allow(dead_code)]
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
