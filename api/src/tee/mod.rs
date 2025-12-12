// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

#![allow(non_camel_case_types, non_snake_case)]
#![allow(unused_imports)]
mod log;
mod property;
mod tee_fs;
mod tee_obj;
mod tee_pobj;
mod tee_session;
mod tee_svc_cryp;
mod user_access;
mod libmbedtls;
mod libutee;
mod utils;
#[cfg(feature = "tee_test")]
mod tee_unit_test;
#[cfg(feature = "tee_test")]
mod test;
mod time;
mod config;
mod crypto;

use core::arch::asm;

use log::*;
use tee_raw_sys::TEE_ERROR_NOT_SUPPORTED;
use time::*;

use axerrno::{AxError, AxResult};
use axhal::uspace::UserContext;
use syscalls::Sysno;
#[cfg(feature = "tee_test")]
use test::test_framework::{TestDescriptor, TestRunner};
#[cfg(feature = "tee_test")]
use test::test_framework_basic::TestResult;

use crate::tee::property::{sys_tee_scn_get_property, sys_tee_scn_get_property_name_to_index};

pub type TeeResult<T = ()> = Result<T, u32>;

pub(crate) fn handle_tee_syscall(_sysno: Sysno, _uctx: &mut UserContext) -> TeeResult {
    // Handle TEE-specific syscalls here
    match _sysno {
        Sysno::tee_scn_log => sys_tee_scn_log(_uctx.arg0() as _, _uctx.arg1() as _),
        Sysno::tee_scn_get_property => {
            let prop_type: usize;
            unsafe {
                asm!(
                    "mov {0}, x6",
                    out(reg) prop_type,
                );
            }
            sys_tee_scn_get_property(
                _uctx.arg0() as _,
                _uctx.arg1() as _,
                _uctx.arg2() as _,
                _uctx.arg3() as _,
                _uctx.arg4() as _,
                _uctx.arg5() as _,
                prop_type as _,
            )
        }
        Sysno::tee_scn_get_property_name_to_index => sys_tee_scn_get_property_name_to_index(
            _uctx.arg0() as _,
            _uctx.arg1() as _,
            _uctx.arg2() as _,
            _uctx.arg3() as _,
        ),
        _ => Err(TEE_ERROR_NOT_SUPPORTED),
    }
}
