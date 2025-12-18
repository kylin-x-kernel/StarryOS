// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use core::default::Default;

/// user ta context
/// NOTE: NEVER USE THIS STRUCT IN YOUR CODE
#[repr(C)]
pub struct user_ta_ctx {}

impl Default for user_ta_ctx {
    fn default() -> Self {
        Self {}
    }
}
