// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.
//
// NOTE: Temporary file for crypto operations.
// TODO: Remove this file after the crypto module are implemented.

use alloc::boxed::Box;

use mbedtls::hash::{Md, Type};
use tee_raw_sys::{
    TEE_ALG_HMAC_SHA256, TEE_ALG_HMAC_SHA512, TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_BAD_STATE,
    TEE_ERROR_NOT_IMPLEMENTED, TEE_OperationMode,
};

use crate::tee::{TeeResult, crypto_temp::aes_cbc::MbedAesCbcCtx, utee_defines::TEE_ALG};

pub trait HashOps {
    fn init(&mut self, key: &[u8]) -> TeeResult;
    fn update(&mut self, data: &[u8]) -> TeeResult;
    fn finalize(&mut self, digest: &mut [u8]) -> TeeResult;
    fn free_ctx(&mut self);
    fn copy_state(&self, dst_ctx: &mut dyn HashOps) -> TeeResult;
}

// 添加MacAlgorithmTrait trait
pub trait MacAlgorithmTrait {
    type Context: HashOps + 'static;

    fn alloc_hash() -> Result<Self::Context, TeeResult>;
}

pub trait HashAlgorithm {
    type Ops: HashOps + 'static;

    fn get_ops() -> &'static Self::Ops;
    // fn get_md_type() -> mbedtls_md_type_t;
}

pub struct MbedCtx;

// 具体的MAC算法实现
// HMAC-SHA256
#[allow(dead_code)]
pub struct HmacSha256 {
    inner: MbedCtx,
}
#[allow(dead_code)]
pub struct HmacSha512 {
    inner: MbedCtx,
}

impl HashOps for HmacSha256 {
    fn init(&mut self, _key: &[u8]) -> TeeResult {
        debug!("HashOps with HmacSha256");
        Ok(())
    }

    fn update(&mut self, _data: &[u8]) -> TeeResult {
        Ok(())
    }

    fn finalize(&mut self, _digest: &mut [u8]) -> TeeResult {
        Ok(())
    }

    fn free_ctx(&mut self) {
        // 这里应该释放资源，但返回类型是()
    }

    fn copy_state(&self, _dst_ctx: &mut dyn HashOps) -> TeeResult {
        Ok(())
    }
}

// 为HmacSha256实现MacAlgorithmTrait
impl MacAlgorithmTrait for HmacSha256 {
    type Context = HmacSha256;

    fn alloc_hash() -> Result<Self::Context, TeeResult> {
        Ok(HmacSha256 { inner: MbedCtx })
    }
}

impl HashOps for HmacSha512 {
    fn init(&mut self, _key: &[u8]) -> TeeResult {
        Ok(())
    }

    fn update(&mut self, _data: &[u8]) -> TeeResult {
        Ok(())
    }

    fn finalize(&mut self, _digest: &mut [u8]) -> TeeResult {
        Ok(())
    }

    fn free_ctx(&mut self) {
        // 这里应该释放资源，但返回类型是()
    }

    fn copy_state(&self, _dst_ctx: &mut dyn HashOps) -> TeeResult {
        Ok(())
    }
}

// 为HmacSha512实现MacAlgorithmTrait
impl MacAlgorithmTrait for HmacSha512 {
    type Context = HmacSha512;

    fn alloc_hash() -> Result<Self::Context, TeeResult> {
        Ok(HmacSha512 { inner: MbedCtx })
    }
}

// 工厂方法：根据hash_id生成不同的HMAC实例
pub fn crypto_mac_alloc_ctx(algorithm: TEE_ALG) -> Result<Box<dyn HashOps>, TeeResult> {
    match algorithm {
        TEE_ALG_HMAC_SHA256 => {
            let ctx: HmacSha256 = HmacSha256::alloc_hash()?;
            Ok(Box::new(ctx))
        }
        TEE_ALG_HMAC_SHA512 => {
            let ctx = HmacSha512::alloc_hash()?;
            Ok(Box::new(ctx))
        }
        _ => Err(Err(TEE_ERROR_BAD_PARAMETERS)),
    }
}

