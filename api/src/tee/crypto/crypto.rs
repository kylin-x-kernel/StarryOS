// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.
//
// for source:
// 	- core/include/crypto/crypto.h
//  - core/crypto/crypto.c

use alloc::boxed::Box;
use core::default::Default;

use tee_raw_sys::*;

use crate::tee::{
    TeeResult,
    crypto::crypto_impl::crypto_ecc_keypair_ops,
    libmbedtls::{
        bignum::{BigNum, crypto_bignum_allocate},
        ecc::{EcdOps, Sm2DsaOps, Sm2KepOps, Sm2PkeOps},
    },
    tee_obj::tee_obj_id_type,
    tee_svc_cryp::{CryptoAttrRef, tee_crypto_ops},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ecc_public_key {
    pub x: BigNum,
    pub y: BigNum,
    curve: u32,
    // ops: Box<dyn crypto_ecc_public_ops>,
}

impl Default for ecc_public_key {
    fn default() -> Self {
        ecc_public_key {
            x: BigNum::default(),
            y: BigNum::default(),
            curve: 0,
        }
    }
}

impl tee_crypto_ops for ecc_public_key {
    fn new(key_type: u32, key_size_bits: usize) -> TeeResult<Self> {
        match key_type {
            TEE_TYPE_SM2_DSA_PUBLIC_KEY
            | TEE_TYPE_SM2_PKE_PUBLIC_KEY
            | TEE_TYPE_SM2_KEP_PUBLIC_KEY => {
                return Err(TEE_ERROR_NOT_IMPLEMENTED);
            }
            _ => {}
        };

        Ok(ecc_public_key {
            x: crypto_bignum_allocate(key_size_bits)?,
            y: crypto_bignum_allocate(key_size_bits)?,
            curve: 0,
        })
    }

    fn get_attr_by_id(&mut self, attr_id: tee_obj_id_type) -> TeeResult<CryptoAttrRef<'_>> {
        match attr_id as u32 {
            TEE_ATTR_ECC_PUBLIC_VALUE_X => Ok(CryptoAttrRef::BigNum(&mut self.x)),
            TEE_ATTR_ECC_PUBLIC_VALUE_Y => Ok(CryptoAttrRef::BigNum(&mut self.y)),
            TEE_ATTR_ECC_CURVE => Ok(CryptoAttrRef::U32(&mut self.curve)),
            _ => Err(TEE_ERROR_ITEM_NOT_FOUND),
        }
    }
}

pub struct ecc_keypair {
    pub d: BigNum,
    pub x: BigNum,
    pub y: BigNum,
    pub curve: u32,
    // TODO: add ops
    pub ops: Box<dyn crypto_ecc_keypair_ops>,
}

impl Default for ecc_keypair {
    fn default() -> Self {
        ecc_keypair {
            d: BigNum::default(),
            x: BigNum::default(),
            y: BigNum::default(),
            curve: 0,
            ops: Box::new(EcdOps),
        }
    }
}

impl tee_crypto_ops for ecc_keypair {
    fn new(key_type: u32, key_size_bits: usize) -> TeeResult<Self> {
        let mut curve = 0;

        let ops: Box<dyn crypto_ecc_keypair_ops> = match key_type {
            TEE_TYPE_ECDSA_KEYPAIR | TEE_TYPE_ECDH_KEYPAIR => Box::new(EcdOps),
            TEE_TYPE_SM2_DSA_KEYPAIR => {
                curve = TEE_ECC_CURVE_SM2;
                Box::new(Sm2DsaOps)
            }
            TEE_TYPE_SM2_PKE_KEYPAIR => {
                curve = TEE_ECC_CURVE_SM2;
                Box::new(Sm2PkeOps)
            }
            TEE_TYPE_SM2_KEP_KEYPAIR => {
                curve = TEE_ECC_CURVE_SM2;
                Box::new(Sm2KepOps)
            }
            _ => return Err(TEE_ERROR_NOT_IMPLEMENTED),
        };

        Ok(ecc_keypair {
            d: crypto_bignum_allocate(key_size_bits)?,
            x: crypto_bignum_allocate(key_size_bits)?,
            y: crypto_bignum_allocate(key_size_bits)?,
            curve,
            ops,
        })
    }

    fn get_attr_by_id(&mut self, attr_id: tee_obj_id_type) -> TeeResult<CryptoAttrRef<'_>> {
        match attr_id as u32 {
            TEE_ATTR_ECC_PRIVATE_VALUE => Ok(CryptoAttrRef::BigNum(&mut self.d)),
            TEE_ATTR_ECC_PUBLIC_VALUE_X => Ok(CryptoAttrRef::BigNum(&mut self.x)),
            TEE_ATTR_ECC_PUBLIC_VALUE_Y => Ok(CryptoAttrRef::BigNum(&mut self.y)),
            TEE_ATTR_ECC_CURVE => Ok(CryptoAttrRef::U32(&mut self.curve)),
            _ => Err(TEE_ERROR_ITEM_NOT_FOUND),
        }
    }
}

impl PartialEq for ecc_keypair {
    fn eq(&self, other: &Self) -> bool {
        self.d == other.d && self.x == other.x && self.y == other.y && self.curve == other.curve
    }
}

impl Eq for ecc_keypair {}

// The crypto context used by the crypto_hash_*() functions
pub(crate) struct CryptoHashContext {
    pub ops: Option<&'static CryptoHashOps>,
}

