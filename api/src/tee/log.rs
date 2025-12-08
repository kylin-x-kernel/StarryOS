// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

#[cfg(feature = "tee_test")]
use super::{
    tee_obj::tee_obj, tee_unit_test::tee_test_unit, test::test_examples::tee_test_example,
};
use crate::mm::vm_load_string;
use alloc::{boxed::Box, collections::VecDeque, string::String, sync::Arc, vec::Vec};
use axerrno::{AxError, AxResult};
use axtask::current;
use core::{any::Any, ffi::c_char};
use spin::RwLock;
use starry_core::{task::AsThread, task::TeeSessionCtxTrait};

scope_local::scope_local! {
    /// The current file descriptor table.
    pub static SESSION_CTX: Arc<RwLock<String>> = Arc::default();
}

pub struct tee_session_ctx {
    pub session_id: u32,
    pub login_type: u32,
    pub user_id: u32,
    pub objects: VecDeque<tee_obj>,
}

impl TeeSessionCtxTrait for tee_session_ctx {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub(crate) fn sys_tee_scn_log(buf: *const c_char, len: usize) -> AxResult<isize> {
    // Implementation for TEE log syscall we use info to output the log now
    info!("TEE log syscall invoked with len: {}", len);
    let message = vm_load_string(buf)?;

    // using tee_session_ctx
    current()
        .as_thread()
        .set_tee_session_ctx(Box::new(super::tee_session_ctx {
            session_id: 1,
            login_type: 2,
            user_id: 3,
            objects: VecDeque::new(),
        }));

    let current_task = current();
    let binding = &current_task.as_thread().tee_session_ctx;
    let lock = binding.lock();

    let concrete = lock
        .as_ref()
        .unwrap()
        .as_any()
        .downcast_ref::<tee_session_ctx>()
        .unwrap();
    info!(
        "TEE Session Context - session_id: {}, login_type: {}, user_id: {}",
        concrete.session_id, concrete.login_type, concrete.user_id
    );

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
