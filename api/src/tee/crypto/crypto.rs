// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.
//
// for source:
// 	- core/include/crypto/crypto.h
//  - core/crypto/crypto.c

use alloc::{boxed::Box, format, sync::Arc};
use core::{default::Default, fmt, fmt::Debug};

use mbedtls::{
    cipher::raw::{Cipher, CipherId, CipherMode, CipherType, Operation},
    hash::{Hmac, Md, Type as MdType},
    pk::Type as PkType,
};
use mbedtls_sys_auto::mpi_write_binary;
use spin::Mutex;
use tee_raw_sys::*;

use crate::tee::{
    TEE_ALG_SM4_XTS, TeeResult,
    crypto::crypto_impl::{
        EccAlgoKeyPair, EccComKeyPair, EccKeypair, Sm2DsaKeyPair, Sm2KepKeyPair, Sm2PkeKeyPair,
        crypto_ecc_keypair_ops, crypto_ecc_keypair_ops_generate,
    },
    libmbedtls::{
        bignum::{BigNum, crypto_bignum_allocate},
        ecc::{EcdOps, Sm2DsaOps, Sm2KepOps, Sm2PkeOps},
    },
    tee_obj::tee_obj_id_type,
    tee_svc_cryp::{CryptoAttrRef, tee_cryp_obj_secret_wrapper, tee_crypto_ops},
    tee_svc_cryp2::{CrypCtx, CrypState, TeeCrypState},
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
    // pub ops: Box<dyn crypto_ecc_keypair_ops>,
}

impl Default for ecc_keypair {
    fn default() -> Self {
        ecc_keypair {
            d: BigNum::default(),
            x: BigNum::default(),
            y: BigNum::default(),
            curve: 0,
            // ops: Box::new(EcdOps),
        }
    }
}

impl Debug for ecc_keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ecc_keypair")
            .field("d", &self.d)
            .field("x", &self.x)
            .field("y", &self.y)
            .field("curve", &format!("{:#010X?}", self.curve))
            .finish()
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
            // ops,
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

pub struct rsa_keypair {
    pub e: BigNum, // Public exponent
    pub d: BigNum, // Private exponent
    pub n: BigNum, // Modulus

    // Optional CRT parameters (all NULL if unused)
    pub p: BigNum, // N = pq
    pub q: BigNum,
    pub qp: BigNum, // 1/q mod p
    pub dp: BigNum, // d mod (p-1)
    pub dq: BigNum, // d mod (q-1)
}

impl Debug for rsa_keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("rsa_keypair")
            .field("e", &self.e)
            .field("d", &self.d)
            .field("n", &self.n)
            .field("p", &self.p)
            .field("q", &self.q)
            .field("qp", &self.qp)
            .field("dp", &self.dp)
            .field("dq", &self.dq)
            .finish()
    }
}

impl tee_crypto_ops for rsa_keypair {
    fn new(key_type: u32, key_size_bits: usize) -> TeeResult<Self> {
        Ok(rsa_keypair {
            e: crypto_bignum_allocate(key_size_bits)?,
            d: crypto_bignum_allocate(key_size_bits)?,
            n: crypto_bignum_allocate(key_size_bits)?,
            p: BigNum::default(),
            q: BigNum::default(),
            qp: BigNum::default(),
            dp: BigNum::default(),
            dq: BigNum::default(),
        })
    }

    fn get_attr_by_id(&mut self, attr_id: tee_obj_id_type) -> TeeResult<CryptoAttrRef<'_>> {
        match attr_id as u32 {
            TEE_ATTR_RSA_MODULUS => Ok(CryptoAttrRef::BigNum(&mut self.n)),
            TEE_ATTR_RSA_PUBLIC_EXPONENT => Ok(CryptoAttrRef::BigNum(&mut self.e)),
            TEE_ATTR_RSA_PRIVATE_EXPONENT => Ok(CryptoAttrRef::BigNum(&mut self.d)),
            TEE_ATTR_RSA_PRIME1 => Ok(CryptoAttrRef::BigNum(&mut self.p)),
            TEE_ATTR_RSA_PRIME2 => Ok(CryptoAttrRef::BigNum(&mut self.q)),
            TEE_ATTR_RSA_EXPONENT1 => Ok(CryptoAttrRef::BigNum(&mut self.dp)),
            TEE_ATTR_RSA_EXPONENT2 => Ok(CryptoAttrRef::BigNum(&mut self.dq)),
            TEE_ATTR_RSA_COEFFICIENT => Ok(CryptoAttrRef::BigNum(&mut self.qp)),
            _ => Err(TEE_ERROR_ITEM_NOT_FOUND),
        }
    }
}
pub fn crypto_acipher_gen_ecc_key(
    key: &mut ecc_keypair,
    key_size_bits: usize,
    object_type: u32,
) -> TeeResult {
    let mut key: Box<dyn crypto_ecc_keypair_ops_generate> = match object_type {
        TEE_TYPE_ECDSA_KEYPAIR | TEE_TYPE_ECDH_KEYPAIR => {
            Box::new(EccKeypair::<EccComKeyPair>::new(key))
        }
        TEE_TYPE_SM2_PKE_KEYPAIR => Box::new(EccKeypair::<Sm2PkeKeyPair>::new(key)),
        TEE_TYPE_SM2_DSA_KEYPAIR => Box::new(EccKeypair::<Sm2DsaKeyPair>::new(key)),
        TEE_TYPE_SM2_KEP_KEYPAIR => Box::new(EccKeypair::<Sm2KepKeyPair>::new(key)),
        _ => return Err(TEE_ERROR_NOT_IMPLEMENTED),
    };
    key.generate(key_size_bits)
}

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

