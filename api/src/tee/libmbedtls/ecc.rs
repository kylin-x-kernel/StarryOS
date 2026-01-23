// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

// for source:
// 	- lib/libmbedtls/core/ecc.c

use mbedtls::pk::EcGroupId;
use mbedtls_sys_auto::*;
use tee_raw_sys::{
    TEE_ALG_DSA_SHA3_224, TEE_ALG_DSA_SHA3_256, TEE_ALG_DSA_SHA3_384, TEE_ALG_DSA_SHA3_512,
    TEE_ALG_RSAES_PKCS1_OAEP_MGF1_SHA3_224, TEE_ALG_RSAES_PKCS1_OAEP_MGF1_SHA3_256,
    TEE_ALG_RSAES_PKCS1_OAEP_MGF1_SHA3_384, TEE_ALG_RSAES_PKCS1_OAEP_MGF1_SHA3_512,
    TEE_ALG_SM2_DSA_SM3, TEE_ALG_SM2_KEP, TEE_ALG_SM2_PKE, TEE_ECC_CURVE_NIST_P192,
    TEE_ECC_CURVE_NIST_P224, TEE_ECC_CURVE_NIST_P256, TEE_ECC_CURVE_NIST_P384,
    TEE_ECC_CURVE_NIST_P521, TEE_ECC_CURVE_SM2, TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_NOT_SUPPORTED,
};

use crate::tee::{
    TeeResult,
    crypto::{
        crypto::{ecc_keypair, ecc_public_key},
        crypto_impl::crypto_ecc_keypair_ops,
    },
};
/// Elliptic Curve Digital for ecdsa and ecdh
pub struct EcdOps;

impl crypto_ecc_keypair_ops for EcdOps {
    fn generate(&mut self, key_size_bits: usize) -> TeeResult<()> {
        todo!()
    }

    fn sign(
        &mut self,
        algo: u32,
        msg: &[u8],
        sig: &mut [u8],
        sig_len: &mut usize,
    ) -> TeeResult<()> {
        todo!()
    }

    fn shared_secret(
        &mut self,
        public_key: &mut ecc_public_key,
        secret: &mut [u8],
        secret_len: &mut usize,
    ) -> TeeResult<()> {
        todo!()
    }

    fn decrypt(&mut self, src: &[u8], dst: &mut [u8], dst_len: &mut usize) -> TeeResult<()> {
        todo!()
    }
}

/// Elliptic Curve Digital Signature Algorithm for sm2 dsa
pub struct Sm2DsaOps;

impl crypto_ecc_keypair_ops for Sm2DsaOps {
    fn generate(&mut self, key_size_bits: usize) -> TeeResult<()> {
        todo!()
    }

    fn sign(
        &mut self,
        algo: u32,
        msg: &[u8],
        sig: &mut [u8],
        sig_len: &mut usize,
    ) -> TeeResult<()> {
        todo!()
    }

    fn shared_secret(
        &mut self,
        public_key: &mut ecc_public_key,
        secret: &mut [u8],
        secret_len: &mut usize,
    ) -> TeeResult<()> {
        todo!()
    }

    fn decrypt(&mut self, src: &[u8], dst: &mut [u8], dst_len: &mut usize) -> TeeResult<()> {
        todo!()
    }
}

/// Elliptic Key Exchange Protocol for sm2
pub struct Sm2KepOps;

impl crypto_ecc_keypair_ops for Sm2KepOps {
    fn generate(&mut self, key_size_bits: usize) -> TeeResult<()> {
        todo!()
    }

    fn sign(
        &mut self,
        algo: u32,
        msg: &[u8],
        sig: &mut [u8],
        sig_len: &mut usize,
    ) -> TeeResult<()> {
        todo!()
    }

    fn shared_secret(
        &mut self,
        public_key: &mut ecc_public_key,
        secret: &mut [u8],
        secret_len: &mut usize,
    ) -> TeeResult<()> {
        todo!()
    }

    fn decrypt(&mut self, src: &[u8], dst: &mut [u8], dst_len: &mut usize) -> TeeResult<()> {
        todo!()
    }
}

/// Elliptic Public Key Encryption for sm2
pub struct Sm2PkeOps;

impl crypto_ecc_keypair_ops for Sm2PkeOps {
    fn generate(&mut self, key_size_bits: usize) -> TeeResult<()> {
        todo!()
    }

    fn sign(
        &mut self,
        algo: u32,
        msg: &[u8],
        sig: &mut [u8],
        sig_len: &mut usize,
    ) -> TeeResult<()> {
        todo!()
    }

    fn shared_secret(
        &mut self,
        public_key: &mut ecc_public_key,
        secret: &mut [u8],
        secret_len: &mut usize,
    ) -> TeeResult<()> {
        todo!()
    }

    fn decrypt(&mut self, src: &[u8], dst: &mut [u8], dst_len: &mut usize) -> TeeResult<()> {
        todo!()
    }
}

pub fn ecc_get_keysize(
    curve: u32,
    algo: u32,
    key_size_bytes: &mut usize,
    key_size_bits: &mut usize,
) -> TeeResult<()> {
    match curve {
        TEE_ECC_CURVE_NIST_P192 => {
            *key_size_bits = 192;
            *key_size_bytes = 24;
        }
        TEE_ECC_CURVE_NIST_P224 => {
            *key_size_bits = 224;
            *key_size_bytes = 28;
        }
        TEE_ECC_CURVE_NIST_P256 => {
            *key_size_bits = 256;
            *key_size_bytes = 32;
        }
        TEE_ECC_CURVE_NIST_P384 => {
            *key_size_bits = 384;
            *key_size_bytes = 48;
        }
        TEE_ECC_CURVE_NIST_P521 => {
            *key_size_bits = 521;
            *key_size_bytes = 66;
        }
        TEE_ECC_CURVE_SM2 => {
            *key_size_bits = 256;
            *key_size_bytes = 32;
            if algo != 0
                && algo != TEE_ALG_SM2_DSA_SM3
                && algo != TEE_ALG_SM2_KEP
                && algo != TEE_ALG_SM2_PKE
            {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        }
        _ => {
            *key_size_bits = 0;
            *key_size_bytes = 0;
            return Err(TEE_ERROR_NOT_SUPPORTED);
        }
    }
    Ok(())
}

pub fn curve_to_group_id(curve: u32) -> EcGroupId {
    match curve {
        TEE_ECC_CURVE_NIST_P192 => EcGroupId::SecP192R1,
        TEE_ECC_CURVE_NIST_P224 => EcGroupId::SecP224R1,
        TEE_ECC_CURVE_NIST_P256 => EcGroupId::SecP256R1,
        TEE_ECC_CURVE_NIST_P384 => EcGroupId::SecP384R1,
        TEE_ECC_CURVE_NIST_P521 => EcGroupId::SecP521R1,
        TEE_ECC_CURVE_SM2 => EcGroupId::SM2P256R1,
        _ => EcGroupId::None,
    }
}
