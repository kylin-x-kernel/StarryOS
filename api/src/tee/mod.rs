use axhal::uspace::UserContext;
use axerrno::{AxError, AxResult};
use syscalls::Sysno;

mod time;

use time::*;

pub(crate) fn handle_tee_syscall(_sysno: Sysno, _uctx: &mut UserContext) -> AxResult<isize>  {
    // Handle TEE-specific syscalls here
    Err(AxError::Unsupported)
}
