// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.
//
// NOTE: Temporary file for crypto operations.
// TODO: Remove this file after the crypto module are implemented.

use alloc::boxed::Box;

use tee_raw_sys::{TEE_ALG_HMAC_SHA512, TEE_ERROR_BAD_PARAMETERS, TEE_OperationMode};

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
