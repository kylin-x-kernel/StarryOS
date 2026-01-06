use tee_raw_sys::TEE_ERROR_NOT_IMPLEMENTED;

use super::crypto::CryptoMacCtx;
use crate::tee::TeeResult;
pub struct SM3HmacCtx;
impl CryptoMacCtx for SM3HmacCtx {
    // Initialize the hash context
    fn init(&mut self, key: &[u8]) -> TeeResult {
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }

    // Update the hash context with data
    fn update(&mut self, data: &[u8]) -> TeeResult {
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }

    // Finalize the hash computation and return the digest
    fn r#final(&mut self, digest: &mut [u8]) -> TeeResult {
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }

    // Free the hash context resources
    fn free_ctx(self) {
        unimplemented!("Not implemented")
    }

    // Copy the state from one context to another
    fn copy_state(&mut self, ctx: &dyn CryptoMacCtx) {
        unimplemented!("Not implemented")
    }

    // fn get_ops(&self) -> &dyn CryptoHashOps {
    // }
    // fn get_ops_mut(&mut self) -> &mut dyn CryptoHashOps {
    // }
    // fn get_ops_ptr(&self) -> *const dyn CryptoHashOps {
    // }
    // fn get_ops_ptr_mut(&mut self) -> *mut dyn CryptoHashOps {
    // }
}
