// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::boxed::Box;

use mbedtls_sys_auto::{
    AES_DECRYPT, AES_ENCRYPT, aes_context, aes_crypt_cbc, aes_free, aes_init, aes_setkey_dec,
    aes_setkey_enc,
};
use tee_raw_sys::{TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_BAD_STATE, TEE_OperationMode};

use super::crypto_temp::{CryptoCipherCtx, CryptoCipherOps};
use crate::tee::{TeeResult, common::array, utee_defines::TEE_AES_BLOCK_SIZE, utils::slice_fmt};

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
    tee_debug!(
        "mbed_aes_cbc_init: mode: {:?}, key1: {:?}, key2: {:?}, iv: {:?}",
        mode,
        key1,
        key2,
        iv
    );
    let (key1_ptr, key1_len) = array::get_const_ptr_and_len(key1);
    let (_key2_ptr, _key2_len) = array::get_const_ptr_and_len(key2);
    let (_iv_ptr, _iv_len) = array::get_const_ptr_and_len(iv);

    if let Some(iv) = iv {
        if iv.len() != ctx.iv.len() {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }

        ctx.iv.copy_from_slice(iv);
    }

    unsafe { aes_init(&mut ctx.aes_ctx) };

    let mbed_res = match mode {
        TEE_OperationMode::TEE_MODE_ENCRYPT => {
            ctx.mbed_mode = AES_ENCRYPT;
            unsafe { aes_setkey_enc(&mut ctx.aes_ctx, key1_ptr, key1_len as u32 * 8) }
        }
        TEE_OperationMode::TEE_MODE_DECRYPT => {
            ctx.mbed_mode = AES_DECRYPT;
            unsafe { aes_setkey_dec(&mut ctx.aes_ctx, key1_ptr, key1_len as u32 * 8) }
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

    tee_debug!(
        "mbed_aes_cbc_update: mode: {:?}, data_len: {:?}, dst_len: {:?}",
        ctx.mbed_mode,
        data_len,
        _dst_len
    );
    let mbed_res = unsafe {
        aes_crypt_cbc(
            &mut ctx.aes_ctx,
            ctx.mbed_mode,
            data_len,
            ctx.iv.as_mut_ptr(),
            data_ptr,
            dst_ptr,
        )
    };

    if mbed_res != 0 {
        return Err(TEE_ERROR_BAD_STATE);
    }

    Ok(())
}

fn mbed_aes_cbc_final(ctx: &mut MbedAesCbcCtx) {
    unsafe { aes_free(&mut ctx.aes_ctx as *mut aes_context) };
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
            aes_ctx: aes_context::default(),
            iv: [0; TEE_AES_BLOCK_SIZE],
        };

        Ok(Box::new(ctx))
    }
}

#[cfg(feature = "tee_test")]
pub mod tests_aes_cbc {
    use hashbrown::hash_map::Keys;

    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{TestDescriptor, TestResult},
        test_fn, tests, tests_name,
    };

    test_fn! {
        using TestResult;

        fn test_tee_aes_cbc_init_update_final() {
            // test encrypt
            let plaintext = [1u8; 16];
            let iv = [3u8; 16];
            let Key = [2u8; 16];
            let mut ciphertext = [0u8; 16];
            let mut ctx = MbedAesCbcCtx::alloc_cipher_ctx().expect("Failed to allocate AES CBC context");
            let _ = ctx.init(TEE_OperationMode::TEE_MODE_ENCRYPT, Some(&Key), None, Some(&iv)).expect("Failed to initialize AES CBC context");
            let _ = ctx.update(true, Some(&plaintext), Some(&mut ciphertext)).expect("Failed to update AES CBC context");
            ctx.finalize();
            tee_debug!("ciphertext: {:?}", slice_fmt(&ciphertext));
            // test decrypt
            let mut decrypted_text = [0u8; 16];
            let _ = ctx.init(TEE_OperationMode::TEE_MODE_DECRYPT, Some(&Key), None, Some(&iv)).expect("Failed to initialize AES CBC context");
            let _ = ctx.update(true, Some(&ciphertext), Some(&mut decrypted_text)).expect("Failed to update AES CBC context");
            ctx.finalize();
            tee_debug!("decrypted_text: {:?}", slice_fmt(&decrypted_text));
            assert_eq!(decrypted_text, plaintext);
        }
    }

    tests_name! {
        TEST_TEE_AES_CBC;
        //------------------------
        test_tee_aes_cbc_init_update_final,
    }
}
