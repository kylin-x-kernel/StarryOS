use core::arch::asm;

use axhal::uspace::UserContext;
use syscalls::Sysno;
use tee_raw_sys::TEE_ERROR_NOT_SUPPORTED;

use crate::tee::{
    tee_generic::{sys_tee_scn_log, sys_tee_scn_panic, sys_tee_scn_return},
    tee_property::{sys_tee_scn_get_property, sys_tee_scn_get_property_name_to_index},
};

mod protocal;
mod tee_generic;
mod tee_property;
mod tee_session;
#[cfg(feature = "tee_test")]
pub mod test;
mod user_access;

pub type TeeResult<T = ()> = Result<T, u32>;

pub fn handle_tee_syscall(sysno: Sysno, uctx: &mut UserContext) -> TeeResult {
    // Handle TEE-specific syscalls here
    match sysno {
        Sysno::tee_scn_return => sys_tee_scn_return(uctx.arg0() as _),
        Sysno::tee_scn_log => sys_tee_scn_log(uctx.arg0() as _, uctx.arg1() as _),
        Sysno::tee_scn_panic => sys_tee_scn_panic(uctx.arg0() as _),
        Sysno::tee_scn_get_property => {
            let prop_type: usize;
            unsafe {
                asm!(
                    "mov {0}, x6",
                    out(reg) prop_type,
                );
            }
            sys_tee_scn_get_property(
                uctx.arg0() as _,
                uctx.arg1() as _,
                uctx.arg2() as _,
                uctx.arg3() as _,
                uctx.arg4() as _,
                uctx.arg5() as _,
                prop_type as _,
            )
        }
        Sysno::tee_scn_get_property_name_to_index => sys_tee_scn_get_property_name_to_index(
            uctx.arg0() as _,
            uctx.arg1() as _,
            uctx.arg2() as _,
            uctx.arg3() as _,
        ),
        _ => Err(TEE_ERROR_NOT_SUPPORTED),
    }
}
