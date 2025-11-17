use axhal::uspace::UserContext;
use syscalls::Sysno;
use tee_raw_sys::

mod time;

use time::*;

pub(crate) fn handle_tee_syscall(uctx: &mut UserContext, sysno: Sysno) {
    // Handle TEE-specific syscalls here
    match sysno {
        Sysno::tee_scn_get_time => {
            let timeval_ptr = uctx.arg1() as *mut TeeTimeval;
            let timezone_ptr = uctx.arg2() as *mut TeeTimezone;

            match get_time(timeval_ptr, timezone_ptr) {
                Ok(_) => {
                    uctx.set_return_value(0); // Success
                }
                Err(err) => {
                    uctx.set_return_value(err as usize); // Return error code
                }
            }
        }
        _ => {
            panic!("Unknown TEE syscall number: {}", syscall_num);
        }
    }
}
