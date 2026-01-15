use axhal::uspace::UserContext;
use syscalls::Sysno;
use tee_raw_sys::TEE_ERROR_NOT_SUPPORTED;

#[cfg(feature = "tee_test")]
pub mod test;

pub type TeeResult<T = ()> = Result<T, u32>;

pub fn handle_tee_syscall(sysno: Sysno, uctx: &mut UserContext) -> TeeResult {
    // Handle TEE-specific syscalls here
    match sysno {
        _ => Err(TEE_ERROR_NOT_SUPPORTED),
    }
}
