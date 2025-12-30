// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use super::utee_defines::HW_UNIQUE_KEY_LENGTH;
use crate::tee::TeeResult;

#[repr(C)]
pub struct TeeHwUniqueKey {
    pub data: [u8; HW_UNIQUE_KEY_LENGTH],
}

// TODO: need to be implement
pub fn tee_otp_get_hw_unique_key(hwkey: &mut TeeHwUniqueKey) -> TeeResult {
    hwkey.data.fill(0xAA);
    Ok(())
}
