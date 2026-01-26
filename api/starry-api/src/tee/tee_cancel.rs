// Cancellation syscalls implementation for TEE using session-level state

use core::{ffi::c_uint, slice};

use axhal::time::wall_time;
use tee_raw_sys::TeeTime;

use crate::tee::{
    TeeResult,
    tee_session::{TeeSessionCtx, with_tee_session_ctx, with_tee_session_ctx_mut},
    user_access::copy_to_user,
};

/// TEE_GetCancellationFlag
/// Returns 1 if the session cancel flag is set and not masked, otherwise 0.
pub fn sys_tee_scn_get_cancellation_flag(cancel: *mut c_uint) -> TeeResult {
    let is_cancelled = with_tee_session_ctx(|ctx| Ok(tee_ta_session_is_cancelled(ctx, None)))?;
    let flag: u32 = if is_cancelled { 1 } else { 0 };
    copy_to_user(
        unsafe { slice::from_raw_parts_mut(cancel as _, size_of::<u32>()) },
        &flag.to_ne_bytes(),
        size_of::<u32>(),
    )?;
    Ok(())
}

/// TEE_UnmaskCancellation
/// Unmasks cancellation at session level; returns previous masked state (1 if masked before).
/// If unmasking reveals a pending cancellation, interrupt the current task so cancellable
/// functions can detect the flag.
pub fn sys_tee_scn_unmask_cancellation(old_mask: *mut c_uint) -> TeeResult {
    let prev = with_tee_session_ctx_mut(|ctx| {
        let prev = ctx.cancel_mask;
        ctx.cancel_mask = false;
        Ok(prev)
    })?;
    let prev_mask: u32 = if prev { 1 } else { 0 };
    copy_to_user(
        unsafe { slice::from_raw_parts_mut(old_mask as _, size_of::<u32>()) },
        &prev_mask.to_ne_bytes(),
        size_of::<u32>(),
    )?;
    Ok(())
}

/// TEE_MaskCancellation
/// Masks cancellation at session level; returns previous masked state (1 if masked before).
pub fn sys_tee_scn_mask_cancellation(old_mask: *mut c_uint) -> TeeResult {
    let prev = with_tee_session_ctx_mut(|ctx| {
        let prev = ctx.cancel_mask;
        ctx.cancel_mask = true;
        Ok(prev)
    })?;
    let prev_mask: u32 = if prev { 1 } else { 0 };
    copy_to_user(
        unsafe { slice::from_raw_parts_mut(old_mask as _, size_of::<u32>()) },
        &prev_mask.to_ne_bytes(),
        size_of::<u32>(),
    )?;
    Ok(())
}

fn tee_ta_session_is_cancelled(ctx: &TeeSessionCtx, curr_time: Option<&TeeTime>) -> bool {
    if ctx.cancel_mask {
        return false;
    }

    if ctx.cancel {
        return true;
    }

    if ctx.cancel_time.seconds == u32::MAX {
        return false;
    }

    let current_time = match curr_time {
        Some(time) => *time,
        None => tee_time_get_sys_time(),
    };

    if current_time.seconds > ctx.cancel_time.seconds
        || (current_time.seconds == ctx.cancel_time.seconds
            && current_time.millis >= ctx.cancel_time.millis)
    {
        return true;
    }
    false
}

fn tee_time_get_sys_time() -> TeeTime {
    let systiem = wall_time();
    TeeTime {
        seconds: systiem.as_secs() as u32,
        millis: systiem.subsec_millis(),
    }
}
