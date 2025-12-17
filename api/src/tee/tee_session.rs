// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use super::{TeeResult, tee_obj::tee_obj};
use alloc::{boxed::Box, sync::Arc};
use axtask::current;
use core::{any::Any, default::Default};
use slab::Slab;
use starry_core::task::{AsThread, TeeSessionCtxTrait};
use tee_raw_sys::*;

pub struct tee_session_ctx {
    pub session_id: u32,
    pub login_type: u32,
    pub user_id: u32,
    pub objects: Slab<Arc<tee_obj>>,
    pub clnt_id: TEE_Identity,
}

impl TeeSessionCtxTrait for tee_session_ctx {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for tee_session_ctx {
    fn default() -> Self {
        tee_session_ctx {
            session_id: 0,
            login_type: 0,
            user_id: 0,
            objects: Slab::new(),
            clnt_id: TEE_Identity {
                login: 0,
                uuid: TEE_UUID {
                    timeLow: 0,
                    timeMid: 0,
                    timeHiAndVersion: 0,
                    clockSeqAndNode: [0; 8],
                },
            },
        }
    }
}

/// 获取当前线程的 tee_session_ctx 的可变引用，并在闭包中使用
///
/// # 参数
/// - `f`: 一个接受 `&mut tee_session_ctx` 的闭包
///
/// # 返回
/// 闭包的返回值
pub fn with_tee_session_ctx_mut<F, R>(f: F) -> TeeResult<R>
where
    F: FnOnce(&mut tee_session_ctx) -> TeeResult<R>,
{
    let current_task = current();
    current_task
        .as_thread()
        .set_tee_session_ctx(Box::new(tee_session_ctx::default()));

    let binding = &current_task.as_thread().tee_session_ctx;
    let mut lock = binding.lock();

    let concrete = {
        let boxed = lock.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;
        boxed
            .as_any_mut()
            .downcast_mut::<tee_session_ctx>()
            .ok_or(TEE_ERROR_BAD_STATE)?
    };

    f(concrete)
}

/// 获取当前线程的 tee_session_ctx 的不可变引用，并在闭包中使用
///
/// # 参数
/// - `f`: 一个接受 `&tee_session_ctx` 的闭包
///
/// # 返回
/// 闭包的返回值
pub fn with_tee_session_ctx<F, R>(f: F) -> TeeResult<R>
where
    F: FnOnce(&tee_session_ctx) -> TeeResult<R>,
{
    let current_task = current();
    current_task
        .as_thread()
        .set_tee_session_ctx(Box::new(tee_session_ctx::default()));

    let binding = &current_task.as_thread().tee_session_ctx;
    let lock = binding.lock();

    let concrete = {
        let boxed = lock.as_ref().ok_or(TEE_ERROR_BAD_STATE)?;
        boxed
            .as_any()
            .downcast_ref::<tee_session_ctx>()
            .ok_or(TEE_ERROR_BAD_STATE)?
    };

    f(concrete)
}
