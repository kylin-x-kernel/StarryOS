// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, collections::VecDeque, string::String, sync::Arc, vec::Vec};
use core::{any::Any, ffi::c_char};

use axerrno::{AxError, AxResult};
use spin::RwLock;
use tee_raw_sys::TEE_ERROR_BAD_PARAMETERS;

#[cfg(feature = "tee_test")]
use super::{
    tee_obj::tee_obj, tee_unit_test::tee_test_unit, test::test_examples::tee_test_example,
};
use crate::{mm::vm_load_any_string, tee::TeeResult};

scope_local::scope_local! {
    /// The current file descriptor table.
    pub static SESSION_CTX: Arc<RwLock<String>> = Arc::default();
}

pub(crate) fn sys_tee_scn_log(buf: *const c_char, len: usize) -> TeeResult {
    // Implementation for TEE log syscall we use info to output the log now
    info!("TEE log syscall invoked with len: {}", len);
    let message = match vm_load_any_string(buf, len) {
        Ok(s) => s,
        Err(_) => return Err(TEE_ERROR_BAD_PARAMETERS),
    };

    info!("TEE Log: {}", message);

    let mut ctx = SESSION_CTX.write();
    ctx.push_str("abc");
    info!("after push {}", *ctx);

    #[cfg(feature = "tee_test")]
    tee_test_example();
    #[cfg(feature = "tee_test")]
    tee_test_unit();

    Ok(())
}
