// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.
//
// for source:
// 	- core/include/crypto/crypto.h
//  - core/crypto/crypto.c

use crate::tee::{
    TeeResult,
    libmbedtls::bignum::{BigNum, crypto_bignum_allocate},
    tee_svc_cryp::tee_crypto_ops,
};
use core::default::Default;
use tee_raw_sys::*;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ecc_public_key {
    pub x: BigNum,
    pub y: BigNum,
    curve: u32,
    // TODO: add ops
    //const struct crypto_ecc_public_ops *ops; /* Key Operations */
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ecc_keypair {
    pub d: BigNum,
    pub x: BigNum,
    pub y: BigNum,
    pub curve: u32,
    // TODO: add ops
    //const struct crypto_ecc_keypair_ops *ops; /* Key Operations */
}

impl Default for ecc_keypair {
    fn default() -> Self {
        ecc_keypair {
            d: BigNum::default(),
            x: BigNum::default(),
            y: BigNum::default(),
            curve: 0,
        }
    }
}

impl tee_crypto_ops for ecc_keypair {
    fn new(key_type: u32, key_size_bits: usize) -> TeeResult<Self> {
        match key_type {
            TEE_TYPE_SM2_DSA_PUBLIC_KEY
            | TEE_TYPE_SM2_PKE_PUBLIC_KEY
            | TEE_TYPE_SM2_KEP_PUBLIC_KEY => {
                return Err(TEE_ERROR_NOT_IMPLEMENTED);
            }
            _ => {}
        };

        Ok(ecc_keypair {
            d: crypto_bignum_allocate(key_size_bits)?,
            x: crypto_bignum_allocate(key_size_bits)?,
            y: crypto_bignum_allocate(key_size_bits)?,
            curve: 0,
        })
    }
}
