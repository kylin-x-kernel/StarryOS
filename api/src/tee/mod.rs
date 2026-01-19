use axhal::uspace::UserContext;
use syscalls::Sysno;
use tee_raw_sys::TEE_ERROR_NOT_SUPPORTED;

use crate::tee::tee_generic::{sys_tee_scn_log, sys_tee_scn_panic, sys_tee_scn_return};

mod protocal;
mod tee_generic;
mod tee_session;
#[cfg(feature = "tee_test")]
pub mod test;

pub type TeeResult<T = ()> = Result<T, u32>;

pub fn handle_tee_syscall(sysno: Sysno, uctx: &mut UserContext) -> TeeResult {
    // Handle TEE-specific syscalls here
    match sysno {
        Sysno::tee_scn_return => sys_tee_scn_return(uctx.arg0() as _),
        Sysno::tee_scn_log => sys_tee_scn_log(uctx.arg0() as _, uctx.arg1() as _),
        Sysno::tee_scn_panic => sys_tee_scn_panic(uctx.arg0() as _),
        _ => Err(TEE_ERROR_NOT_SUPPORTED),
    }
}
