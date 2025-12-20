
use tee_raw_sys::{TEE_OperationInfo, TEE_ObjectHandle};

/// 对应 C 的 __TEE_OperationHandle 结构体
#[repr(C)]
pub(self) struct OperationHandle {
    /// 操作信息
    pub info: TEE_OperationInfo,

    /// 第一个密钥对象句柄
    pub key1: TEE_ObjectHandle,

    /// 第二个密钥对象句柄（某些算法需要两个密钥）
    pub key2: TEE_ObjectHandle,

    /// 操作状态：INITIAL 或 ACTIVE
    pub operation_state: u32,

    /// 缓冲区：用于收集完整的块或保存完整的摘要，供 TEE_DigestExtract() 使用
    pub buffer: *mut u8,

    /// 如果为 true，表示需要缓冲两个块
    pub buffer_two_blocks: bool,

    /// 加密算法的块大小
    pub block_size: usize,

    /// 缓冲区中的偏移量
    pub buffer_offs: usize,

    /// TEE Core 中的状态句柄
    pub state: u32,
}

pub type TEEOperationHandle = *mut OperationHandle;

// --- 主 API 函数 ---

/// TEE Core API function to allocate a cryptographic operation.
///
/// This function is the public interface and must not change its signature.
#[cfg(feature = "api_crypto_tee_allocate_operation")]
#[no_mangle]
pub extern "C"
fn TEE_AllocateOperation(
    operation: *mut TEE_OperationHandle,
    algorithm: u32,
    mode: u32,
    max_key_size: u32,
) -> TEE_Result {
    // Validate input
    if operation.is_null() {
        unsafe { TEE_Panic(0) };
    }

    // 1. Check algorithm key size
    let res = check_algorithm_key_size(algorithm, max_key_size);
    if res != TEE_SUCCESS {
        return res;
    }

    // 2. Determine operation parameters
    let (req_key_usage, with_private_key, buffer_two_blocks, block_size_initial) =
        match determine_operation_params(algorithm, mode) {
            Ok(params) => params,
            Err(e) => return e,
        };

    // 3. Allocate and initialize handle
    let op_raw_ptr = match allocate_and_initialize_handle(
        algorithm,
        mode,
        max_key_size,
        req_key_usage,
        0, // handle_state is determined inside `allocate_and_initialize_handle`
        buffer_two_blocks,
        block_size_initial,
    ) {
        Ok(ptr) => ptr,
        Err(e) => return e,
    };

    // 4. Allocate transient objects
    let res = allocate_transient_objects(op_raw_ptr, algorithm, max_key_size, with_private_key);
    if res != TEE_SUCCESS {
        // Cleanup on error
        unsafe {
            let op = &mut *op_raw_ptr;
            if !op.buffer.is_null() {
                TEE_Free(op.buffer as *mut core::ffi::c_void);
            }
            // Note: key1/key2 are only partially set up, TEE_FreeTransientObject might panic
            // if called on invalid handles. A more robust approach in the C code relies on
            // TEE_FreeOperation or manual checks. Here we assume they are null or invalid
            // and TEE_Free handles null gracefully, or we need a safer way to track initialization.
            // For strict adherence, we mimic C: call TEE_Free on the struct itself.
            TEE_Free(op_raw_ptr as *mut core::ffi::c_void);
        }
        return res;
    }

    // 5. Initialize crypto state in TEE core
    let res = initialize_crypto_state(op_raw_ptr, algorithm, mode);
    if res != TEE_SUCCESS {
        // On error, use TEE_FreeOperation for comprehensive cleanup as per C logic
        // It handles freeing keys, buffer, and the op struct itself via _utee_cryp_state_free
        unsafe {
            TEE_FreeOperation(op_raw_ptr as TEE_OperationHandle);
        }
        return res;
    }

    // Success: Assign the created handle to the output parameter
    unsafe {
        *operation = op_raw_ptr as TEE_OperationHandle;
    }
    TEE_SUCCESS
}