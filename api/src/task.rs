use core::{ffi::c_long, sync::atomic::Ordering};

use axerrno::{AxError, AxResult};
use axhal::uspace::{ExceptionKind, ReturnReason, UserContext};
use axtask::{TaskInner, current};
use bytemuck::AnyBitPattern;
use linux_raw_sys::general::ROBUST_LIST_LIMIT;
use starry_core::{
    futex::FutexKey,
    mm::access_user_memory,
    shm::SHM_MANAGER,
    task::{
        AsThread, get_process_data, get_task, send_signal_to_process, send_signal_to_thread,
        set_timer_state,
    },
    time::TimerState,
};
use starry_process::Pid;
use starry_signal::{SignalInfo, Signo};
use starry_vm::{VmMutPtr, VmPtr};

use crate::{
    signal::{check_signals, unblock_next_signal},
    syscall::handle_syscall,
};

/// Create a new user task.
pub fn new_user_task(
    name: &str,
    mut uctx: UserContext,
    set_child_tid: Option<&'static mut Pid>,
) -> TaskInner {
    TaskInner::new(
        move || {
            let curr = axtask::current();
            access_user_memory(|| {
                if let Some(tid) = set_child_tid {
                    *tid = curr.id().as_u64() as Pid;
                }
            });

            info!("Enter user space: ip={:#x}, sp={:#x}", uctx.ip(), uctx.sp());

            let thr = curr.as_thread();
            while !thr.pending_exit() {
                let reason = uctx.run();

                set_timer_state(&curr, TimerState::Kernel);

                match reason {
                    ReturnReason::Syscall => handle_syscall(&mut uctx),
                    ReturnReason::PageFault(addr, flags) => {
                        if !thr.proc_data.aspace.lock().handle_page_fault(addr, flags) {
                            info!(
                                "{:?}: segmentation fault at {:#x} {:?}",
                                thr.proc_data.proc, addr, flags
                            );
                            raise_signal_fatal(SignalInfo::new_kernel(Signo::SIGSEGV))
                                .expect("Failed to send SIGSEGV");
                        }
                    }
                    ReturnReason::Interrupt => {}
                    #[allow(unused_labels)]
                    ReturnReason::Exception(exc_info) => 'exc: {
                        // TODO: detailed handling
                        let signo = match exc_info.kind() {
                            ExceptionKind::Misaligned => {
                                #[cfg(target_arch = "loongarch64")]
                                if unsafe { uctx.emulate_unaligned() }.is_ok() {
                                    break 'exc;
                                }
                                Signo::SIGBUS
                            }
                            ExceptionKind::Breakpoint => Signo::SIGTRAP,
                            ExceptionKind::IllegalInstruction => Signo::SIGILL,
                            _ => Signo::SIGTRAP,
                        };
                        raise_signal_fatal(SignalInfo::new_kernel(signo))
                            .expect("Failed to send SIGTRAP");
                    }
                    r => {
                        warn!("Unexpected return reason: {r:?}");
                        raise_signal_fatal(SignalInfo::new_kernel(Signo::SIGSEGV))
                            .expect("Failed to send SIGSEGV");
                    }
                }

                if !unblock_next_signal() {
                    while check_signals(thr, &mut uctx, None) {}
                }

                // Check if process is stopped and block until continued
                if thr.proc_data.proc.is_stopped() {
                    use core::{future::poll_fn, task::Poll};

                    use axtask::future::block_on;

                    info!(
                        "Task {} blocked (process {} stopped)",
                        curr.id().as_u64(),
                        thr.proc_data.proc.pid()
                    );

                    block_on(poll_fn(|cx| {
                        if !thr.proc_data.proc.is_stopped() {
                            Poll::Ready(())
                        } else {
                            thr.proc_data.child_exit_event.register(cx.waker());
                            Poll::Pending
                        }
                    }));

                    info!(
                        "Task {} resumed (process {} continued)",
                        curr.id().as_u64(),
                        thr.proc_data.proc.pid()
                    );

                    // Once resumed, the process is in Continued state, which
                    // will be reported to parent via
                    // waitpid(WCONTINUED). The process continues
                    // execution normally until parent acknowledges.
                }

                if !unblock_next_signal() {
                    while check_signals(thr, &mut uctx, None) {}
                }

                set_timer_state(&curr, TimerState::User);
                // Clear interrupt state
                let _ = curr.interrupted();
            }
        },
        name.into(),
        starry_core::config::KERNEL_STACK_SIZE,
    )
}

