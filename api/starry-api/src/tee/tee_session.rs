// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, string::String, sync::Arc};
use core::{any::Any, default::Default};

use axtask::current;
use hashbrown::HashMap;
use spin::RwLock;
use starry_core::task::{AsThread, TeeSessionCtxTrait};
use tee_raw_sys::*;

use crate::tee::{TeeResult, tee_ta_manager::SessionIdentity};

scope_local::scope_local! {
    /// Global TEE TA (Trusted Application) context shared across all sessions
    /// This stores global state and resources for the TEE environment
    pub static TEE_TA_CTX: Arc<RwLock<TeeTaCtx>> = Arc::default();
}

/// TEE Session Context
/// This context stores per-session information and state for a client session
///
/// Parameters:
/// - session_id: Unique identifier for this session
/// - login_type: Type of login/authentication used for this session
/// - user_id: Identifier of the user associated with this session
///
/// This structure is attached to each thread handling a client session
pub struct TeeSessionCtx {
    pub session_id: u32,
    pub login_type: u32,
    pub user_id: u32,
    pub clnt_id: TEE_Identity,
    pub cancel: bool,
    pub cancel_mask: bool,
    pub cancel_time: TeeTime,
}

impl TeeSessionCtxTrait for TeeSessionCtx {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for TeeSessionCtx {
    fn default() -> Self {
        TeeSessionCtx {
            session_id: 0,
            login_type: 0,
            user_id: 0,
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
            cancel_time: TeeTime {
                seconds: 0,
                millis: 0,
            },
        }
    }
}

/// Acquire a mutable reference to the current thread's tee_session_ctx
/// Executes the provided closure with the mutable reference
///
/// # Parameters
/// - `f`: Closure that takes `&mut TeeSessionCtx` and returns `TeeResult<R>`
///
/// # Returns
/// The result of the closure execution
///
/// # Note
/// Creates a default session context if none exists for the current thread
pub fn with_tee_session_ctx_mut<F, R>(f: F) -> TeeResult<R>
where
    F: FnOnce(&mut TeeSessionCtx) -> TeeResult<R>,
{
    let current_task = current();
    current_task
        .as_thread()
        .set_tee_session_ctx(Box::new(TeeSessionCtx::default()));

    let binding = &current_task.as_thread().tee_session_ctx;
    let mut lock = binding.lock();

    let concrete = {
        let boxed = lock.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;
        boxed
            .as_any_mut()
            .downcast_mut::<TeeSessionCtx>()
            .ok_or(TEE_ERROR_BAD_STATE)?
    };

    f(concrete)
}

/// Acquire an immutable reference to the current thread's tee_session_ctx
/// Executes the provided closure with the immutable reference
///
/// # Parameters
/// - `f`: Closure that takes `&TeeSessionCtx` and returns `TeeResult<R>`
///
/// # Returns
/// The result of the closure execution
///
/// # Note
/// Creates a default session context if none exists for the current thread
pub fn with_tee_session_ctx<F, R>(f: F) -> TeeResult<R>
where
    F: FnOnce(&TeeSessionCtx) -> TeeResult<R>,
{
    let current_task = current();
    current_task
        .as_thread()
        .set_tee_session_ctx(Box::new(TeeSessionCtx::default()));

    let binding = &current_task.as_thread().tee_session_ctx;
    let lock = binding.lock();

    let concrete = {
        let boxed = lock.as_ref().ok_or(TEE_ERROR_BAD_STATE)?;
        boxed
            .as_any()
            .downcast_ref::<TeeSessionCtx>()
            .ok_or(TEE_ERROR_BAD_STATE)?
    };

    f(concrete)
}

/// TEE Trusted Application Context
/// This structure holds the global state for TA
/// All sessions in TA share this context
#[derive(Default, Debug)]
pub struct TeeTaCtx {
    /// Test-only field, used only when the tee_test feature is enabled
    #[cfg(feature = "tee_test")]
    pub for_test_only: u32,
    pub session_handle: u32,
    pub open_sessions: HashMap<u32, SessionIdentity>,
    pub uuid: String,
}

/// Acquire a mutable reference to the global tee_ta_ctx
/// Executes the provided closure with the mutable reference
/// The closure pattern ensures proper lock release
///
/// # Parameters
/// - `f`: Closure that takes `&mut tee_ta_ctx` and returns `TeeResult<R>`
///
/// # Returns
/// The result of the closure execution
pub fn with_tee_ta_ctx_mut<F, R>(f: F) -> TeeResult<R>
where
    F: FnOnce(&mut TeeTaCtx) -> TeeResult<R>,
{
    let mut ta_ctx = TEE_TA_CTX.write();
    f(&mut *ta_ctx)
}

/// Acquire an immutable reference to the global tee_ta_ctx
/// Executes the provided closure with the immutable reference
/// The closure pattern ensures proper lock release
///
/// # Parameters
/// - `f`: Closure that takes `&tee_ta_ctx` and returns `TeeResult<R>`
///
/// # Returns
/// The result of the closure execution
pub fn with_tee_ta_ctx<F, R>(f: F) -> TeeResult<R>
where
    F: FnOnce(&TeeTaCtx) -> TeeResult<R>,
{
    let ta_ctx = TEE_TA_CTX.read();
    f(&*ta_ctx)
}

// Test module for TEE session functionality
// Only compiled when the tee_test feature is enabled
#[cfg(feature = "tee_test")]
pub mod tests_tee_session {
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert_eq,
        tee::test::{test_framework::TestDescriptor, test_framework_basic::TestResult},
        test_fn, tests_name,
    };

    // Test function for basic tee_ta_ctx operations
    test_fn! {
        using TestResult;

        fn test_tee_ta_ctx() {
            // Test reading from TEE_TA_CTX
            let mut test_only: u32 = 0;
            {
                let ta_ctx = TEE_TA_CTX.read();
                test_only = ta_ctx.for_test_only;
            }

            // Test writing to TEE_TA_CTX
            {
                let mut ta_ctx = TEE_TA_CTX.write();
                ta_ctx.for_test_only = test_only + 1;
                assert_eq!(ta_ctx.for_test_only, test_only + 1);
            }

            // Read again to verify the write was successful
            {
                let ta_ctx = TEE_TA_CTX.read();
                assert_eq!(ta_ctx.for_test_only, test_only + 1);
            }
        }
    }

    // Test function for with_tee_ta_ctx helper functions
    test_fn! {
        using TestResult;

        fn test_with_tee_ta_ctx() {
            let mut test_only: u32 = 0;
            // Test with_tee_ta_ctx (immutable reference)
            with_tee_ta_ctx(|ta_ctx| {
                test_only = ta_ctx.for_test_only;
                Ok(())
            }).unwrap();

            let mut new_value = 0;
            // Test with_tee_ta_ctx_mut (mutable reference)
            with_tee_ta_ctx_mut(|ta_ctx| {
                ta_ctx.for_test_only = test_only + 1;
                new_value = ta_ctx.for_test_only;
                Ok(())
            }).unwrap();
            assert_eq!(new_value, test_only + 1);

            new_value = 0;
            // Verify the change persists
            with_tee_ta_ctx(|ta_ctx| {
                new_value = ta_ctx.for_test_only;
                Ok(())
            }).unwrap();
            assert_eq!(new_value, test_only + 1);
        }
    }

    // Test suite definition
    tests_name! {
        TEST_TEE_SESSION;
        //------------------------
        test_tee_ta_ctx,
        test_with_tee_ta_ctx,
    }
}
