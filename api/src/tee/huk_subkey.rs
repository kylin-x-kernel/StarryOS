// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use tee_raw_sys::{TEE_ERROR_BAD_PARAMETERS, TEE_UUID};

use super::{
    otp_stubs::{TeeHwUniqueKey, tee_otp_get_hw_unique_key},
    utee_defines::{HW_UNIQUE_KEY_LENGTH, TEE_SHA256_HASH_SIZE},
};
use crate::tee::TeeResult;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HukSubkeyUsage {
    Rpmb     = 0,
    Ssk      = 1,
    DieId    = 2,
    UniqueTa = 3,
    TaEnc    = 4,
}

pub const HUK_SUBKEY_MAX_LEN: usize = TEE_SHA256_HASH_SIZE;

pub fn huk_subkey_derive(
    _usage: HukSubkeyUsage,
    _const_data: Option<&[u8]>,
    subkey: &mut [u8],
) -> TeeResult {
    let mut huk = TeeHwUniqueKey {
        data: [0; HW_UNIQUE_KEY_LENGTH],
    };

    if subkey.len() > HUK_SUBKEY_MAX_LEN {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    tee_otp_get_hw_unique_key(&mut huk)?;

    // TODO: subkey derive from HUK
    for byte in subkey.iter_mut() {
        *byte = 0xAB;
    }

    Ok(())
}
