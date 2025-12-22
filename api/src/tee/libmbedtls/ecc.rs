// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

// for source:
// 	- lib/libmbedtls/core/ecc.c

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
