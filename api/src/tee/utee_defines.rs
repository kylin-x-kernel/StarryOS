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


/// Chaining mode constants for TEE (Trusted Execution Environment) API
/// These values represent different block cipher modes of operation
// Electronic Codebook mode without padding
pub const TEE_CHAIN_MODE_ECB_NOPAD: u32 = 0x0;
// Cipher Block Chaining mode without padding
pub const TEE_CHAIN_MODE_CBC_NOPAD: u32 = 0x1;
// Counter mode
pub const TEE_CHAIN_MODE_CTR: u32 = 0x2;
// Cipher Text Stealing mode
pub const TEE_CHAIN_MODE_CTS: u32 = 0x3;
// XEX-based Tweaked-codebook mode with ciphertext stealing
pub const TEE_CHAIN_MODE_XTS: u32 = 0x4;
// CBC MAC with PKCS#5 padding
pub const TEE_CHAIN_MODE_CBC_MAC_PKCS5: u32 = 0x5;
// Cipher-based MAC
pub const TEE_CHAIN_MODE_CMAC: u32 = 0x6;
// Counter with CBC-MAC mode
pub const TEE_CHAIN_MODE_CCM: u32 = 0x7;
// Galois/Counter Mode
pub const TEE_CHAIN_MODE_GCM: u32 = 0x8;
// Galois/Counter Mode
pub const TEE_CHAIN_MODE_PKCS1_PSS_MGF1: u32 = 0x9;
