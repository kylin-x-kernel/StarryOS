use alloc::vec::Vec;
use core::{future::poll_fn, task::Poll};

use axerrno::{AxError, AxResult, LinuxError};
use axtask::{
    current,
    future::{block_on, interruptible},
};
use bitflags::bitflags;
use linux_raw_sys::general::{
    __WALL, __WCLONE, __WNOTHREAD, WCONTINUED, WEXITED, WNOHANG, WNOWAIT, WUNTRACED,
};
use starry_core::task::AsThread;
use starry_process::{Pid, Process, ProcessState};
use starry_vm::{VmMutPtr, VmPtr};

use crate::syscall::task::wait_status::WaitStatus;

bitflags! {
    #[derive(Debug)]
    struct WaitOptions: u32 {
        /// Do not block when there are no processes wishing to report status.
        const WNOHANG = WNOHANG;
        /// Report the status of selected processes which are stopped due to a
        /// `SIGTTIN`, `SIGTTOU`, `SIGTSTP`, or `SIGSTOP` signal.
        const WUNTRACED = WUNTRACED;
        /// Report the status of selected processes which have terminated.
        const WEXITED = WEXITED;
        /// Report the status of selected processes that have continued from a
        /// job control stop by receiving a `SIGCONT` signal.
        const WCONTINUED = WCONTINUED;
        /// Don't reap, just poll status.
        const WNOWAIT = WNOWAIT;

        /// Don't wait on children of other threads in this group
        const WNOTHREAD = __WNOTHREAD;
        /// Wait on all children, regardless of type
        const WALL = __WALL;
        /// Wait for "clone" children only.
        const WCLONE = __WCLONE;
    }
}

#[derive(Debug, Clone, Copy)]
enum WaitPid {
    /// Wait for any child process
    Any,
    /// Wait for the child whose process ID is equal to the value.
    Pid(Pid),
    /// Wait for any child process whose process group ID is equal to the value.
    Pgid(Pid),
}

impl WaitPid {
    fn apply(&self, child: &Process) -> bool {
        match self {
            WaitPid::Any => true,
            WaitPid::Pid(pid) => child.pid() == *pid,
            WaitPid::Pgid(pgid) => child.group().pgid() == *pgid,
        }
    }
}

