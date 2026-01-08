// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

#![allow(non_camel_case_types, non_snake_case)]
#![allow(unused_imports)]
#![allow(unused)]
#![allow(missing_docs)]
#![allow(non_upper_case_globals)]
#[macro_use]
mod macros;
mod bitstring;
mod cancel;
mod common;
mod config;
mod crypto;
mod fs_dirfile;
mod fs_htree;
mod inter_ta;
mod libmbedtls;
mod libutee;
mod log;
mod memtag;
mod panic;
mod property;
mod protocol;
mod ree_fs_rpc;
mod tee_fs;
mod tee_misc;
mod tee_obj;
mod tee_pobj;
mod tee_ree_fs;
mod tee_return;
mod tee_session;
mod tee_svc_cryp;
mod tee_svc_cryp2;
mod tee_svc_storage;
mod tee_ta_manager;
mod utee_defines;
// mod ts_manager;
mod crypto_temp;
mod huk_subkey;
mod otp_stubs;
mod tee_api_defines_extensions;
mod tee_fs_key_manager;
mod tee_time;
#[cfg(feature = "tee_test")]
mod tee_unit_test;
#[cfg(feature = "tee_test")]
mod test;
mod types_ext;
mod user_access;
mod user_mode_ctx_struct;
mod user_ta;
mod utils;
mod uuid;
mod vm;
use core::arch::asm;

use axerrno::{AxError, AxResult};
use axhal::uspace::UserContext;
use cancel::*;
use log::*;
use syscalls::Sysno;
pub use tee_api_defines_extensions::*;
use tee_raw_sys::{TEE_ERROR_NOT_SUPPORTED, TeeTime};
pub use tee_ree_fs::{
    ree_fs_rpc_read_init as rpc_read_init, ree_fs_rpc_write_init as rpc_write_init,
    tee_fs_rpc_read_final as rpc_read_final, tee_fs_rpc_write_final as rpc_write_final,
};
use tee_return::sys_tee_scn_return;
#[cfg(feature = "tee_test")]
use test::test_framework::{TestDescriptor, TestRunner};
#[cfg(feature = "tee_test")]
use test::test_framework_basic::TestResult;

use crate::tee::{
    inter_ta::{
        sys_tee_scn_close_ta_session, sys_tee_scn_invoke_ta_command, sys_tee_scn_open_ta_session,
    },
    panic::sys_tee_scn_panic,
    property::{sys_tee_scn_get_property, sys_tee_scn_get_property_name_to_index},
    // tee_svc_cryp::sys_tee_scn_hash_init
    tee_svc_cryp2::sys_tee_scn_hash_init,
    tee_svc_cryp2::sys_tee_scn_hash_update,
    tee_time::{sys_tee_scn_get_time, sys_tee_scn_set_ta_time, sys_tee_scn_wait},
};

pub type TeeResult<T = ()> = Result<T, u32>;

pub(crate) fn handle_tee_syscall(_sysno: Sysno, _uctx: &mut UserContext) -> TeeResult {
    // Handle TEE-specific syscalls here
    match _sysno {
        Sysno::tee_scn_return => sys_tee_scn_return(_uctx.arg0() as _),
        Sysno::tee_scn_log => sys_tee_scn_log(_uctx.arg0() as _, _uctx.arg1() as _),
        Sysno::tee_scn_panic => sys_tee_scn_panic(_uctx.arg0() as _),
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
        Sysno::tee_scn_open_ta_session => sys_tee_scn_open_ta_session(
            _uctx.arg0() as _,
            _uctx.arg1() as _,
            _uctx.arg2() as _,
            _uctx.arg3() as _,
            _uctx.arg4() as _,
        ),
        Sysno::tee_scn_close_ta_session => sys_tee_scn_close_ta_session(_uctx.arg0() as _),
        Sysno::tee_scn_invoke_ta_command => sys_tee_scn_invoke_ta_command(
            _uctx.arg0() as _,
            _uctx.arg1() as _,
            _uctx.arg2() as _,
            _uctx.arg3() as _,
            _uctx.arg4() as _,
        ),
        Sysno::tee_scn_get_cancellation_flag => {
            sys_tee_scn_get_cancellation_flag(_uctx.arg0() as _)
        }
        Sysno::tee_scn_unmask_cancellation => sys_tee_scn_unmask_cancellation(_uctx.arg0() as _),
        Sysno::tee_scn_mask_cancellation => sys_tee_scn_mask_cancellation(_uctx.arg0() as _),
        Sysno::tee_scn_wait => sys_tee_scn_wait(_uctx.arg0() as u32),

        Sysno::tee_scn_get_time => {
            let teetime_ptr = _uctx.arg1() as *mut TeeTime;
            let teetime_ref = unsafe { &mut *teetime_ptr };
            sys_tee_scn_get_time(_uctx.arg0() as _, teetime_ref)
        }
        Sysno::tee_scn_set_ta_time => {
            let teetime_ptr = _uctx.arg1() as *const TeeTime;
            let teetime_ref = unsafe { &*teetime_ptr };
            sys_tee_scn_set_ta_time(teetime_ref)
        }

        // Sysno::tee_scn_hash_init => sys_tee_scn_hash_init(_uctx.arg0() as _, _uctx.arg1() as _, _uctx.arg2() as _),
        Sysno::tee_scn_hash_init => {
            sys_tee_scn_hash_init(_uctx.arg0() as _, _uctx.arg1() as _, _uctx.arg2() as _)
        }

        Sysno::tee_scn_hash_update => {
            sys_tee_scn_hash_update(_uctx.arg0() as _, _uctx.arg1() as _, _uctx.arg2() as _)
        }

        _ => Err(TEE_ERROR_NOT_SUPPORTED),
    }
}
