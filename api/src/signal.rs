use core::sync::atomic::{AtomicBool, Ordering};

use axerrno::AxResult;
use axhal::uspace::UserContext;
use axtask::current;
use starry_core::task::{AsThread, Thread};
use starry_signal::{SignalOSAction, SignalSet};
use axlog::info;

use crate::task::{do_continue, do_exit, do_stop};

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
            info!("{:?} terminated by signal {:?}", thr.proc_data.proc, signo);
            do_exit(128 + signo as i32, true, Some(signo), false);
        }
        SignalOSAction::CoreDump => {
            // TODO: implement core dump, 
            // now the core_dumped is set to true as a indication without actual core dump
            info!("{:?} core dumped by signal {:?}", thr.proc_data.proc, signo);
            do_exit(128 + signo as i32, true, Some(signo), true);
        }
        SignalOSAction::Stop => {
            info!("{:?} stopped by signal {:?}", thr.proc_data.proc, signo);
            do_stop(signo as i32);
        }
        SignalOSAction::Continue => {
            info!("{:?} continued by signal {:?}", thr.proc_data.proc, signo);
            do_continue();
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
