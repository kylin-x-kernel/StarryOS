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
mod rng_software;
mod tee_fs;
mod tee_misc;
mod tee_obj;
mod tee_pobj;
pub mod tee_ree_fs;
mod tee_return;
mod tee_session;
mod tee_svc_cryp;
mod tee_svc_cryp2;
mod tee_svc_storage;
mod tee_ta_manager;
mod utee_defines;
// mod ts_manager;
mod crypto_temp;
#[cfg(feature = "tee_test")]
mod fs_htree_tests;
mod huk_subkey;
mod otp_stubs;
mod tee_api_defines_extensions;
mod tee_fs_key_manager;
mod tee_time;
#[cfg(target_arch = "x86_64")]
mod tee_get_sealing_key;
#[cfg(feature = "tee_test")]
mod tee_unit_test;
#[cfg(feature = "tee_test")]
mod test;
mod types_ext;
mod user_access;
mod user_ta;
mod utils;
mod uuid;
mod vm;
use core::{arch::asm, ffi::c_uint};

use axerrno::{AxError, AxResult};
use axhal::uspace::UserContext;
use cancel::*;
use log::*;
use syscalls::Sysno;
pub use tee_api_defines_extensions::*;
use tee_raw_sys::{TEE_ERROR_NOT_SUPPORTED, TeeTime};
use tee_return::sys_tee_scn_return;
#[cfg(feature = "tee_test")]
use test::test_framework::{TestDescriptor, TestRunner};
#[cfg(feature = "tee_test")]
use test::test_framework_basic::TestResult;

#[cfg(feature = "tee_test")]
use crate::tee::test::sys_tee_scn_test;
use crate::tee::{
    inter_ta::{
        sys_tee_scn_close_ta_session, sys_tee_scn_invoke_ta_command, sys_tee_scn_open_ta_session,
    },
    panic::sys_tee_scn_panic,
    property::{sys_tee_scn_get_property, sys_tee_scn_get_property_name_to_index},
    tee_svc_cryp::{
        syscall_cryp_obj_alloc, syscall_cryp_obj_close, syscall_cryp_obj_copy,
        syscall_cryp_obj_get_attr, syscall_cryp_obj_get_info, syscall_cryp_obj_populate,
        syscall_cryp_obj_reset, syscall_cryp_obj_restrict_usage, syscall_obj_generate_key,
    },
    tee_svc_cryp2::sys_tee_scn_hash_final,
    // tee_svc_cryp::sys_tee_scn_hash_init
    tee_svc_cryp2::sys_tee_scn_hash_init,
    tee_svc_cryp2::sys_tee_scn_hash_update,
    tee_svc_storage::{
        syscall_storage_alloc_enum, syscall_storage_free_enum, syscall_storage_next_enum,
        syscall_storage_obj_create, syscall_storage_obj_del, syscall_storage_obj_open,
        syscall_storage_obj_read, syscall_storage_obj_rename, syscall_storage_obj_seek,
        syscall_storage_obj_trunc, syscall_storage_obj_write, syscall_storage_reset_enum,
        syscall_storage_start_enum,
    },
    tee_time::{sys_tee_scn_get_time, sys_tee_scn_set_ta_time, sys_tee_scn_wait},
};

pub type TeeResult<T = ()> = Result<T, u32>;

