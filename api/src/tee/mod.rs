// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

#![allow(non_camel_case_types, non_snake_case)]
#![allow(unused_imports)]
mod log;
mod tee_fs;
mod tee_obj;
mod tee_pobj;
mod tee_svc_cryp;
mod user_access;
mod tee_session;

#[cfg(feature = "tee_test")]
mod tee_unit_test;
#[cfg(feature = "tee_test")]
mod test;
mod libutee;
mod time;

use log::*;
use time::*;

use axerrno::{AxError, AxResult};
use axhal::uspace::UserContext;
use syscalls::Sysno;
#[cfg(feature = "tee_test")]
use test::test_framework::{TestDescriptor, TestRunner};
#[cfg(feature = "tee_test")]
use test::test_framework_basic::TestResult;

pub type TeeResult<T = ()> = Result<T, u32>;

pub(crate) fn handle_tee_syscall(_sysno: Sysno, _uctx: &mut UserContext) -> AxResult<isize> {
    // Handle TEE-specific syscalls here
    match _sysno {
        Sysno::tee_scn_log => sys_tee_scn_log(_uctx.arg0() as _, _uctx.arg1() as _),
        _ => Err(AxError::Unsupported),
    }
}
