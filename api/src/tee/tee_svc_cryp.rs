// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use crate::mm::vm_load_string;
use crate::tee;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use axerrno::{AxError, AxResult};
use core::ffi::c_char;
use core::{any::Any, ffi::c_uint, ffi::c_ulong, time::Duration};
use tee_raw_sys::libc_compat::size_t;

use super::tee_obj::{tee_obj, tee_obj_add};

pub fn tee_obj_set_type(O: &mut tee_obj, obj_type: u32, max_key_size: size_t) -> AxResult<isize> {
    Ok(0)
}

pub(crate) fn syscall_cryp_obj_alloc(obj_type: c_ulong, max_key_size: c_ulong) -> AxResult<c_uint> {
    let mut obj = tee_obj::default();

    tee_obj_set_type(&mut obj, obj_type as _, max_key_size as _)?;
    tee_obj_add(obj).map(|id| id as c_uint);
    Ok(0)
}
