use axhal::uspace::UserContext;
use syscalls::Sysno;
use tee_raw_sys::TEE_ERROR_NOT_SUPPORTED;

#[cfg(feature = "tee_test")]
use crate::tee::test::sys_tee_scn_test;

#[cfg(feature = "tee_test")]
mod test;

pub type TeeResult<T = ()> = Result<T, u32>;

pub fn handle_tee_syscall(sysno: Sysno, uctx: &mut UserContext) -> TeeResult {
    // Handle TEE-specific syscalls here
    match sysno {
        #[cfg(feature = "tee_test")]
        Sysno::tee_scn_test => sys_tee_scn_test(),
        _ => Err(TEE_ERROR_NOT_SUPPORTED),
    }
}
