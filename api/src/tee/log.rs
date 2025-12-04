// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::vec::Vec;
use alloc::{sync::Arc};
use spin::RwLock;
use alloc::string::String;
use core::ffi::c_char;
use axerrno::{AxError, AxResult};
use crate::mm::vm_load_string;
#[cfg(feature = "tee_test")]
use super::{
    test::test_examples::tee_test_example,
    tee_unit_test::tee_test_unit,
};

scope_local::scope_local! {
    /// The current file descriptor table.
    pub static SESSION_CTX: Arc<RwLock<String>> = Arc::default();
}

pub(crate) fn sys_tee_scn_log(buf: *const c_char, len: usize) -> AxResult<isize> {
    // Implementation for TEE log syscall we use info to output the log now
    info!("TEE log syscall invoked with len: {}", len);
    let message = vm_load_string(buf)?;

    info!("TEE Log: {}", message);

    let mut ctx = SESSION_CTX.write();
    ctx.push_str("abc");
    info!("after push {}", *ctx);

    #[cfg(feature = "tee_test")]
    tee_test_example();
    #[cfg(feature = "tee_test")]
    tee_test_unit();
    Ok(0)
}