pub trait CryptoCipherOps {
    fn init(
        &mut self,
        mode: TEE_OperationMode,
        key1: Option<&[u8]>,
        key2: Option<&[u8]>,
        iv: Option<&[u8]>,
    ) -> TeeResult;
    fn update(
        &mut self,
        last_block: bool,
        data: Option<&[u8]>,
        dst: Option<&mut [u8]>,
    ) -> TeeResult;
    fn finalize(&mut self);
    fn free_ctx(&mut self);
    fn copy_state(&self, dst_ctx: &mut MbedAesCbcCtx);
}

pub trait CryptoCipherCtx {
    type Context;

    fn alloc_cipher_ctx() -> Result<Box<Self::Context>, TeeResult>;
}
pub trait CryptoHashOps {
    fn init(&mut self) -> TeeResult;
    fn update(&mut self, data: Option<&[u8]>) -> TeeResult;
    fn finalize(&mut self, digest: Option<&mut [u8]>) -> TeeResult;
    fn free_ctx(&mut self);
    fn copy_state(&self, dst_ctx: &mut Self);
}

// pub fn crypto_mac_alloc_ctx<A: MacAlgorithmTrait>() -> Result<A::Context, TeeResult> {
//     A::alloc_ctx()
// }

/// Convert TEE_ALG to mbedtls::hash::Type
/// This is a helper function instead of TryFrom implementation due to Rust's orphan rule
fn tee_alg_to_hash_type(value: TEE_ALG) -> Result<Type, u32> {
        match value {
            TEE_ALG_MD5 => Ok(Type::Md5),
            TEE_ALG_SHA256 => Ok(Type::Sha256),
            TEE_ALG_SHA512 => Ok(Type::Sha512),
            _ => Err(TEE_ERROR_NOT_IMPLEMENTED),
    }
}

pub fn crypto_cipher_alloc_ctx(algo: TEE_ALG) -> Result<Box<dyn CryptoCipherOps>, TeeResult> {
    match algo {
        TEE_ALG_AES_ECB_NOPAD => {
            let ctx: MbedAesCbcCtx = *MbedAesCbcCtx::alloc_cipher_ctx()?;
            Ok(Box::new(ctx))
        }
        _ => Err(Err(TEE_ERROR_NOT_IMPLEMENTED)),
    }
}

pub fn crypto_hash_free_ctx<H>(ctx: &mut H)
where
    H: CryptoHashOps,
{
    ctx.free_ctx();
}

// pub fn crypto_hash_copy_state<H>(src: &H, dst: &mut H)
// where
//     H: CryptoHashOps,
// {
//     src.copy_state(dst)
// }

// pub fn crypto_hash_alloc_ctx(t : Type) -> Result<Md, TeeResultCode> {
//     let mut md = Md::new(t).map_err(|e| TeeResultCode::ErrorBadState)?;
//
//     Ok(md)
// }

pub fn crypto_hash_alloc_ctx(alg: TEE_ALG) -> TeeResult<Md> {
    let t = tee_alg_to_hash_type(alg)?;
    let md = Md::new(t).map_err(|_| TEE_ERROR_BAD_STATE)?;

    Ok(md)
}

pub fn crypto_hash_init(_md: &mut Md) -> TeeResult {
    // initialized in Md.new
    Ok(())
}

pub fn crypto_hash_update(md: &mut Md, data: &[u8]) -> TeeResult {
    md.update(data).map_err(|_| TEE_ERROR_BAD_STATE)?;
    Ok(())
}

pub fn crypto_hash_final(md: Md, digest: &mut [u8]) -> TeeResult {
    md.finish(digest).map_err(|_| TEE_ERROR_BAD_STATE)?;
    Ok(())
}
