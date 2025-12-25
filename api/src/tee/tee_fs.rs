use core::ffi::c_uint;

use tee_raw_sys::*;

use super::tee_pobj::{REE_FS_OPS, TeeFileOperations};
use crate::tee::TeeResult;

#[repr(C)]
pub struct tee_file_handle;

// Returns the appropriate tee_file_operations for the specified storage ID.
// The value TEE_STORAGE_PRIVATE will select the REE FS if available, otherwise
// RPMB.
// 
// only support REE FS now
pub fn tee_svc_storage_file_ops(storage_id: c_uint) -> TeeResult<&'static TeeFileOperations> {
    match storage_id {
        TEE_STORAGE_PRIVATE => Ok(&REE_FS_OPS),
        TEE_STORAGE_PRIVATE_REE => Ok(&REE_FS_OPS),
        TEE_STORAGE_PRIVATE_RPMB => Err(TEE_ERROR_NOT_SUPPORTED),
        _ => Err(TEE_ERROR_BAD_PARAMETERS),
    }
}
