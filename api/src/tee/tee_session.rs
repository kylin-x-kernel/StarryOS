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
use spin::RwLock;
use starry_core::task::{AsThread, TeeSessionCtxTrait};
use tee_raw_sys::*;

scope_local::scope_local! {
    /// The tee ta context.
    pub static TEE_TA_CTX: Arc<RwLock<tee_ta_ctx>> = Arc::default();
}

pub struct tee_session_ctx {
    pub session_id: u32,
    pub login_type: u32,
    pub user_id: u32,
    pub objects: Slab<Arc<tee_obj>>,
    pub clnt_id: TEE_Identity,
    pub cancel: bool,
    pub cancel_mask: bool,
    pub cancel_time: TeeTime,
}

#[repr(C)]
#[derive(Default, Debug)]
pub struct tee_ta_ctx {
    #[cfg(feature = "tee_test")]
    pub for_test_only: u32,
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
            cancel: false,
            cancel_mask: false,
            cancel_time: TeeTime { seconds: 0, millis: 0, },
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

/// 获取当前线程的 tee_ta_ctx 的可变引用，并在闭包中使用
/// 闭包使用可确保锁的正确释放
/// 
/// # 参数
/// - `f`: 一个接受 `&mut tee_ta_ctx` 的闭包
///
/// # 返回
/// 闭包的返回值
pub fn with_tee_ta_ctx_mut<F, R>(f: F) -> TeeResult<R>
where
    F: FnOnce(&mut tee_ta_ctx) -> TeeResult<R>,
{
    let mut ta_ctx = TEE_TA_CTX.write();
    f(&mut *ta_ctx)
}

/// 获取当前线程的 tee_ta_ctx 的不可变引用，并在闭包中使用
/// 闭包使用可确保锁的正确释放
/// 
/// # 参数
/// - `f`: 一个接受 `&tee_ta_ctx` 的闭包
///
/// # 返回
/// 闭包的返回值
pub fn with_tee_ta_ctx<F, R>(f: F) -> TeeResult<R>
where
    F: FnOnce(&tee_ta_ctx) -> TeeResult<R>,
{
    let ta_ctx = TEE_TA_CTX.read();
    f(&*ta_ctx)
}

#[cfg(feature = "tee_test")]
pub mod tests_tee_session {
    //-------- test framework import --------
    use crate::tee::TestDescriptor;
    use crate::tee::TestResult;
    use crate::test_fn;
    use crate::{assert, assert_eq, assert_ne, tests, tests_name};

    //-------- local tests import --------
    use super::*;

    test_fn! {
        using TestResult;

        fn test_tee_ta_ctx() {
            // test read TEE_TA_CTX
            let mut test_only: u32 = 0;
            {
                let ta_ctx = TEE_TA_CTX.read();
                test_only = ta_ctx.for_test_only;
            }

            // test write TEE_TA_CTX
            {
                let mut ta_ctx = TEE_TA_CTX.write();
                ta_ctx.for_test_only = test_only + 1;
                assert_eq!(ta_ctx.for_test_only, test_only + 1);
            }

            // read again
            {
                let ta_ctx = TEE_TA_CTX.read();
                assert_eq!(ta_ctx.for_test_only, test_only + 1);
            }
        }
    }

    test_fn! {
        using TestResult;

        fn test_with_tee_ta_ctx() {
            let mut test_only: u32 = 0;
            with_tee_ta_ctx(|ta_ctx| {
                test_only = ta_ctx.for_test_only;
                Ok(())
            }).unwrap();
            
            let mut new_value = 0;
            with_tee_ta_ctx_mut(|ta_ctx| {
                ta_ctx.for_test_only = test_only + 1;
                new_value = ta_ctx.for_test_only;
                Ok(())
            }).unwrap();
            assert_eq!(new_value, test_only + 1);

            new_value = 0;
            with_tee_ta_ctx(|ta_ctx| {
                new_value = ta_ctx.for_test_only;
                Ok(())
            }).unwrap();
            assert_eq!(new_value, test_only + 1);
        }
    }

    tests_name! {
        TEST_TEE_SESSION;
        //------------------------
        test_tee_ta_ctx,
        test_with_tee_ta_ctx,
    }
}
