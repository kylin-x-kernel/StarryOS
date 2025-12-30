use super::crypto::CryptoHashCtx;
use crate::tee::TeeResult;
use tee_raw_sys::TEE_ERROR_NOT_IMPLEMENTED;
pub struct SM3HashCtx;
impl CryptoHashCtx for SM3HashCtx {
    // Initialize the hash context
    fn init(&mut self) -> TeeResult {
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }

    // Update the hash context with data
    fn update(&mut self, data: &[u8]) -> TeeResult{
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }

    // Finalize the hash computation and return the digest
    fn r#final(&mut self, digest: &mut [u8]) -> TeeResult{
        Err(TEE_ERROR_NOT_IMPLEMENTED)
    }

    // Free the hash context resources
    fn free_ctx(self){
        unimplemented!("Not implemented")
    }

    // Copy the state from one context to another
    fn copy_state(&mut self, ctx: &dyn CryptoHashCtx){
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