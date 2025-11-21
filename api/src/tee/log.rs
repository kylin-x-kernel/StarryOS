// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.


use core::ffi::c_char;
use axerrno::{AxError, AxResult};
use crate::mm::vm_load_string;

pub(crate) fn sys_tee_scn_log(buf: *const c_char, len: usize) -> AxResult<isize> {
    // Implementation for TEE log syscall we use info to output the log now
    info!("TEE log syscall invoked with len: {}", len);
    let message = vm_load_string(buf)?;
    info!("TEE Log: {}", message);
    Ok(0)
}
