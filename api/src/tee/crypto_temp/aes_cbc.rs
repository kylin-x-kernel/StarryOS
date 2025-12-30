// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::boxed::Box;

use tee_raw_sys::{TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_BAD_STATE, TEE_OperationMode};

use super::crypto_temp::{CryptoCipherCtx, CryptoCipherOps};
use crate::tee::{TeeResult, common::array, utee_defines::TEE_AES_BLOCK_SIZE};

// aes_context is defined in  rust-mbedtls
// use mbedtls::aes_context;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct aes_context;
pub fn aes_init(_ctx: *mut aes_context) {
    // TODO: 实现 AES 初始化
    // 避免未实现的功能被调用暂时返回，
}
pub fn aes_setkey_enc(_ctx: *mut aes_context, _key: *const u8, _keybits: u32) -> i32 {
    0
}
pub fn aes_crypt_cbc(
    _ctx: *mut aes_context,
    _mode: i32,
    _length: usize,
    _iv: *mut u8,
    _input: *const u8,
    _output: *mut u8,
) -> i32 {
    0
}

#[allow(dead_code)]
pub fn aes_free(_ctx: *mut aes_context) {}

pub const AES_ENCRYPT: i32 = 1;
pub const AES_DECRYPT: i32 = 0;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MbedAesCbcCtx {
    // inner: CryptoCipherCtx,
    mbed_mode: i32,
    aes_ctx: aes_context,
    iv: [u8; TEE_AES_BLOCK_SIZE],
}

fn mbed_aes_cbc_init(
    ctx: &mut MbedAesCbcCtx,
    mode: TEE_OperationMode,
    key1: Option<&[u8]>,
    key2: Option<&[u8]>,
    iv: Option<&[u8]>,
) -> TeeResult {
    let (key1_ptr, key1_len) = array::get_const_ptr_and_len(key1);
    let (_key2_ptr, _key2_len) = array::get_const_ptr_and_len(key2);
    let (_iv_ptr, _iv_len) = array::get_const_ptr_and_len(iv);

    if let Some(iv) = iv {
        if iv.len() != ctx.iv.len() {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }

        ctx.iv.copy_from_slice(iv);
    }

    aes_init(&mut ctx.aes_ctx);

    let mbed_res = match mode {
        TEE_OperationMode::TEE_MODE_ENCRYPT => {
            ctx.mbed_mode = AES_ENCRYPT;
            aes_setkey_enc(&mut ctx.aes_ctx, key1_ptr, key1_len as u32 * 8)
        }
        TEE_OperationMode::TEE_MODE_DECRYPT => {
            ctx.mbed_mode = AES_DECRYPT;
            aes_setkey_enc(&mut ctx.aes_ctx, key1_ptr, key1_len as u32 * 8)
        }
        _ => {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }
    };

    if mbed_res != 0 {
        return Err(TEE_ERROR_BAD_STATE);
    }

    Ok(())
}

fn mbed_aes_cbc_update(
    ctx: &mut MbedAesCbcCtx,
    _last_block: bool,
    data: Option<&[u8]>,
    dst: Option<&mut [u8]>,
) -> TeeResult {
    let (data_ptr, data_len) = array::get_const_ptr_and_len(data);
    let (dst_ptr, _dst_len) = array::get_mut_ptr_and_len(dst);

    let mbed_res = aes_crypt_cbc(
        &mut ctx.aes_ctx,
        ctx.mbed_mode,
        data_len,
        ctx.iv.as_mut_ptr(),
        data_ptr,
        dst_ptr,
    );

    if mbed_res != 0 {
        return Err(TEE_ERROR_BAD_STATE);
    }

    Ok(())
}

fn mbed_aes_cbc_final(ctx: &mut MbedAesCbcCtx) {
    aes_free(&mut ctx.aes_ctx as *mut aes_context);
}

// optee_os, the context is allocated by function crypto_aes_ecb_alloc_ctx, so need free it manually.
// in libmbedtls, the context is owned by Box with function alloc_cipher_ctx, freed automatically
// when it goes out of scope.
// So this function is a no-op in this case.
fn mbed_aes_cbc_free_ctx(_ctx: &mut MbedAesCbcCtx) {}

fn mbed_aes_cbc_copy_state(dst_ctx: &mut MbedAesCbcCtx, src_ctx: &MbedAesCbcCtx) {
    dst_ctx.iv.copy_from_slice(&src_ctx.iv);
    dst_ctx.mbed_mode = src_ctx.mbed_mode;
    dst_ctx.aes_ctx = src_ctx.aes_ctx;
}

impl CryptoCipherOps for MbedAesCbcCtx {
    fn init(
        &mut self,
        mode: TEE_OperationMode,
        key1: Option<&[u8]>,
        key2: Option<&[u8]>,
        iv: Option<&[u8]>,
    ) -> TeeResult {
        mbed_aes_cbc_init(self, mode, key1, key2, iv)
    }

    fn update(
        &mut self,
        last_block: bool,
        data: Option<&[u8]>,
        dst: Option<&mut [u8]>,
    ) -> TeeResult {
        mbed_aes_cbc_update(self, last_block, data, dst)
    }

    fn finalize(&mut self) {
        mbed_aes_cbc_final(self)
    }

    fn free_ctx(&mut self) {
        mbed_aes_cbc_free_ctx(self)
    }

    fn copy_state(&self, dst_ctx: &mut MbedAesCbcCtx) {
        mbed_aes_cbc_copy_state(dst_ctx, self);
    }
}

impl CryptoCipherCtx for MbedAesCbcCtx {
    type Context = MbedAesCbcCtx;

    fn alloc_cipher_ctx() -> Result<Box<Self::Context>, TeeResult> {
        let ctx = MbedAesCbcCtx {
            mbed_mode: 0,
            aes_ctx: aes_context,
            iv: [0; TEE_AES_BLOCK_SIZE],
        };

        Ok(Box::new(ctx))
    }
}
