use core::sync::atomic::{AtomicBool, Ordering};

use axerrno::AxResult;
use axhal::uspace::UserContext;
use axtask::current;
use starry_core::task::{AsThread, Thread};
use starry_signal::{SignalOSAction, SignalSet};

use crate::task::do_exit;

pub fn check_signals(
    thr: &Thread,
    uctx: &mut UserContext,
    restore_blocked: Option<SignalSet>,
) -> bool {
    let Some((sig, os_action)) = thr.signal.check_signals(uctx, restore_blocked) else {
        return false;
    };

    let signo = sig.signo();
    match os_action {
        SignalOSAction::Terminate => {
            do_exit(signo as i32, true);
        }
        SignalOSAction::CoreDump => {
            // TODO: implement core dump
            do_exit(128 + signo as i32, true);
        }
        SignalOSAction::Stop => {
            // If under ptrace, emulate signal-delivery stop instead of exiting.
            // This is critical for PTRACE_TRACEME flows where the tracee raises SIGSTOP.
            #[cfg(feature = "ptrace")]
            {
                // Enter ptrace signal stop; tracer will resume us.
                starry_ptrace::signal_stop(signo as i32, uctx);
                return true;
            }
            // Fallback (no ptrace support): exit with SIGHUP-equivalent code for now.
            // TODO: implement proper job control stop (WIFSTOPPED) semantics.
            do_exit(1, true);
        }
        SignalOSAction::Continue => {
            // TODO: implement continue
        }
        SignalOSAction::Handler => {
            // do nothing
        }
    }
    true
}

static BLOCK_NEXT_SIGNAL_CHECK: AtomicBool = AtomicBool::new(false);

pub fn block_next_signal() {
    BLOCK_NEXT_SIGNAL_CHECK.store(true, Ordering::SeqCst);
}

pub fn unblock_next_signal() -> bool {
    BLOCK_NEXT_SIGNAL_CHECK.swap(false, Ordering::SeqCst)
}

pub fn with_replacen_blocked<R>(
    blocked: Option<SignalSet>,
    f: impl FnOnce() -> AxResult<R>,
) -> AxResult<R> {
    let curr = current();
    let sig = &curr.as_thread().signal;

    let old_blocked = blocked.map(|set| sig.set_blocked(set));
    f().inspect(|_| {
        if let Some(old) = old_blocked {
            sig.set_blocked(old);
        }
    })
}
