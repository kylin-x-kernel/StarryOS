// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use core::default::Default;
use core::slice::IterMut;

use tee_raw_sys::TEE_ERROR_NOT_IMPLEMENTED;

use crate::tee::TeeResult;
// use crate::tee::ts_manager::TsCtx;

/// user ta context
/// NOTE: NEVER USE THIS STRUCT IN YOUR CODE
#[repr(C)]
pub struct user_ta_ctx {}

impl Default for user_ta_ctx {
    fn default() -> Self {
        Self {}
    }
}

impl Clone for user_ta_ctx {
    fn clone(&self) -> Self {
        Self {}
    }
}

pub const FTRACE_RETFUNC_DEPTH: usize = 50;

#[derive(Clone, Copy)]
pub union CompatPtr {
    pub ptr64: u64,
    pub ptr32: CompatPtr32,
}

#[derive(Debug, Clone, Copy)]
pub struct CompatPtr32 {
    pub lo: u32,
    pub hi: u32,
}

// #[derive(Debug)]
pub struct FtraceInfo {
    pub buf_start: CompatPtr,
    pub buf_end: CompatPtr,
    pub ret_ptr: CompatPtr,
}

#[derive(Debug)]
pub struct FtraceBuf {
    /// __ftrace_return pointer
    pub ret_func_ptr: u64,
    /// Return stack
    pub ret_stack: [u64; FTRACE_RETFUNC_DEPTH],
    /// Return stack index
    pub ret_idx: u32,
    /// lr index used for stack unwinding
    pub lr_idx: u32,
    /// Timestamp
    pub begin_time: [u64; FTRACE_RETFUNC_DEPTH],
    /// Suspend timestamp
    pub suspend_time: u64,
    /// Current entry in the (circular) buffer
    pub curr_idx: u32,
    /// Max allowed size of ftrace buffer
    pub max_size: u32,
    /// Ftrace buffer header offset
    pub head_off: u32,
    /// Ftrace buffer offset
    pub buf_off: u32,
    /// Some syscalls are never traced
    pub syscall_trace_enabled: bool,
    /// By foreign interrupt or RPC
    pub syscall_trace_suspended: bool,
    /// Circular buffer has wrapped
    pub overflow: bool,
}

impl FtraceBuf {
    pub fn new() -> Self {
        Self {
            ret_func_ptr: 0,
            ret_stack: [0; FTRACE_RETFUNC_DEPTH],
            ret_idx: 0,
            lr_idx: 0,
            begin_time: [0; FTRACE_RETFUNC_DEPTH],
            suspend_time: 0,
            curr_idx: 0,
            max_size: 0,
            head_off: 0,
            buf_off: 0,
            syscall_trace_enabled: false,
            syscall_trace_suspended: false,
            overflow: false,
        }
    }
}

impl user_ta_ctx {
    pub(crate) fn iter_mut(&mut self) -> TeeResult<&mut IterMut<'_, usize>> {
        // [0u8; 0].iter_mut()
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }
}

// 模拟 to_user_ta_ctx 函数的 Rust 版本
// pub(crate) fn to_user_ta_ctx<'a>(ctx: &'a TsCtx<'a>) -> TeeResult<&'a mut user_ta_ctx> {
//     // unsafe { core::mem::transmute(()) }
//     Err(TEE_ERROR_NOT_IMPLEMENTED)
// }