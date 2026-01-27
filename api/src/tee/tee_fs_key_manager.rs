// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, vec, vec::Vec};
use core::mem::size_of;

use lazy_static::lazy_static;
use mbedtls::hash;
use spin::Mutex;
use static_assertions::const_assert;
use tee_raw_sys::{
    TEE_ALG_AES_ECB_NOPAD, TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_GENERIC, TEE_ERROR_NOT_IMPLEMENTED,
    TEE_OperationMode, TEE_UUID,
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

#[derive(Debug, Clone)]
pub struct TeeFsSsk {
    pub is_init: bool,
    pub key: [u8; TEE_FS_KM_SSK_SIZE],
}

pub static STRING_FOR_SSK_GEN: &[u8] = b"ONLY_FOR_tee_fs_ssk";

const_assert!(TEE_FS_KM_SSK_SIZE <= HUK_SUBKEY_MAX_LEN);

// Helper function to initialize SSK
fn init_ssk() -> TeeFsSsk {
    let mut ssk = TeeFsSsk {
        is_init: false,
        key: [0u8; TEE_FS_KM_SSK_SIZE],
    };

    let res = huk_subkey_derive(HukSubkeyUsage::Ssk, None, &mut ssk.key);

    match res {
        Ok(_) => {
            ssk.is_init = true;
        }
        Err(_) => {
            // If initialization fails, keep is_init = false and key filled with zeros
            ssk.key.fill(0);
            error!("init_ssk: huk_subkey_derive failed");
        }
    }

    tee_debug!("init_ssk: ssk: {:?}", ssk);
    ssk
}

lazy_static! {
    static ref TEE_FS_SSK: Mutex<TeeFsSsk> = Mutex::new(init_ssk());
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
        TEE_FS_SSK.lock().is_init
    }
}

pub fn tee_fs_fek_crypt(
    uuid: Option<&TEE_UUID>,
    mode: TEE_OperationMode,
    in_key: Option<&[u8]>,
    size: usize,
    out_key: Option<&mut [u8]>,
) -> TeeResult {
    tee_debug!(
        "tee_fs_fek_crypt: uuid: {:?}, mode: {:?}, in_key: {:?}, size: {:?}, out_key: {:?}",
        uuid,
        mode as u32,
        in_key,
        size,
        out_key
    );
    let mut dst_key = vec![0u8; size];
    let mut tsk = [0u8; TEE_FS_KM_TSK_SIZE];

    if in_key.is_none() || out_key.is_none() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    if size != TEE_FS_KM_FEK_SIZE || size != out_key.as_ref().unwrap().len() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    // Extract in_key slice before unsafe block
    let in_key_slice = in_key.ok_or(TEE_ERROR_BAD_PARAMETERS)?;

    let ssk = TEE_FS_SSK.lock();
    if ssk.is_init == false {
        error!("tee_fs_fek_crypt: TEE_FS_SSK is not initialized");
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    // Use SSK.key as HMAC key, not in_key_slice (FEK)
    // Consistent with C implementation: do_hmac(tsk, sizeof(tsk), tee_fs_ssk.key, TEE_FS_KM_SSK_SIZE, uuid, sizeof(*uuid))
    let ssk_key_slice = &ssk.key[..];

    if let Some(uuid) = uuid {
        let uuid_bytes = unsafe {
            core::slice::from_raw_parts(
                (uuid as *const TEE_UUID) as *const u8,
                size_of::<TEE_UUID>(),
            )
        };
        do_hmac(&mut tsk, ssk_key_slice, uuid_bytes)?;
    } else {
        let dummy = [0u8, 1];
        do_hmac(&mut tsk, ssk_key_slice, &dummy)?;
    }
    let _ = match crypto_cipher_alloc_ctx(TEE_ALG_AES_ECB_NOPAD) {
        Ok(mut ctx) => {
            ctx.init(mode, Some(&tsk), None, None).inspect_err(|_| {
                error!("tee_fs_fek_crypt: ctx.init failed");
            })?;
            ctx.update(true, Some(in_key_slice), Some(&mut dst_key))
                .inspect_err(|_| {
                    error!("tee_fs_fek_crypt: ctx.update failed");
                })?;
            ctx.finalize();
            if let Some(out_key) = out_key {
                out_key.copy_from_slice(&dst_key);
                tee_debug!(
                    "tee_fs_fek_crypt: in_key: {:?}, out_key: {:?}",
                    hex::encode(in_key_slice),
                    hex::encode(out_key)
                );
            }
        }
        Err(e) => return e,
    };

    Ok(())
}
