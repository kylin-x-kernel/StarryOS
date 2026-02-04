// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use crate::tee::{TeeResult};

pub fn vm_check_access_rights(
    _flags: u32,
    uaddr: usize,
    len: usize,
) -> TeeResult {
    Ok(())
}