#[repr(C)]
#[derive(Debug, Copy, Clone, AnyBitPattern)]
pub struct RobustList {
    pub next: *mut RobustList,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, AnyBitPattern)]
pub struct RobustListHead {
    pub list: RobustList,
    pub futex_offset: c_long,
    pub list_op_pending: *mut RobustList,
}

fn handle_futex_death(entry: *mut RobustList, offset: i64) -> AxResult<()> {
    let address = (entry as u64)
        .checked_add_signed(offset)
        .ok_or(AxError::InvalidInput)?;
    let address: usize = address.try_into().map_err(|_| AxError::InvalidInput)?;
    let key = FutexKey::new_current(address);

    let curr = current();
    let futex_table = curr.as_thread().proc_data.futex_table_for(&key);

    let Some(futex) = futex_table.get(&key) else {
        return Ok(());
    };
    futex.owner_dead.store(true, Ordering::SeqCst);
    futex.wq.wake(1, u32::MAX);
    Ok(())
}

pub fn exit_robust_list(head: *const RobustListHead) -> AxResult<()> {
    // Reference: https://elixir.bootlin.com/linux/v6.13.6/source/kernel/futex/core.c#L777

    let mut limit = ROBUST_LIST_LIMIT;

    let end_ptr = unsafe { &raw const (*head).list };
    let head = head.vm_read()?;
    let mut entry = head.list.next;
    let offset = head.futex_offset;
    let pending = head.list_op_pending;

    while !core::ptr::eq(entry, end_ptr) {
        let next_entry = entry.vm_read()?.next;
        if entry != pending {
            handle_futex_death(entry, offset)?;
        }
        entry = next_entry;

        limit -= 1;
        if limit == 0 {
            return Err(AxError::FilesystemLoop);
        }
        axtask::yield_now();
    }

    Ok(())
}

/// Terminates the current thread and potentially the entire process.
///
/// This function handles the cleanup for a thread that is exiting. If this is the
/// last thread in the process, it will also handle the full process termination.
///
/// A thread can exit for several reasons:
/// - It called the `exit` or `exit_group` syscall.
/// - It was terminated by a fatal signal.
///
/// The termination procedure involves several steps:
///
/// 1. **TID Futex Cleanup**: If the `clear_child_tid` attribute was set by `clone()`,
///    this function writes 0 to the specified user-space address and wakes up any
///    threads waiting on that futex. This is used to notify a parent thread when
///    a child thread exits.
///
/// 2. **Robust Futex Cleanup**: It processes the robust futex list, releasing any
///    futexes held by the exiting thread to prevent deadlocks.
///
/// 3. **Thread Exit**: It marks the current thread as exited within the process's
///    thread list.
///
/// 4. **Process Termination (if last thread)**: If this is the last thread, it
///    triggers process-wide cleanup:
///    - Sets the final exit code or signal status of the process.
///    - Notifies the parent process by sending `SIGCHLD` and waking up any
///      `waitpid` calls.
///    - Cleans up resources like shared memory.
///
/// 5. **Group Exit**: If `group_exit` is true, it ensures all other threads in the
///    same process are also terminated by sending them `SIGKILL`.
///
/// 6. **Task Scheduling**: Finally, it marks the current kernel task as exited,
///    so the scheduler will not run it anymore and can deallocate its resources.
///
/// # Arguments
/// * `exit_code` - The exit code of the process, or the signal number if terminated by a signal.
/// * `group_exit` - If `true`, terminate all threads in the process's thread group.
/// * `signal` - If the process was terminated by a signal, this contains the signal number.
/// * `core_dumped` - If `true`, indicates that a core dump was generated.
pub fn do_exit(exit_code: i32, group_exit: bool, signal: Option<Signo>, core_dumped: bool) {
    let curr = current();
    let thr = curr.as_thread();

    info!(
        "{:?} exit with code: {}, signal: {:?}, core_dumped: {}",
        thr.proc_data.proc, exit_code, signal, core_dumped
    );

    let clear_child_tid = thr.clear_child_tid() as *mut u32;
    if clear_child_tid.vm_write(0).is_ok() {
        let key = FutexKey::new_current(clear_child_tid as usize);
        let table = thr.proc_data.futex_table_for(&key);
        let guard = table.get(&key);
        if let Some(futex) = guard {
            futex.wq.wake(1, u32::MAX);
        }
        axtask::yield_now();
    }
    let head = thr.robust_list_head() as *const RobustListHead;
    if !head.is_null()
        && let Err(err) = exit_robust_list(head)
    {
        warn!("exit robust list failed: {err:?}");
    }

    let process = &thr.proc_data.proc;
    if process.exit_thread(curr.id().as_u64() as Pid, exit_code) {
        if let Some(signo) = signal {
            process.exit_with_signal(signo as i32, core_dumped);
        } else {
            process.exit();
        }
        if let Some(parent) = process.parent() {
            if let Some(signo) = thr.proc_data.exit_signal {
                let _ = send_signal_to_process(parent.pid(), Some(SignalInfo::new_kernel(signo)));
            }
            if let Ok(data) = get_process_data(parent.pid()) {
                data.child_exit_event.wake();
            }
        }
        thr.proc_data.exit_event.wake();

        SHM_MANAGER.lock().clear_proc_shm(process.pid());
    }
    if group_exit && !process.is_group_exited() {
        process.group_exit();
        let sig = SignalInfo::new_kernel(Signo::SIGKILL);
        for tid in process.threads() {
            let _ = send_signal_to_thread(None, tid, Some(sig.clone()));
        }
    }
    thr.set_exit();
}

