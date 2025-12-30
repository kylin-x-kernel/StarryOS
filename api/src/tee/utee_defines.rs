// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

pub type TeeResultCode = u32;
pub type TEE_ALG = u32;
pub const HW_UNIQUE_KEY_LENGTH: usize = 16;

pub const TEE_MD5_HASH_SIZE: usize = 16;
pub const TEE_SHA1_HASH_SIZE: usize = 20;
pub const TEE_SHA224_HASH_SIZE: usize = 28;
pub const TEE_SHA256_HASH_SIZE: usize = 32;
pub const TEE_SM3_HASH_SIZE: usize = 32;
pub const TEE_SHA384_HASH_SIZE: usize = 48;
pub const TEE_SHA512_HASH_SIZE: usize = 64;
pub const TEE_MD5SHA1_HASH_SIZE: usize = TEE_MD5_HASH_SIZE + TEE_SHA1_HASH_SIZE;
pub const TEE_MAX_HASH_SIZE: usize = 64;

pub const TEE_AES_BLOCK_SIZE: usize = 16;
pub const TEE_DES_BLOCK_SIZE: usize = 8;
pub const TEE_SM4_BLOCK_SIZE: usize = 16;
