use crate::tee::user_mode_ctx_struct::user_mode_ctx;
use crate::tee::TeeResult;

pub fn vm_check_access_rights(_uctx: &mut user_mode_ctx, _flags: u32, uaddr: usize, len: usize) -> TeeResult {
    Ok(())
}