// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use crate::tee::TeeResult;
use crate::tee::user_mode_ctx_struct::user_mode_ctx;

pub fn vm_check_access_rights(
    _uctx: &mut user_mode_ctx,
    _flags: u32,
    uaddr: usize,
    len: usize,
) -> TeeResult {
    Ok(())
}