pub(crate) fn crypto_hash_init(cs: Arc<Mutex<TeeCrypState>>) -> TeeResult {
    let mut cs_guard = cs.lock();
    let algo = cs_guard.algo;
    let mut md_type: MdType = MdType::None;
    match algo {
        TEE_ALG_MD5 => md_type = MdType::Md5,
        TEE_ALG_SHA1 => md_type = MdType::Sha1,
        TEE_ALG_SHA224 => md_type = MdType::Sha224,
        TEE_ALG_SHA256 => md_type = MdType::Sha256,
        TEE_ALG_SHA384 => md_type = MdType::Sha384,
        TEE_ALG_SHA512 => md_type = MdType::Sha512,
        TEE_ALG_SM3 => md_type = MdType::SM3,
        _ => return Err(TEE_ERROR_NOT_IMPLEMENTED),
    }
    if let Ok(md) = Md::new(md_type) {
        cs_guard.ctx = CrypCtx::HashCtx(md);
        cs_guard.state = CrypState::Initialized;
        Ok(())
    } else {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }
}

pub(crate) fn crypto_hash_update(cs: Arc<Mutex<TeeCrypState>>, data: &[u8]) -> TeeResult {
    let mut cs_guard = cs.lock();

    match &mut cs_guard.ctx {
        CrypCtx::HashCtx(md) => md.update(data).map_err(|_| TEE_ERROR_BAD_PARAMETERS),
        _ => Err(TEE_ERROR_BAD_PARAMETERS),
    }
}

