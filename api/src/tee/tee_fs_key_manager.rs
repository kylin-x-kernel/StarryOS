// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, vec, vec::Vec};

use mbedtls::hash;
use static_assertions::const_assert;
use tee_raw_sys::{
    TEE_ALG_AES_ECB_NOPAD, TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_NOT_IMPLEMENTED, TEE_OperationMode,
    TEE_UUID,
};

use super::{
    TeeResult,
    huk_subkey::{HUK_SUBKEY_MAX_LEN, HukSubkeyUsage, huk_subkey_derive},
    otp_stubs::{TeeHwUniqueKey, tee_otp_get_hw_unique_key},
    utee_defines::{TEE_ALG, TEE_SHA256_HASH_SIZE},
};
use crate::tee::crypto_temp::{
    aes_cbc::MbedAesCbcCtx,
    crypto_temp::{CryptoCipherCtx, CryptoCipherOps},
};

const TEE_FS_KM_SSK_SIZE: usize = TEE_SHA256_HASH_SIZE;
const TEE_FS_KM_CHIP_ID_LENGTH: usize = 32;
const TEE_FS_KM_TSK_SIZE: usize = TEE_SHA256_HASH_SIZE;
pub const TEE_FS_KM_FEK_SIZE: usize = 16; /* bytes */

pub struct TeeFsSsk {
    pub is_init: bool,
    pub key: [u8; TEE_FS_KM_SSK_SIZE],
}

pub static STRING_FOR_SSK_GEN: &[u8] = b"ONLY_FOR_tee_fs_ssk";

static mut TEE_FS_SSK: TeeFsSsk = TeeFsSsk {
    is_init: false,
    key: [0u8; TEE_FS_KM_SSK_SIZE],
};

pub fn crypto_cipher_alloc_ctx(algo: TEE_ALG) -> Result<Box<dyn CryptoCipherOps>, TeeResult> {
    match algo {
        TEE_ALG_AES_ECB_NOPAD => {
            let ctx: MbedAesCbcCtx = *MbedAesCbcCtx::alloc_cipher_ctx()?;
            Ok(Box::new(ctx))
        }
        _ => Err(Err(TEE_ERROR_NOT_IMPLEMENTED)),
    }
}

pub fn do_hmac(out_key: &mut [u8], in_key: &[u8], message: &[u8]) -> TeeResult {
    // 参数检查
    if out_key.is_empty() || in_key.is_empty() || message.is_empty() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    let mut mac =
        hash::Hmac::new(hash::Type::Sha256, in_key).map_err(|_| TEE_ERROR_BAD_PARAMETERS)?;

    mac.update(message).map_err(|_| TEE_ERROR_BAD_PARAMETERS)?;

    mac.finish(out_key).map_err(|_| TEE_ERROR_BAD_PARAMETERS)?;

    Ok(())
}

impl TeeFsSsk {
    #[allow(dead_code)]
    fn is_initial(&self) -> bool {
        unsafe { TEE_FS_SSK.is_init }
    }
}
#[allow(dead_code)]
pub fn tee_fs_init_key_manager() -> TeeResult {
    const_assert!(TEE_FS_KM_SSK_SIZE <= HUK_SUBKEY_MAX_LEN);

    unsafe {
        // 获取裸指针（指向 u8）
        let key_ptr = core::ptr::addr_of_mut!(TEE_FS_SSK.key) as *mut u8;
        // 构造切片
        let key_slice: &mut [u8] = core::slice::from_raw_parts_mut(key_ptr, TEE_FS_KM_SSK_SIZE);
        let res = huk_subkey_derive(HukSubkeyUsage::Ssk, None, key_slice);

        match res {
            Ok(_) => {
                TEE_FS_SSK.is_init = true;
            }
            Err(e) => {
                // TEE_FS_SSK.key.fill(0);
                let key_ptr = core::ptr::addr_of_mut!(TEE_FS_SSK.key);
                (*key_ptr).fill(0);
                return Err(e);
            }
        }
    }
    Ok(())
}

pub fn tee_fs_fek_crypt(
    uuid: Option<&TEE_UUID>,
    mode: TEE_OperationMode,
    in_key: Option<&[u8]>,
    size: usize,
    out_key: Option<&mut [u8]>,
) -> TeeResult {
    let mut dst_key = vec![0u8; size];
    let mut tsk = [0u8; TEE_FS_KM_TSK_SIZE];

    if in_key.is_none() || out_key.is_none() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    if size != TEE_FS_KM_FEK_SIZE || size != out_key.as_ref().unwrap().len() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    unsafe {
        if TEE_FS_SSK.is_init == false {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }

        let in_key = in_key.ok_or(TEE_ERROR_BAD_PARAMETERS)?;

        if let Some(uuid) = uuid {
            let uuid_bytes = core::slice::from_raw_parts(
                (uuid as *const TEE_UUID) as *const u8,
                size_of::<TEE_UUID>(),
            );
            do_hmac(&mut tsk, in_key, uuid_bytes)?;
        } else {
            let dummy = [0u8, 1];
            do_hmac(&mut tsk, in_key, &dummy)?;
        }
    }
    let _ = match crypto_cipher_alloc_ctx(TEE_ALG_AES_ECB_NOPAD) {
        Ok(mut ctx) => {
            let _ = ctx.init(mode, Some(&tsk), None, None);
            let _ = ctx.update(true, in_key, Some(&mut dst_key));
            ctx.finalize();
            if let Some(out_key) = out_key {
                out_key.copy_from_slice(&dst_key);
            }
        }
        Err(e) => return e,
    };

    Ok(())
}