pub(crate) fn handle_tee_syscall(_sysno: Sysno, _uctx: &mut UserContext) -> TeeResult {
    // back up x6, x7
    let old_x6: usize;
    let old_x7: usize;
    unsafe {
        asm!(
            "",
            lateout("x6") old_x6,
            lateout("x7") old_x7,
            options(nostack, preserves_flags),
        );
    }
    tee_debug!("---> TEE syscall: sysno: {:?}", _sysno);
    // restore x6, x7
    unsafe {
        asm!(
            "",
            in("x6") old_x6,
            in("x7") old_x7,
            options(nostack, preserves_flags),
        );
    }
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

        Sysno::tee_scn_hash_update => sys_tee_scn_hash_final(
            _uctx.arg0() as _,
            _uctx.arg1() as _,
            _uctx.arg2() as _,
            _uctx.arg3() as _,
            _uctx.arg4() as _,
        ),

        Sysno::tee_scn_cryp_obj_get_info => {
            syscall_cryp_obj_get_info(_uctx.arg0() as _, _uctx.arg1() as _)
        }

        Sysno::tee_scn_cryp_obj_restrict_usage => {
            syscall_cryp_obj_restrict_usage(_uctx.arg0() as _, _uctx.arg1() as _)
        }

        Sysno::tee_scn_cryp_obj_get_attr => syscall_cryp_obj_get_attr(
            _uctx.arg0() as _,
            _uctx.arg1() as _,
            _uctx.arg2() as _,
            _uctx.arg3() as _,
        ),

        Sysno::tee_scn_cryp_obj_alloc => {
            syscall_cryp_obj_alloc(_uctx.arg0() as _, _uctx.arg1() as _, _uctx.arg2() as _)
        }

        Sysno::tee_scn_cryp_obj_close => syscall_cryp_obj_close(_uctx.arg0() as _),

        Sysno::tee_scn_cryp_obj_reset => syscall_cryp_obj_reset(_uctx.arg0() as _),

        Sysno::tee_scn_cryp_obj_populate => {
            syscall_cryp_obj_populate(_uctx.arg0() as _, _uctx.arg1() as _, _uctx.arg2() as _)
        }

        Sysno::tee_scn_cryp_obj_copy => syscall_cryp_obj_copy(_uctx.arg0() as _, _uctx.arg1() as _),

        Sysno::tee_scn_storage_obj_open => syscall_storage_obj_open(
            _uctx.arg0() as _,
            _uctx.arg1() as _,
            _uctx.arg2() as _,
            _uctx.arg3() as _,
            _uctx.arg4() as _,
        ),

        Sysno::tee_scn_storage_obj_create => {
            let len: usize;
            let obj_ptr: *mut c_uint;

            unsafe {
                asm!(
                    "mov {len}, x6",
                    "mov {obj}, x7",
                    len = out(reg) len,
                    obj = out(reg) obj_ptr,
                    options(nostack, preserves_flags),
                );
            }
            syscall_storage_obj_create(
                _uctx.arg0() as _,
                _uctx.arg1() as _,
                _uctx.arg2() as _,
                _uctx.arg3() as _,
                _uctx.arg4() as _,
                _uctx.arg5() as _,
                len as _,
                obj_ptr as _,
            )
        }

        Sysno::tee_scn_storage_obj_del => syscall_storage_obj_del(_uctx.arg0() as _),

        Sysno::tee_scn_storage_obj_rename => {
            syscall_storage_obj_rename(_uctx.arg0() as _, _uctx.arg1() as _, _uctx.arg2() as _)
        }

        Sysno::tee_scn_storage_enum_alloc => syscall_storage_alloc_enum(_uctx.arg0() as _),

        Sysno::tee_scn_storage_enum_free => syscall_storage_free_enum(_uctx.arg0() as _),

        Sysno::tee_scn_storage_enum_reset => syscall_storage_reset_enum(_uctx.arg0() as _),

        Sysno::tee_scn_storage_enum_start => {
            syscall_storage_start_enum(_uctx.arg0() as _, _uctx.arg1() as _)
        }

        Sysno::tee_scn_storage_enum_next => syscall_storage_next_enum(
            _uctx.arg0() as _,
            _uctx.arg1() as _,
            _uctx.arg2() as _,
            _uctx.arg3() as _,
        ),

        Sysno::tee_scn_storage_obj_read => syscall_storage_obj_read(
            _uctx.arg0() as _,
            _uctx.arg1() as _,
            _uctx.arg2() as _,
            _uctx.arg3() as _,
        ),

        Sysno::tee_scn_storage_obj_write => {
            syscall_storage_obj_write(_uctx.arg0() as _, _uctx.arg1() as _, _uctx.arg2() as _)
        }

        Sysno::tee_scn_storage_obj_trunc => {
            syscall_storage_obj_trunc(_uctx.arg0() as _, _uctx.arg1() as _)
        }

        Sysno::tee_scn_storage_obj_seek => {
            syscall_storage_obj_seek(_uctx.arg0() as _, _uctx.arg1() as _, _uctx.arg2() as _)
        }

        Sysno::tee_scn_cryp_obj_generate_key => syscall_obj_generate_key(
            _uctx.arg0() as _,
            _uctx.arg1() as _,
            _uctx.arg2() as _,
            _uctx.arg3() as _,
        ),
        #[cfg(feature = "tee_test")]
        Sysno::tee_scn_test => sys_tee_scn_test(),

        _ => Err(TEE_ERROR_NOT_SUPPORTED),
    }
}