/// Sends a fatal signal to the current process.
pub fn raise_signal_fatal(sig: SignalInfo) -> AxResult<()> {
    let curr = current();
    let proc_data = &curr.as_thread().proc_data;

    let signo = sig.signo();
    info!("Send fatal signal {signo:?} to the current process");
    if let Some(tid) = proc_data.signal.send_signal(sig)
        && let Ok(task) = get_task(tid)
    {
        task.interrupt();
    } else {
        // No task wants to handle the signal, abort the task
        do_exit(128 + signo as i32, true, None, false);
    }

    Ok(())
}

/// Stops the current process in response to a signal.
///
/// This function is called when a stop signal (like `SIGSTOP`, `SIGTSTP`, etc.)
/// is delivered to the process. It sets the process state to "stopped" and
/// notifies the parent process. The parent can then inspect the stopped child
/// via `waitpid()` with the `WUNTRACED` option.
///
/// The task will block in the main task loop until a `SIGCONT` is received.
///
/// # Arguments
/// * `stop_signal` - The signal number that caused the process to stop.
pub fn do_stop(stop_signal: i32) {
        let curr = current();
    let curr_thread = curr.as_thread();
    let curr_process = &curr_thread.proc_data.proc;

    info!(
        "Process {} stopping due to signal {}",
        curr_process.pid(),
        stop_signal
    );
    curr_process.stop_by_signal(stop_signal);

    if let Some(parent) = curr_process.parent()
        && let Ok(data) = get_process_data(parent.pid())
    {
        data.child_exit_event.wake();
    }
}

/// Resumes a stopped process in response to `SIGCONT`.
///
/// This function is called when a `SIGCONT` signal is delivered to a stopped
/// process. It sets the process state back to "running" and notifies the parent
/// process (if it's waiting with `WCONTINUED`).
///
/// It also wakes up any threads of the process that were blocked in the main
/// task loop, allowing them to resume execution.
/// 
/// Notice that the state of the process after being continued are not guranteed.
/// There is no auto-restart mechanism after a syscall has been interrupted.
pub fn do_continue() {
    let curr = current();
    let curr_thread = curr.as_thread();
    let curr_process = &curr_thread.proc_data.proc;

    info!(
        "Process {} continuing from stopped state",
        curr_process.pid()
    );
    curr_process.continue_from_stop();
    curr_thread.proc_data.child_exit_event.wake();

    if let Some(parent) = curr_process.parent()
        && let Ok(data) = get_process_data(parent.pid())
    {
        data.child_exit_event.wake();
    }
}
