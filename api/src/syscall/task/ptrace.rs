use axerrno::AxResult;

pub fn sys_ptrace(request: u32, pid: i32, addr: usize, data: usize) -> AxResult<isize> {
    #[cfg(feature = "ptrace")]
    {
        // Delegate to the decoupled ptrace crate when feature is enabled.
        return starry_ptrace::do_ptrace(request, pid, addr, data);
    }

    #[cfg(not(feature = "ptrace"))]
    {
        // Gracefully report unsupported when ptrace feature is disabled.
        return Err(axerrno::AxError::Unsupported);
    }
}