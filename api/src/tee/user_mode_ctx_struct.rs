// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use core::default::Default;

#[repr(C)]
pub struct user_mode_ctx {}

impl Default for user_mode_ctx {
    fn default() -> Self {
        Self {}
    }
}
