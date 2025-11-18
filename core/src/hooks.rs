//! Extension hooks for optional OS features.
//!
//! Keep this file minimal and decoupled. It defines trait interfaces and a
//! tiny registry so optional components (like ptrace) can hook into core
//! behavior without adding dependencies or runtime cost when disabled.

use axhal::uspace::UserContext;
use axsync::Mutex;
use lazy_static::lazy_static;

/// Hook trait to observe syscall entry/exit.
///
/// Implementations should be lightweight and avoid blocking in the hook
/// methods. Any heavy logic should hand off to external code.
pub trait SyscallHook: Send + Sync {
    /// Called right after the kernel decodes the syscall number but before
    /// dispatching the syscall handler. The implementation may inspect or
    /// modify the user context registers.
    fn on_syscall_entry(&self, _uctx: &mut UserContext) {}

    /// Called right after the syscall handler computed the result and before
    /// the return value is written back to user space.
    fn on_syscall_exit(&self, _uctx: &mut UserContext) {}
}

lazy_static! {
    static ref SYSCALL_HOOK: Mutex<Option<&'static dyn SyscallHook>> = Mutex::new(None);
}

/// Register a global syscall hook. Only one hook is supported.
///
/// Returns Err(()) if a hook has already been registered.
pub fn register_syscall_hook(hook: alloc::boxed::Box<dyn SyscallHook>) -> Result<(), ()> {
    let hook: &'static dyn SyscallHook = alloc::boxed::Box::leak(hook);
    let mut guard = SYSCALL_HOOK.lock();
    if guard.is_some() {
        return Err(());
    }
    *guard = Some(hook);
    Ok(())
}

/// Get the registered syscall hook if any.
pub fn get_syscall_hook() -> Option<&'static dyn SyscallHook> {
    // Copy the leaked 'static pointer out of the guard to avoid borrowing the guard.
    SYSCALL_HOOK.lock().as_ref().copied()
}
