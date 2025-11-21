// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.


mod time;
mod log;

use time::*;
use log::*;

use axhal::uspace::UserContext;
use axerrno::{AxError, AxResult};
use syscalls::Sysno;

pub(crate) fn handle_tee_syscall(_sysno: Sysno, _uctx: &mut UserContext) -> AxResult<isize>  {
    // Handle TEE-specific syscalls here
    match _sysno {
        Sysno::tee_scn_log => sys_tee_scn_log(
            _uctx.arg0() as _,
            _uctx.arg1() as _,
        ),
        _ => Err(AxError::Unsupported),
    }
}
