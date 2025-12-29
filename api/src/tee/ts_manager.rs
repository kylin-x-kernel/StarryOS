// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
    boxed::Box,
};

use axnet::{
    RecvOptions, SendOptions, SocketAddrEx, SocketOps,
    unix::{StreamTransport, UnixSocket, UnixSocketAddr},
};
use axtask::current;
use bincode::config;
use starry_core::task::{AsThread, Thread};
use tee_raw_sys::{
    TEE_ERROR_GENERIC, TEE_ERROR_ITEM_NOT_FOUND, TEE_ErrorOrigin, TEE_SUCCESS, utee_params,
    TEE_UUID,
};

use crate::{
    socket::SocketAddrExt,
    tee::{
        TeeResult,
        protocol::{CARequest, CAResponse, Parameters, TARequest},
        tee_session::{with_tee_ta_ctx, with_tee_ta_ctx_mut},
        user_ta::FtraceBuf,
        tee_ta_manager::*,
    },
};

// Forward declaration for thread syscall registers
pub struct ThreadScallRegs;

// Context for Trusted Services
// #[derive(Debug)]
pub struct TsCtx<'a> {
    pub uuid: TEE_UUID,
    pub ops: Option<&'a dyn TsOps>,
}

// Session for Trusted Services
// #[derive(Debug)]
pub struct TsSession<'a> {
    pub ctx: Option<&'a TsCtx<'a>>,
    #[cfg(feature = "ta_gprof_support")]
    pub sbuf: Option<SampleBuf>, // Profiling data (PC sampling)
    #[cfg(feature = "ftrace_support")]
    pub fbuf: Option<FtraceBuf>, // ftrace buffer
    pub user_ctx: Option<&'a core::ffi::c_void>, // Used by PTAs to store session specific information
    pub handle_scall: Option<fn(&ThreadScallRegs) -> bool>,
}

// Trait defining operations for Trusted Services
pub trait TsOps//: Send + Sync
{
    /// Called when opening a session to the service
    fn enter_open_session(&self, session: &mut TsSession) -> TeeResult;

    /// Called when invoking a command in the service
    fn enter_invoke_cmd(&self, session: &mut TsSession, cmd: u32) -> TeeResult;

    /// Called when closing a session to the service
    fn enter_close_session(&self, session: &mut TsSession);

    /// Called to dump heap state of the service
    #[cfg(feature = "ta_stats")]
    fn dump_mem_stats(&self, session: &TsSession) -> TeeResult {
        Ok(())
    }

    /// Called to dump active memory mappings
    fn dump_state(&self, ctx: &TsCtx);

    /// Called to dump the ftrace data via RPC
    fn dump_ftrace(&self, ctx: &TsCtx);

    /// Called when the service has panicked and as much state as possible need to be released
    fn release_state(&self, ctx: &mut TsCtx);

    /// Called to free the TsCtx, removing all trace of the service
    fn destroy(&self, ctx: Box<dyn TsOps>);

    /// Called to get a unique ID of the service
    fn get_instance_id(&self, ctx: &TsCtx) -> u32;

    /// Called to handle a syscall from the service
    fn handle_scall(&self, regs: &ThreadScallRegs) -> bool;

    /// Called to update the gprof status of the service
    #[cfg(feature = "ta_gprof_support")]
    fn gprof_set_status(&self, status: TsGprofStatus);
}

// Public API functions
pub fn ts_get_current_session() -> TeeResult<TsSession<'static>> {
    unimplemented!();
}

pub fn ts_get_current_session_may_fail() -> Option<TsSession<'static>> {
    unimplemented!();
}

pub fn ts_push_current_session(session: TsSession) {
    unimplemented!();
}

pub fn ts_pop_current_session() -> Option<TsSession<'static>> {
    unimplemented!();
}

pub fn ts_get_calling_session() -> Option<TsSession<'static>> {
    unimplemented!();
}