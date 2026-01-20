// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

// for source:
// 	- core/include/crypto/crypto.h
use core::marker::PhantomData;

use crate::tee::{
    TeeResult,
    crypto::crypto::{ecc_keypair, ecc_public_key},
};

pub trait crypto_ecc_keypair_ops {
    fn generate(&mut self, key_size_bits: usize) -> TeeResult<()>;
    fn sign(&mut self, algo: u32, msg: &[u8], sig: &mut [u8], sig_len: &mut usize)
    -> TeeResult<()>;
    fn shared_secret(
        &mut self,
        public_key: &mut ecc_public_key,
        secret: &mut [u8],
        secret_len: &mut usize,
    ) -> TeeResult<()>;
    fn decrypt(&mut self, src: &[u8], dst: &mut [u8], dst_len: &mut usize) -> TeeResult<()>;
}

/// traits for ecc keypair operations, using crypto_ecc_keypair_ops
pub trait crypto_ecc_keypair_ops_generate {
    fn generate(&mut self, key_size_bits: usize) -> TeeResult;
}

pub trait crypto_ecc_keypair_ops_sign {
    fn sign(&mut self, algo: u32, msg: &[u8], sig: &mut [u8], sig_len: &mut usize);
}

pub trait crypto_ecc_keypair_ops_sign_impl {
    fn sign_impl(key: &mut ecc_keypair, algo: u32, msg: &[u8], sig: &mut [u8], sig_len: &mut usize);
}

pub trait crypto_ecc_keypair_ops_shared_secret {
    fn shared_secret(
        &mut self,
        public_key: &mut ecc_public_key,
        secret: &mut [u8],
        secret_len: &mut usize,
    ) -> TeeResult<()>;
}

pub trait crypto_ecc_keypair_ops_decrypt {
    fn decrypt(&mut self, src: &[u8], dst: &mut [u8], dst_len: &mut usize) -> TeeResult<()>;
}

/// traits for ecc keypair abilities
pub trait EccKeyPairCanGenerate {}

pub trait EccKeyPairCanSign {}

pub trait EccKeyPairCanSharedSecret {}

pub trait EccKeyPairCanDecrypt {}

pub enum EccAlgoKeyPair {
    EccCom,
    Sm2Pke,
    Sm2Dsa,
    Sm2Kep,
}

pub struct EccComKeyPair;
pub struct Sm2PkeKeyPair;
pub struct Sm2DsaKeyPair;
pub struct Sm2KepKeyPair;

/// Ecc Common Key Pair Operations
/// - Generate
/// - Sign
/// - Shared Secret
impl EccKeyPairCanGenerate for EccComKeyPair {}
impl EccKeyPairCanSign for EccComKeyPair {}
impl EccKeyPairCanSharedSecret for EccComKeyPair {}

impl crypto_ecc_keypair_ops_sign_impl for EccComKeyPair {
    fn sign_impl(
        key: &mut ecc_keypair,
        algo: u32,
        msg: &[u8],
        sig: &mut [u8],
        sig_len: &mut usize,
    ) {
        todo!()
    }
}

/// Sm2 Dsa Key Pair Operations
/// - Generate
/// - Sign
impl EccKeyPairCanGenerate for Sm2DsaKeyPair {}
impl EccKeyPairCanSign for Sm2DsaKeyPair {}

impl crypto_ecc_keypair_ops_sign_impl for Sm2DsaKeyPair {
    fn sign_impl(
        key: &mut ecc_keypair,
        algo: u32,
        msg: &[u8],
        sig: &mut [u8],
        sig_len: &mut usize,
    ) {
        todo!()
    }
}

/// Sm2 Kep Key Pair Operations
/// - Generate
impl EccKeyPairCanGenerate for Sm2KepKeyPair {}

/// Sm2 Pke Key Pair Operations
/// - Generate
/// - Decrypt
impl EccKeyPairCanGenerate for Sm2PkeKeyPair {}

impl EccKeyPairCanDecrypt for Sm2PkeKeyPair {}

/// Ecc Key Pair
///
/// inner: ecc_keypair, the data for ecc keypair
/// _marker: PhantomData<A>, A implements the ecc keypair operations
pub struct EccKeypair<'a, A> {
    pub inner: &'a mut ecc_keypair,
    pub _marker: PhantomData<A>,
}

impl<'a, A> EccKeypair<'a, A> {
    /// constructor, pass a mutable reference of ecc_keypair
    pub fn new(inner: &'a mut ecc_keypair) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }
}

impl<A: EccKeyPairCanGenerate> crypto_ecc_keypair_ops_generate for EccKeypair<'_, A> {
    fn generate(&mut self, key_size_bits: usize) -> TeeResult {
        todo!()
    }
}

impl<A> crypto_ecc_keypair_ops_sign for EccKeypair<'_, A>
where
    A: EccKeyPairCanSign + crypto_ecc_keypair_ops_sign_impl,
{
    fn sign(&mut self, algo: u32, msg: &[u8], sig: &mut [u8], sig_len: &mut usize) {
        A::sign_impl(&mut self.inner, algo, msg, sig, sig_len);
    }
}

impl<A: EccKeyPairCanSharedSecret> crypto_ecc_keypair_ops_shared_secret for EccKeypair<'_, A> {
    fn shared_secret(
        &mut self,
        public_key: &mut ecc_public_key,
        secret: &mut [u8],
        secret_len: &mut usize,
    ) -> TeeResult<()> {
        todo!()
    }
}

impl<A: EccKeyPairCanDecrypt> crypto_ecc_keypair_ops_decrypt for EccKeypair<'_, A> {
    fn decrypt(&mut self, src: &[u8], dst: &mut [u8], dst_len: &mut usize) -> TeeResult<()> {
        todo!()
    }
}

pub trait crypto_ecc_public_ops_free {
    fn free(&mut self) -> TeeResult;
}

pub struct Sm2DsaPubKey;
pub struct Sm2PkePubKey;

pub trait EccPublicKeyCanFree {}

impl EccPublicKeyCanFree for Sm2DsaPubKey {}

impl EccPublicKeyCanFree for Sm2PkePubKey {}

pub struct EccPublicKey<A> {
    inner: ecc_public_key,
    _marker: PhantomData<A>,
}

impl<A: EccPublicKeyCanFree> crypto_ecc_public_ops_free for EccPublicKey<A> {
    fn free(&mut self) -> TeeResult {
        todo!()
    }
}
