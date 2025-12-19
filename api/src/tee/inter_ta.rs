// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use core::{
    ffi::{c_uint, c_ulong},
    ptr::addr_of,
};

use alloc::string::ToString;
use tee_raw_sys::{TEE_Identity, TEE_LOGIN_TRUSTED_APP, TEE_UUID, utee_params};

use crate::tee::{
    TeeResult,
    tee_session::{tee_session_ctx, with_tee_session_ctx},
    tee_ta_manager::{tee_ta_close_session, tee_ta_get_session, tee_ta_init_session},
    user_access::copy_from_user,
    uuid::Uuid,
};

pub(crate) fn sys_tee_scn_open_ta_session(
    dest: *const TEE_UUID,
    cancel_req_to: c_ulong,
    usr_param: *mut utee_params,
    ta_sees: *mut c_uint,
    ret_orig: *mut c_uint,
) -> TeeResult {
    let uuid = TEE_UUID {
        timeLow: 0,
        timeMid: 0,
        timeHiAndVersion: 0,
        clockSeqAndNode: [0; 8],
    };
    let uuid_size = core::mem::size_of::<TEE_UUID>();
    copy_from_user(
        unsafe { core::slice::from_raw_parts_mut(addr_of!(uuid) as _, uuid_size) },
        unsafe { core::slice::from_raw_parts(dest as _, uuid_size) },
        uuid_size,
    )?;

    tee_ta_init_session(Uuid::from(uuid).to_string())?;

    Ok(())
}

pub(crate) fn sys_tee_scn_close_ta_session(ta_sees: c_ulong) -> TeeResult {
    let sess_id = tee_ta_get_session(ta_sees as u32)?;
    tee_ta_close_session(sess_id)?;
    Ok(())
}