pub(crate) fn crypto_hash_final(cs: Arc<Mutex<TeeCrypState>>, hash: &mut [u8]) -> TeeResult<usize> {
    let mut cs_guard = cs.lock();

    let ctx = core::mem::replace(&mut cs_guard.ctx, CrypCtx::Others);

    if let CrypCtx::HashCtx(md) = ctx {
        md.finish(hash).map_err(|_| TEE_ERROR_BAD_PARAMETERS)
    } else {
        Err(TEE_ERROR_BAD_PARAMETERS)
    }
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

pub(crate) fn crypto_mac_init(cs: Arc<Mutex<TeeCrypState>>, key: &[u8]) -> TeeResult {
    let mut cs_guard = cs.lock();
    let algo = cs_guard.algo;
    let mut md_type: MdType = MdType::None;

    match algo {
        TEE_ALG_HMAC_MD5 => md_type = MdType::Md5,
        TEE_ALG_HMAC_SHA1 => md_type = MdType::Sha1,
        TEE_ALG_HMAC_SHA224 => md_type = MdType::Sha224,
        TEE_ALG_HMAC_SHA256 => md_type = MdType::Sha256,
        TEE_ALG_HMAC_SHA384 => md_type = MdType::Sha384,
        TEE_ALG_HMAC_SHA512 => md_type = MdType::Sha512,
        TEE_ALG_HMAC_SM3 => md_type = MdType::SM3,
        _ => return Err(TEE_ERROR_NOT_IMPLEMENTED),
    }

    if let Ok(hmac) = Hmac::new(md_type, key) {
        cs_guard.ctx = CrypCtx::HmacCtx(hmac);
        cs_guard.state = CrypState::Initialized;
        Ok(())
    } else {
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }
}

// Crypto MAC update
pub(crate) fn crypto_mac_update(cs: Arc<Mutex<TeeCrypState>>, data: &[u8]) -> TeeResult {
    let mut guard = cs.lock();

    match &mut guard.ctx {
        CrypCtx::HmacCtx(hmac) => hmac.update(data).map_err(|_| TEE_ERROR_BAD_PARAMETERS),
        _ => Err(TEE_ERROR_BAD_PARAMETERS),
    }
}

// Crypto MAC finalization
pub(crate) fn crypto_mac_final(cs: Arc<Mutex<TeeCrypState>>, hash: &mut [u8]) -> TeeResult<usize> {
    let mut cs_guard = cs.lock();

    let ctx = core::mem::replace(&mut cs_guard.ctx, CrypCtx::Others);

    if let CrypCtx::HmacCtx(hmac) = ctx {
        hmac.finish(hash).map_err(|_| TEE_ERROR_BAD_PARAMETERS)
    } else {
        Err(TEE_ERROR_BAD_PARAMETERS)
    }
}

// Crypto MAC free
pub(crate) fn crypto_mac_free(ctx: impl CryptoMacCtx) {
    // Err(TEE_ERROR_NOT_IMPLEMENTED)
    ctx.free_ctx();
}

//
pub(crate) fn crypto_mac_copy_state(ctx: &mut dyn CryptoMacCtx, src_ctx: &dyn CryptoMacCtx) {
    // Err(TEE_ERROR_NOT_IMPLEMENTED)
    ctx.copy_state(src_ctx)
}

pub(crate) fn crypto_cipher_init(
    cs: Arc<Mutex<TeeCrypState>>,
    key: &[u8],
    iv: Option<&[u8]>,
) -> TeeResult {
    let mut cs_guard = cs.lock();
    let algo = cs_guard.algo;
    let mode = cs_guard.mode;

    let mut cipher_id = CipherId::None;
    let mut cipher_mode = CipherMode::None;
    let mut cipher_op = Operation::None;

    match mode {
        TEE_OperationMode::TEE_MODE_ENCRYPT => cipher_op = Operation::Encrypt,
        TEE_OperationMode::TEE_MODE_DECRYPT => cipher_op = Operation::Decrypt,
        _ => return Err(TEE_ERROR_BAD_PARAMETERS),
    }

    match algo {
        TEE_ALG_AES_ECB_NOPAD => {
            cipher_id = CipherId::Aes;
            cipher_mode = CipherMode::ECB;
        }
        TEE_ALG_AES_CBC_NOPAD => {
            cipher_id = CipherId::Aes;
            cipher_mode = CipherMode::CBC;
        }
        TEE_ALG_AES_CTR => {
            cipher_id = CipherId::Aes;
            cipher_mode = CipherMode::CTR;
        }
        TEE_ALG_AES_XTS => {
            cipher_id = CipherId::Aes;
            cipher_mode = CipherMode::XTS;
        }
        TEE_ALG_DES_ECB_NOPAD => {
            cipher_id = CipherId::Des;
            cipher_mode = CipherMode::ECB;
        }
        TEE_ALG_DES3_ECB_NOPAD => {
            cipher_id = CipherId::Des3;
            cipher_mode = CipherMode::ECB;
        }
        TEE_ALG_DES_CBC_NOPAD => {
            cipher_id = CipherId::Des;
            cipher_mode = CipherMode::CBC;
        }
        TEE_ALG_DES3_CBC_NOPAD => {
            cipher_id = CipherId::Des3;
            cipher_mode = CipherMode::CBC;
        }
        TEE_ALG_SM4_ECB_NOPAD => {
            cipher_id = CipherId::SM4;
            cipher_mode = CipherMode::ECB;
        }
        TEE_ALG_SM4_CBC_NOPAD => {
            cipher_id = CipherId::SM4;
            cipher_mode = CipherMode::CBC;
        }
        TEE_ALG_SM4_CTR => {
            cipher_id = CipherId::SM4;
            cipher_mode = CipherMode::CTR;
        }
        TEE_ALG_SM4_XTS => {
            cipher_id = CipherId::SM4;
            cipher_mode = CipherMode::XTS;
        }
        _ => return Err(TEE_ERROR_NOT_IMPLEMENTED),
    }

    if let Ok(mut cipher) = Cipher::setup(cipher_id, cipher_mode, (key.len() * 8) as _) {
        cipher
            .set_key(cipher_op, key)
            .map_err(|_| TEE_ERROR_BAD_PARAMETERS);
        if let Some(iv) = iv {
            cipher.set_iv(iv).map_err(|_| TEE_ERROR_BAD_PARAMETERS);
        }
        cipher.reset().map_err(|_| TEE_ERROR_BAD_PARAMETERS);
        cs_guard.state = CrypState::Initialized;
        Ok(())
    } else {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }
}

pub(crate) fn crypto_cipher_update(
    cs: Arc<Mutex<TeeCrypState>>,
    input: &[u8],
    output: &mut [u8],
) -> TeeResult<usize> {
    let mut cs_guard = cs.lock();
    if let CrypCtx::CipherCtx(cipher) = &mut cs_guard.ctx {
        cipher
            .update(input, output)
            .map_err(|_| TEE_ERROR_BAD_PARAMETERS)
    } else {
        Err(TEE_ERROR_BAD_PARAMETERS)
    }
}

pub(crate) fn crypto_cipher_final(
    cs: Arc<Mutex<TeeCrypState>>,
    output: &mut [u8],
) -> TeeResult<usize> {
    let mut cs_guard = cs.lock();
    if let CrypCtx::CipherCtx(cipher) = &mut cs_guard.ctx {
        cipher.finish(output).map_err(|_| TEE_ERROR_BAD_PARAMETERS)
    } else {
        Err(TEE_ERROR_BAD_PARAMETERS)
    }
}
