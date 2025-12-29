// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use super::utee_defines::{TEE_SHA256_HASH_SIZE};

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
#[derive(Copy, Clone, Default)]
pub struct TeeFsHtreeImeta {
    pub meta: TeeFsHtreeMeta,
    pub max_node_id: u32,
    //pub _padding: [u8; 4],
}

pub const TEE_FS_HTREE_IMETA_SIZE: usize = core::mem::size_of::<TeeFsHtreeImeta>();
#[repr(C)]
#[derive(Copy, Clone, Default)]
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