// Constructor for CryptoHashCtx
pub(crate) struct CryptoHashOps {
    pub init: Option<fn(ctx: &mut CryptoHashContext) -> TeeResult>,
    pub update: Option<fn(ctx: &mut CryptoHashContext, data: &[u8]) -> TeeResult>,
    pub final_: Option<fn(ctx: &mut CryptoHashContext, digest: &mut [u8]) -> TeeResult>,
    pub free_ctx: Option<fn(ctx: &mut CryptoHashContext)>,
    pub copy_state: Option<fn(dst_ctx: &mut CryptoHashContext, src_ctx: &CryptoHashContext)>,
}

// defining hash operations for cryptographic hashing
pub(crate) trait CryptoHashCtx {
    // Initialize the hash context
    fn init(&mut self) -> TeeResult;

    // Update the hash context with data
    fn update(&mut self, data: &[u8]) -> TeeResult;

    // Finalize the hash computation and return the digest
    fn r#final(&mut self, digest: &mut [u8]) -> TeeResult;

    // Free the hash context resources
    fn free_ctx(self);

    // Copy the state from one context to another
    fn copy_state(&mut self, ctx: &dyn CryptoHashCtx);
}

// Helper function to get ops from context
fn hash_ops(ctx: &CryptoHashContext) -> &CryptoHashOps {
    ctx.ops.as_ref().expect("CryptoHashCtx ops is None")
}

pub(crate) fn crypto_hash_free_ctx(ctx: impl CryptoHashCtx) {
    ctx.free_ctx();
}

pub(crate) fn crypto_hash_copy_state(ctx: &mut dyn CryptoHashCtx, src_ctx: &dyn CryptoHashCtx) {
    ctx.copy_state(src_ctx);
}

pub(crate) fn crypto_hash_init(ctx: &mut dyn CryptoHashCtx) -> TeeResult {
    ctx.init()
}

pub(crate) fn crypto_hash_update(ctx: &mut dyn CryptoHashCtx, data: &[u8]) -> TeeResult {
    ctx.update(data)
}

pub(crate) fn crypto_hash_final(ctx: &mut dyn CryptoHashCtx, digest: &mut [u8]) -> TeeResult {
    // Err(TEE_ERROR_NOT_IMPLEMENTED)
    ctx.r#final(digest)
}

// Driver-based hash allocation (stub implementation)
pub(crate) fn drvcrypt_hash_alloc_ctx(algo: u32) -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

// Default hash algorithm allocation functions (stub implementations)
pub(crate) fn crypto_md5_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_sha1_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_sha224_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_sha256_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_sha384_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_sha512_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_sha3_224_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_sha3_256_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_sha3_384_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_sha3_512_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_shake128_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_shake256_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

pub(crate) fn crypto_sm3_alloc_ctx() -> TeeResult<Box<dyn CryptoHashCtx>> {
    Err(TEE_ERROR_NOT_IMPLEMENTED)
}

// defining mac operations for cryptographic hashing
pub(crate) trait CryptoMacCtx {
    // Initialize the hash context
    fn init(&mut self, key: &[u8]) -> TeeResult;

    // Update the hash context with data
    fn update(&mut self, data: &[u8]) -> TeeResult;

    // Finalize the hash computation and return the digest
    fn r#final(&mut self, digest: &mut [u8]) -> TeeResult;

    // Free the hash context resources
    fn free_ctx(self);

    // Copy the state from one context to another
    fn copy_state(&mut self, ctx: &dyn CryptoMacCtx);
}

pub(crate) fn crypto_mac_init(ctx: &mut dyn CryptoMacCtx, key: &[u8]) -> TeeResult {
    return ctx.init(key);
}
// Main hash context allocation function
pub(crate) fn crypto_hash_alloc_ctx(algo: u32) -> TeeResult<Box<dyn CryptoHashCtx>> {
    let mut res = TEE_ERROR_NOT_IMPLEMENTED;
    let mut c: Option<Box<dyn CryptoHashCtx>> = None;

    // Use default cryptographic implementation if no matching drvcrypt device
    match drvcrypt_hash_alloc_ctx(algo) {
        Ok(ctx) => {
            c = Some(ctx);
            res = TEE_SUCCESS;
        }
        Err(error) => {
            if error == TEE_ERROR_NOT_IMPLEMENTED {
                // Fallback to default implementations
                match algo {
                    TEE_ALG_MD5 => match crypto_md5_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHA1 => match crypto_sha1_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHA224 => match crypto_sha224_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHA256 => match crypto_sha256_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHA384 => match crypto_sha384_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHA512 => match crypto_sha512_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHA3_224 => match crypto_sha3_224_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHA3_256 => match crypto_sha3_256_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHA3_384 => match crypto_sha3_384_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHA3_512 => match crypto_sha3_512_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHAKE128 => match crypto_shake128_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SHAKE256 => match crypto_shake256_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    TEE_ALG_SM3 => match crypto_sm3_alloc_ctx() {
                        Ok(ctx) => {
                            c = Some(ctx);
                            res = TEE_SUCCESS;
                        }
                        Err(error) => res = error,
                    },
                    _ => {
                        // Do nothing, res remains TEE_ERROR_NOT_IMPLEMENTED
                    }
                }
            } else {
                res = error;
            }
        }
    }

    if res == TEE_SUCCESS {
        if let Some(ctx) = c {
            Ok(ctx)
        } else {
            Err(TEE_ERROR_NOT_IMPLEMENTED)
        }
    } else {
        Err(res)
    }
}