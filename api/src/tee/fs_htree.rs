// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::boxed::Box;
use core::ptr::NonNull;

use super::utee_defines::TEE_SHA256_HASH_SIZE;

use tee_raw_sys::TEE_UUID;

pub const TEE_FS_HTREE_IV_SIZE: usize = 16;
pub const TEE_FS_HTREE_HASH_SIZE: usize = TEE_SHA256_HASH_SIZE;
pub const TEE_FS_HTREE_FEK_SIZE: usize = 16;
pub const TEE_FS_HTREE_TAG_SIZE: usize = 16;

// unsafe impl Zeroable for TeeFsHtreeNodeImage {}
// unsafe impl Pod for TeeFsHtreeNodeImage {}
#[repr(C)]
#[derive(Copy, Debug, Clone, Default)]
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
#[derive(Copy, Clone, Default, Debug)]
pub struct TeeFsHtreeImage {
    pub iv: [u8; TEE_FS_HTREE_IV_SIZE],
    pub tag: [u8; TEE_FS_HTREE_TAG_SIZE],
    pub enc_fek: [u8; TEE_FS_HTREE_FEK_SIZE],
    pub imeta: [u8; TEE_FS_HTREE_IMETA_SIZE],
    pub counter: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)] // Derive Clone for easy copying if needed
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
    pub left: Option<Box<HtreeNode>>,
    pub right: Option<Box<HtreeNode>>,
}

#[derive(Debug, Default)]
pub struct TeeFsHtree {
    pub root: HtreeNode,
    pub data: TeeFsHtreeData,
}
