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

    fn get_attr_by_id(&mut self, attr_id: tee_obj_id_type) -> TeeResult<CryptoAttrRef> {
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

    fn get_attr_by_id(&mut self, attr_id: tee_obj_id_type) -> TeeResult<CryptoAttrRef> {
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
pub(crate) struct CryptoHashCtx {
    pub ops: Option<&'static CryptoHashOps>,
}


pub(crate) struct CryptoHashOps {
    pub init: Option<fn(ctx: &mut CryptoHashCtx) -> TeeResult>,
    pub update: Option<fn(ctx: &mut CryptoHashCtx, data: &[u8]) -> TeeResult>,
    pub final_: Option<fn(ctx: &mut CryptoHashCtx, digest: &mut [u8]) -> TeeResult>,
    pub free_ctx: Option<fn(ctx: &mut CryptoHashCtx)>,
    pub copy_state: Option<fn(dst_ctx: &mut CryptoHashCtx, src_ctx: &CryptoHashCtx)>,
}

// Constructor for CryptoHashCtx
impl CryptoHashCtx {
    pub fn new(ops: &'static CryptoHashOps) -> Self {
        CryptoHashCtx {
            ops: Some(ops),
        }
    }

    pub fn empty() -> Self {
        CryptoHashCtx {
            ops: None,
        }
    }
}

// Helper function to get ops from context
fn hash_ops(ctx: &CryptoHashCtx) -> &CryptoHashOps {
    ctx.ops.as_ref().expect("CryptoHashCtx ops is None")
}

pub(crate) fn crypto_hash_free_ctx(ctx: &mut CryptoHashCtx) {
    if let Some(free_fn) = hash_ops(ctx).free_ctx {
        free_fn(ctx);
    }
}

pub(crate) fn crypto_hash_copy_state(dst_ctx: &mut CryptoHashCtx, src_ctx: &CryptoHashCtx) {
    if let Some(copy_fn) = hash_ops(dst_ctx).copy_state {
        copy_fn(dst_ctx, src_ctx);
    }
}

pub(crate) fn crypto_hash_init(ctx: &mut CryptoHashCtx) -> TeeResult {
    if let Some(init_fn) = hash_ops(ctx).init {
        init_fn(ctx)
    } else {
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }
}

pub(crate) fn crypto_hash_update(ctx: &mut CryptoHashCtx, data: &[u8]) -> TeeResult {
    if let Some(update_fn) = hash_ops(ctx).update {
        update_fn(ctx, data)
    } else {
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }
}

pub(crate) fn crypto_hash_final(ctx: &mut CryptoHashCtx, digest: &mut [u8]) -> TeeResult {
    if let Some(final_fn) = hash_ops(ctx).final_ {
        final_fn(ctx, digest)
    } else {
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }
}