/// Waits for a child process to change state.
///
/// This function implements the `wait4` and `waitpid` syscalls. It suspends the
/// execution of the current process until a child specified by `pid` has changed
/// state.
///
/// # Arguments
/// * `pid` - Specifies the set of child processes to wait for:
///   - `< -1`: Wait for any child process whose process group ID is equal to the
///     absolute value of `pid`.
///   - `-1`: Wait for any child process.
///   - `0`: Wait for any child process whose process group ID is equal to that of
///     the calling process.
///   - `> 0`: Wait for the child whose process ID is equal to `pid`.
/// * `exit_code` - A pointer to an integer where the status information of the
///   terminated child will be stored. The status can be inspected with macros
///   like `WIFEXITED`, `WIFSIGNALED`, etc.
/// * `options` - A bitmask of flags that modify the behavior of the call.
///
/// # State Changes
/// `sys_waitpid` waits for children that have:
/// - **Terminated (Zombie)**: The child has exited. Its PID and exit status are
///   returned. Unless `WNOWAIT` is set, the child process is reaped (its kernel
///   data structures are freed).
/// - **Stopped**: If `WUNTRACED` is set, it returns for children that have been
///   stopped by a signal. The status indicates the signal that caused the stop.
/// - **Continued**: If `WCONTINUED` is set, it returns for stopped children that
///   have been resumed by `SIGCONT`.
///
/// # Blocking Behavior
/// - By default, the call blocks until a child changes state.
/// - If `WNOHANG` is specified and no child has changed state, it returns `0`
///   immediately.
/// - The call can be interrupted by a signal, in which case it will return
///   `Err(AxError::Interrupted)`.
///
/// # Return Value
/// - On success, returns the process ID of the child that changed state.
/// - If `WNOHANG` was specified and no child has changed state, `0` is returned.
/// - On error, returns `Err(AxError)`. Common errors include:
///   - `ECHILD`: The process does not have any children to wait for.
///   - `EINTR`: The call was interrupted by a signal.
///   - `EINVAL`: Invalid options were provided.
pub fn sys_waitpid(pid: i32, exit_code: *mut i32, options: u32) -> AxResult<isize> {
    let options =
        WaitOptions::from_bits(options).ok_or(AxError::Other(LinuxError::EINVAL))?;
    info!("sys_waitpid <= pid: {pid:?}, options: {options:?}");

    // Currently, WNOTHREAD, WALL, and WCLONE are not supported.
    let unsupported = WaitOptions::WNOTHREAD | WaitOptions::WALL | WaitOptions::WCLONE;
    let requested_unsupported =
        WaitOptions::from_bits_truncate(options.bits() & unsupported.bits());
    if !requested_unsupported.is_empty() {
        warn!("waitpid: unsupported options {requested_unsupported:?}");
        return Err(AxError::Unsupported);
    }

    let curr = current();
    let proc_data = &curr.as_thread().proc_data;
    let proc = &proc_data.proc;

    let pid = if pid == -1 {
        WaitPid::Any
    } else if pid == 0 {
        WaitPid::Pgid(proc.group().pgid())
    } else if pid > 0 {
        WaitPid::Pid(pid as _)
    } else {
        WaitPid::Pgid(-pid as _)
    };

    // FIXME: add back support for WALL & WCLONE, since ProcessData may drop before
    // Process now.

    // Check that we have children to wait for
    let initial_children = proc
        .children()
        .into_iter()
        .filter(|child| pid.apply(child))
        .collect::<Vec<_>>();
    if initial_children.is_empty() {
        return Err(AxError::Other(LinuxError::ECHILD));
    }

    let check_children = || -> AxResult<Option<isize>> {
        // Re-fetch children on each check to get current state
        let children = proc
            .children()
            .into_iter()
            .filter(|child| pid.apply(child))
            .collect::<Vec<_>>();

        info!("sys_waitpid: checking {} children", children.len());

        if children.is_empty() {
            // All children have been reaped
            info!("sys_waitpid: no children, returning ECHILD");
            return Err(AxError::Other(LinuxError::ECHILD));
        }
        // When the WCONTINUED option is specified, check for continued children first.
        // This must come before zombie check because a process can be in Continued state
        // briefly before becoming a zombie (e.g., stopped process receives SIGCONT then exits).
        if options.contains(WaitOptions::WCONTINUED)
            && let Some(continued_child) = children.iter().find(|child| child.is_continued())
        {
            info!("sys_waitpid: found continued child {}", continued_child.pid());
            let wait_status = WaitStatus::continued();
            if let Some(exit_code_ptr) = exit_code.nullable() {
                let _ = exit_code_ptr.vm_write(wait_status.as_raw());
            }
            // Acknowledge that parent has been notified
            continued_child.ack_continued();
            return Ok(Some(continued_child.pid() as isize));
        }

        // When the WUNTRACED option is specified, also check for stopped children that haven't been acknowledged.
        // This should come before zombie check to catch stops before termination.
        // TODO: extend this to cover ptrace stop reporting once ptrace lands.
        if options.contains(WaitOptions::WUNTRACED)
            && let Some(stopped_child) = children.iter().find(|child| {
                // Only report stopped children that haven't been acknowledged yet
                matches!(child.state_snapshot(), ProcessState::Stopped { .. }) && child.stopped_unacked()
            })
            && let Some(stopping_signal) = stopped_child.get_stop_signal()
        {
            info!("sys_waitpid: found stopped child {} (signal {})", stopped_child.pid(), stopping_signal);
            let wait_status = WaitStatus::stopped(stopping_signal);
            if let Some(exit_code_ptr) = exit_code.nullable() {
                let _ = exit_code_ptr.vm_write(wait_status.as_raw());
            }
            // Acknowledge that parent has been notified of stop
            stopped_child.ack_stopped();
            return Ok(Some(stopped_child.pid() as isize));
        }

        // Check for any zombie children
        if let Some(child) = children.iter().find(|child| child.is_zombie()) {
            info!("sys_waitpid: found zombie child {}", child.pid());
            // Get zombie termination info before freeing
            let zombie_info = child.get_zombie_info().ok_or(AxError::Other(LinuxError::ECHILD))?;

            if !options.contains(WaitOptions::WNOWAIT) {
                child.free();
            }

            // Encode status based on how the process terminated
            let wait_status = if let Some(signo) = zombie_info.signal {
                WaitStatus::signaled(signo, zombie_info.core_dumped)
            } else {
                WaitStatus::exited(zombie_info.exit_code)
            };

            if let Some(exit_code_ptr) = exit_code.nullable() {
                let _ = exit_code_ptr.vm_write(wait_status.as_raw());
            }
            info!("sys_waitpid: returning pid {}", child.pid());
            return Ok(Some(child.pid() as isize));
        }

        // When WNOHANG is specified, return immediately if no children are ready
        if options.contains(WaitOptions::WNOHANG) {
            info!("sys_waitpid: WNOHANG set, no ready children, returning 0");
            return Ok(Some(0));
        }

        info!("sys_waitpid: no ready children, will block");
        Ok(None)
    };

    let result = block_on(interruptible(poll_fn(|cx| {
        // Register waker BEFORE checking to avoid lost wakeup race
        proc_data.child_exit_event.register(cx.waker());

        match check_children().transpose() {
            Some(res) => {
                info!("sys_waitpid: poll returning Ready");
                Poll::Ready(res)
            }
            None => {
                info!("sys_waitpid: poll returning Pending (will block)");
                Poll::Pending
            }
        }
    })))?;

    info!("sys_waitpid => {result:?}");
    result
}
