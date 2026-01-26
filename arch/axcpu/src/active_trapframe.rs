//! Active trapframe tracking.
//!
//! This module provides a tiny facility to expose the *current* trapframe
//! (register snapshot) to external callers (e.g. a pseudo-NMI watchdog).
//!
//! ## Design notes
//! - We only keep a single pointer per CPU *logically* (the most inner trap).
//! - The storage itself is a single atomic pointer. This is already useful on
//!   uniprocessor builds and is safe to call from interrupt/NMI-like contexts.
//! - If you need full per-CPU + nested trap support, this can be extended to a
//!   per-CPU stack later.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::TrapFrame;

/// Stores the pointer to the currently active trapframe.
///
/// 0 means no active trapframe.
#[percpu::def_percpu]
static ACTIVE_TRAPFRAME_PTR: AtomicUsize = AtomicUsize::new(0);

/// Returns the currently active trapframe, if any.
///
/// # Safety & lifetime
/// The returned reference is valid only while the CPU is still in the trap
/// context where the trapframe lives on the stack. Therefore, callers should
/// treat it as a short-lived snapshot: use it immediately and don't store it.
#[inline]
pub fn active_trap_frame() -> Option<&'static TrapFrame> {
    // Safety: caller context must tolerate best-effort snapshot.
    let ptr = unsafe {
        ACTIVE_TRAPFRAME_PTR
            .current_ref_raw()
            .load(Ordering::Relaxed)
    };

    if ptr == 0 {
        None
    } else {
        // SAFETY:
        // - pointer was installed from a valid &TrapFrame
        // - valid only while still in the trap context
        Some(unsafe { &*(ptr as *const TrapFrame) })
    }
}

/// Calls `f` with the currently active trapframe.
#[inline]
pub fn with_active_trap_frame<T>(f: impl FnOnce(Option<&TrapFrame>) -> T) -> T {
    f(active_trap_frame().map(|tf| tf as &TrapFrame))
}

/// A guard that exposes `tf` as the active trapframe within a scope.
///
/// This is intended to be used at the beginning of a trap handler function:
///
/// ```no_run
/// fn trap_handler(tf: &mut TrapFrame) {
///     let _guard = TrapFrameGuard::new(tf);
///     // ...
/// }
/// ```
pub struct TrapFrameGuard {
    prev: usize,
}

impl TrapFrameGuard {
    /// Sets `tf` as the active trapframe and returns a guard which will restore
    /// the previous value on drop.
    #[inline]
    pub fn new(tf: &TrapFrame) -> Self {
        let ptr = tf as *const TrapFrame as usize;

        let prev = unsafe {
            ACTIVE_TRAPFRAME_PTR
                .current_ref_raw()
                .swap(ptr, Ordering::Relaxed)
        };

        Self { prev }
    }
}

impl Drop for TrapFrameGuard {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            ACTIVE_TRAPFRAME_PTR
                .current_ref_raw()
                .store(self.prev, Ordering::Relaxed);
        }
    }
}