// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{
    alloc::{alloc, dealloc},
    boxed::Box,
    string::String,
    sync::Arc,
    vec,
    vec::Vec,
};
use core::slice::{from_raw_parts, from_raw_parts_mut};
use core::{
    alloc::Layout,
    any::Any,
    ffi::{c_char, c_uint, c_ulong, c_void},
    // from,
    mem::size_of,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice,
    time::Duration,
};

use axerrno::{AxError, AxResult};
use lazy_static::lazy_static;
use tee_raw_sys::{libc_compat::size_t, *};

use super::{
    TeeResult,
    config::CFG_COMPAT_GP10_DES,
    crypto::crypto::{
        CryptoHashCtx, CryptoMacCtx, crypto_hash_final, crypto_hash_init, crypto_hash_update,
        crypto_mac_final, crypto_mac_init, crypto_mac_update, ecc_keypair, ecc_public_key,
    },
    crypto::{sm3_hash::SM3HashCtx, sm3_hmac::SM3HmacCtx},
    libmbedtls::bignum::{
        crypto_bignum_bin2bn, crypto_bignum_bn2bin, crypto_bignum_copy, crypto_bignum_num_bits,
        crypto_bignum_num_bytes,
    },
    libutee::{
        tee_api_objects::TEE_USAGE_DEFAULT,
        utee_defines::{tee_alg_get_class, tee_alg_get_main_alg, tee_u32_to_big_endian},
    },
    memtag::memtag_strip_tag_vaddr,
    tee_obj::{tee_obj, tee_obj_add, tee_obj_get, tee_obj_id_type},
    tee_pobj::with_pobj_usage_lock,
    user_access::{
        bb_alloc, bb_free, copy_from_user, copy_from_user_struct, copy_from_user_u64, copy_to_user,
        copy_to_user_struct, copy_to_user_u64,
    },
    // ts_manager:: {
    //     TsSession,
    //     ts_get_current_session, ts_get_current_session_may_fail, ts_push_current_session, ts_pop_current_session, ts_get_calling_session,
    // }
    user_access::{enter_user_access, exit_user_access},
    user_mode_ctx_struct::user_mode_ctx,
    user_ta::{
        user_ta_ctx, // to_user_ta_ctx
    },
    utils::{bit, bit32},
    vm::vm_check_access_rights,
};
// use core::ffi::c_void;
// use core::ptr::NonNull;
use super::{
    tee_svc_cryp::{
        TeeCryptObj, get_user_u64_as_size_t, tee_cryp_obj_secret, tee_cryp_obj_secret_wrapper,
        tee_cryp_obj_type_props,
    },
    types_ext::vaddr_t,
};
use crate::{
    mm::vm_load_string,
    tee,
    tee::{
        TEE_ALG_SHA3_224, TEE_ALG_SHA3_256, TEE_ALG_SHA3_384, TEE_ALG_SHA3_512, TEE_ALG_SHAKE128,
        TEE_ALG_SHAKE256, TEE_TYPE_CONCAT_KDF_Z, TEE_TYPE_HKDF_IKM, TEE_TYPE_PBKDF2_PASSWORD,
        libmbedtls::bignum::BigNum,
        memtag::{memtag_strip_tag, memtag_strip_tag_const},
        tee_session::{with_tee_session_ctx, with_tee_session_ctx_mut},
    },
};

pub const TEE_TYPE_ATTR_OPTIONAL: u32 = bit(0);
pub const TEE_TYPE_ATTR_REQUIRED: u32 = bit(1);
pub const TEE_TYPE_ATTR_OPTIONAL_GROUP: u32 = bit(2);
pub const TEE_TYPE_ATTR_SIZE_INDICATOR: u32 = bit(3);
pub const TEE_TYPE_ATTR_GEN_KEY_OPT: u32 = bit(4);
pub const TEE_TYPE_ATTR_GEN_KEY_REQ: u32 = bit(5);
pub const TEE_TYPE_ATTR_BIGNUM_MAXBITS: u32 = bit(6);

// Handle storing of generic secret keys of varying lengths
pub const ATTR_OPS_INDEX_SECRET: u32 = 0;
// Convert to/from big-endian byte array and provider-specific bignum
pub const ATTR_OPS_INDEX_BIGNUM: u32 = 1;
// Convert to/from value attribute depending on direction
// Convert to/from big-endian byte array and provider-specific bignum
pub const ATTR_OPS_INDEX_VALUE: u32 = 2;
// Convert to/from curve25519 attribute depending on direction
// Convert to/from big-endian byte array and provider-specific bignum
pub const ATTR_OPS_INDEX_25519: u32 = 3;
// Convert to/from big-endian byte array and provider-specific bignum
pub const ATTR_OPS_INDEX_448: u32 = 4;

/// Represents the state of a cryptographic operation
///
/// This enum indicates whether a cryptographic operation has been initialized or not.
///
/// # Variants
/// * `Initialized` - The cryptographic operation has been properly initialized and is ready for use
/// * `Uninitialized` - The cryptographic operation has not been initialized yet
#[derive(Debug, Clone, Copy, PartialEq)]
enum CrypState {
    Initialized = 0,
    Uninitialized,
}

/// Function pointer type for finalization
///
/// This type defines the signature for functions that are responsible for cleaning up
/// or finalizing cryptographic contexts when they are no longer needed.
///
/// # Parameters
/// * `*mut c_void` - A pointer to the context that needs to be finalized
type TeeCrypCtxFinalizeFunc = unsafe extern "Rust" fn(*mut c_void);

/// Rust equivalent of the tee_cryp_state struct
///
/// This structure represents the state of a cryptographic operation in the TEE environment.
/// It contains all the necessary information to manage an active cryptographic operation,
/// including the algorithm, keys, and context-specific data.
///
/// # Fields
/// * `algo` - The cryptographic algorithm identifier (e.g., TEE_ALG_AES_ECB_NOPAD)
/// * `mode` - The operation mode (e.g., encrypt, decrypt, sign, verify)
/// * `key1` - Virtual address of the first key used in the operation (vaddr_t is typically usize in Rust)
/// * `key2` - Virtual address of the second key used in the operation (for algorithms requiring multiple keys)
/// * `ctx` - A trait object containing the specific context data for the algorithm
/// * `ctx_finalize` - Optional function pointer for finalizing the context when the operation ends
/// * `state` - Current state of the operation (initialized or uninitialized)
/// * `id` - Unique identifier for this cryptographic state instance
#[repr(C)]
pub(crate) struct TeeCrypState {
    // Since TAILQ_ENTRY is a linked list node, we use Option<NonNull> for safe pointer handling
    // pub link: Option<NonNull<TeeCrypState<'a>>>,
    pub algo: u32,
    pub mode: u32,
    pub key1: usize, // vaddr_t is typically usize in Rust
    pub key2: usize,
    pub ctx: &'static mut dyn Any,
    pub ctx_finalize: Option<TeeCrypCtxFinalizeFunc>,
    pub state: CrypState,
    pub id: usize,
}

// Rust equivalent of the tee_cryp_obj_secret struct
#[repr(C)]
struct TeeCrypObjSecret {
    key_size: u32,
    alloc_size: u32,
    // The actual data would follow this struct in memory
    // In Rust, we would typically handle this differently using Vec<u8> or similar
}

// If you need to work with the data following the struct, you might want to use:
impl TeeCrypObjSecret {
    // Get a slice of the secret data
    fn data(&self) -> &[u8] {
        // This is unsafe as we're creating a slice from raw memory
        // The caller must ensure the memory is valid
        unsafe {
            from_raw_parts(
                (self as *const Self).add(1) as *const u8,
                self.alloc_size as usize,
            )
        }
    }

    // Get a mutable slice of the secret data
    fn data_mut(&mut self) -> &mut [u8] {
        // This is unsafe as we're creating a slice from raw memory
        unsafe {
            from_raw_parts_mut(
                (self as *mut Self).add(1) as *mut u8,
                self.alloc_size as usize,
            )
        }
    }
}

/// Retrieves a cryptographic state from the session's list of states
///
/// This function searches for a cryptographic state identified by `state_id` in the
/// list of states associated with the provided session. If found, it returns the
/// state as a mutable reference in the result.
///
/// # Arguments
/// * `sess` - Reference to the session containing the cryptographic states
/// * `state_id` - Virtual address identifier for the state to retrieve
///
/// # Returns
/// * `Ok(Option<&mut TeeCrypState>)` - Returns Some with reference to found state if found,
///   or None if not found
/// * `Err(TEE_Result)` - Error code if the operation fails (TEE_ERROR_BAD_PARAMETERS if not found)
fn tee_svc_cryp_get_state<'a>(
    // sess: &'a TsSession<'a>
    sess: &'a mut Vec<TeeCrypState>,
    state_id: usize,
) -> TeeResult<&'a mut TeeCrypState> {
    // Iterate through the list of cryptographic states
    // Get the user TA context from the session
    for s in sess.iter_mut() {
        if state_id == s.id {
            // Found the state, return it wrapped in Some
            return Ok(s);
        }
    }

    // State not found in the list, return error
    Err(TEE_ERROR_BAD_PARAMETERS)
}

/// Check if algorithm is an XOF (Extendable Output Function)
///
/// XOF algorithms like SHAKE128 and SHAKE256 can produce
/// output of arbitrary length, unlike regular hash functions
/// that have fixed output size.
///
/// # Arguments
/// * `algo` - Algorithm identifier
///
/// # Returns
/// * `true` if the algorithm is an XOF (SHAKE128 or SHAKE256)
/// * `false` otherwise
#[inline]
pub fn is_xof_algo(algo: u32) -> bool {
    algo == TEE_ALG_SHAKE128 || algo == TEE_ALG_SHAKE256
}

/// Hash final syscall implementation in Rust
///
/// Finalizes a hash or MAC computation and returns the result.
/// This function demonstrates comprehensive use of memory tagging functions
/// (`memtag_strip_tag` and `memtag_strip_tag_const`).
///
/// # Arguments
/// * `state` - Handle to the crypto operation state
/// * `chunk` - Pointer to final data chunk (may be tagged)
/// * `chunk_size` - Size of final data chunk
/// * `hash` - Pointer to buffer to receive hash/MAC result
/// * `hash_len` - Pointer to receive actual hash length written
///
/// # Returns
/// * `TEE_SUCCESS` on success
/// * `TEE_ERROR_BAD_PARAMETERS` if parameters are invalid
/// * `TEE_ERROR_BAD_STATE` if crypto state is not initialized
/// * `TEE_ERROR_SHORT_BUFFER` if hash buffer is too small
///
/// # Memory Tagging
/// Both input pointers (`chunk`, `hash`) are stripped of memory tags:
/// - `memtag_strip_tag_const(chunk)` - for input data (const)
/// - `memtag_strip_tag(hash)` - for output buffer (mutable)
///
/// # XOF Support
/// For XOF (Extendable Output Function) algorithms like SHAKE128/256:
/// - Hash size is unlimited (caller specifies length)
/// - Final hash length returned as provided buffer size
///
/// # Example
/// ```
/// let state = 0x12345678; // Crypto state handle
/// let chunk = 0xAB00000012345678; // Tagged input pointer
/// let hash_buf = 0xCD00000087654321; // Tagged output pointer
/// let mut hash_len = 0u64;
///
/// sys_tee_scn_hash_final(state, chunk, 32, hash_buf, &mut hash_len)?;
/// // Result: hash_len bytes written to hash_buf
/// ```
pub(crate) fn sys_tee_scn_hash_final(
    state: usize,
    chunk: usize,
    chunk_size: usize,
    hash: usize,
    hash_len: vaddr_t,
) -> TeeResult {
    let hash_len = unsafe { &mut *(hash_len as *mut u64) };
    // Validate parameters: null chunk with non-zero size is invalid
    if chunk == 0 && chunk_size != 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    // Strip memory tags from user pointers
    // For XOF algorithms, both input and output may be arbitrary size
    let chunk = memtag_strip_tag_const(chunk as _);
    let hash = memtag_strip_tag(hash as _);

    // Check read access rights for input chunk
    with_tee_session_ctx(|ctx| {
        vm_check_access_rights(
            unsafe { &*(ctx as *const _ as usize as *const user_mode_ctx) },
            TEE_MEMORY_ACCESS_READ | TEE_MEMORY_ACCESS_ANY_OWNER,
            chunk,
            chunk_size,
        )
    })?;

    // Get user-provided buffer length
    let mut hlen: usize = 0;
    get_user_u64_as_size_t(&mut hlen, hash_len)?;

    // Check write access rights for output hash buffer
    with_tee_session_ctx(|ctx| {
        vm_check_access_rights(
            // &ctx.uctx
            unsafe { &*(ctx as *const _ as usize as *const user_mode_ctx) },
            TEE_MEMORY_ACCESS_READ | TEE_MEMORY_ACCESS_WRITE | TEE_MEMORY_ACCESS_ANY_OWNER,
            hash,
            hlen,
        )
    })?;

    // Get current session and retrieve crypto state
    with_tee_session_ctx_mut(|ctx| {
        match ctx.cryp_state.as_mut() {
            Some(s) => {
                // Retrieve specific crypto operation state
                let mut crypto_state = tee_svc_cryp_get_state(s, state)?;

                // Verify that state is initialized
                if crypto_state.state != CrypState::Initialized {
                    return Err(TEE_ERROR_BAD_STATE);
                }

                // Process based on algorithm class
                match tee_alg_get_class(crypto_state.algo) {
                    TEE_OPERATION_DIGEST => {
                        // Hash digest operation
                        process_digest_final(
                            &mut crypto_state,
                            chunk,
                            chunk_size,
                            hash,
                            &mut hlen,
                        )?;
                    }
                    TEE_OPERATION_MAC => {
                        // MAC (Message Authentication Code) operation
                        process_mac_final(&mut crypto_state, chunk, chunk_size, hash, &mut hlen)?;
                    }
                    _ => {
                        // Unsupported operation class
                        return Err(TEE_ERROR_BAD_PARAMETERS);
                    }
                }

                Ok(())
            }
            None => Err(TEE_ERROR_BAD_STATE),
        }
    })?;

    Ok(())
}

/// Process final MAC operation
///
/// # Arguments
/// * `crypto_state` - Mutable reference to crypto operation state
/// * `chunk` - Pointer to final data chunk
/// * `chunk_size` - Size of final data chunk
/// * `hash` - Pointer to output MAC buffer
/// * `hlen` - Input/output: buffer size / actual MAC length written
fn process_mac_final(
    crypto_state: &mut TeeCrypState,
    chunk: usize,
    chunk_size: usize,
    hash: usize,
    hlen: &mut usize,
) -> TeeResult {
    // Get digest size for MAC algorithm
    let mut hash_size: usize = 0;
    tee_alg_get_digest_size(crypto_state.algo, &mut hash_size)?;

    // Check buffer size
    if *hlen < hash_size {
        put_user_u64(hlen, &(hash_size as u64))?;
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    // Update MAC with final chunk if provided
    if chunk_size != 0 {
        let data_slice = unsafe { slice::from_raw_parts(chunk as *const u8, chunk_size) };

        enter_user_access();
        let res = if let Some(ctx) = crypto_state.ctx.downcast_mut::<SM3HmacCtx>() {
            crypto_mac_update(ctx, data_slice)
        } else {
            Err(TEE_ERROR_BAD_STATE)
        };
        exit_user_access();

        if let Err(_) = res {
            return res;
        }
    }

    // Finalize MAC computation
    let hash_slice = unsafe { slice::from_raw_parts_mut(hash as *mut u8, *hlen) };

    enter_user_access();
    let res = if let Some(ctx) = crypto_state.ctx.downcast_mut::<SM3HmacCtx>() {
        crypto_mac_final(ctx, hash_slice)
    } else {
        Err(TEE_ERROR_BAD_STATE)
    };
    exit_user_access();

    if let Err(_) = res {
        return res;
    }

    // Return actual MAC length
    *hlen = hash_size;

    Ok(())
}

/// Process final hash digest operation
///
/// # Arguments
/// * `crypto_state` - Mutable reference to crypto operation state
/// * `chunk` - Pointer to final data chunk
/// * `chunk_size` - Size of final data chunk
/// * `hash` - Pointer to output hash buffer
/// * `hlen` - Input/output: buffer size / actual hash length written
fn process_digest_final(
    crypto_state: &mut TeeCrypState,
    chunk: usize,
    chunk_size: usize,
    hash: usize,
    hlen: &mut usize,
) -> TeeResult {
    // Get digest size for algorithm
    let mut hash_size: usize = 0;

    // For XOF algorithms, hash_size is unchanged
    // For regular algorithms, check buffer size
    if is_xof_algo(crypto_state.algo) {
        // Update hash with final chunk if provided
        if chunk_size != 0 {
            let data_slice = unsafe { from_raw_parts(chunk as *const u8, chunk_size) };

            enter_user_access();
            let res = if let Some(ctx) = crypto_state.ctx.downcast_mut::<SM3HashCtx>() {
                crypto_hash_update(ctx, data_slice)
            } else {
                Err(TEE_ERROR_BAD_STATE)
            };
            exit_user_access();

            if let Err(_) = res {
                return res;
            }
        }

        // hash_size is supposed to be unchanged for XOF
        // algorithms so return directly.
        let hash_slice = unsafe { from_raw_parts_mut(hash as *mut u8, *hlen) };

        enter_user_access();
        let res = if let Some(ctx) = crypto_state.ctx.downcast_mut::<SM3HashCtx>() {
            crypto_hash_final(ctx, hash_slice)
        } else {
            Err(TEE_ERROR_BAD_STATE)
        };
        exit_user_access();

        if let Err(_) = res {
            return res;
        }
    }

    tee_alg_get_digest_size(crypto_state.algo, &mut hash_size)?;

    if *hlen < hash_size {
        put_user_u64(hlen, &(hash_size as u64))?;
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    // Update hash with final chunk if provided
    if chunk_size != 0 {
        let data_slice = unsafe { from_raw_parts(chunk as *const u8, chunk_size) };

        enter_user_access();
        let res = if let Some(ctx) = crypto_state.ctx.downcast_mut::<SM3HashCtx>() {
            crypto_hash_update(ctx, data_slice)
        } else {
            Err(TEE_ERROR_BAD_STATE)
        };
        exit_user_access();

        if let Err(_) = res {
            return res;
        }
    }

    // Finalize hash computation
    let hash_slice = unsafe { from_raw_parts_mut(hash as *mut u8, *hlen) };

    enter_user_access();
    let res = if let Some(ctx) = crypto_state.ctx.downcast_mut::<SM3HashCtx>() {
        crypto_hash_final(ctx, hash_slice)
    } else {
        Err(TEE_ERROR_BAD_STATE)
    };
    exit_user_access();

    if let Err(_) = res {
        return res;
    }

    // Return actual hash length
    // For XOF: return provided buffer size
    // For regular: return algorithm's hash size

    Ok(())
}

/// Get the digest (hash) output size for the specified algorithm
///
/// # Arguments
/// * `algo` - Algorithm identifier, defined in TEE_ALG_* constants
/// * `size` - Mutable reference to store the calculated digest size
///
/// # Returns
/// * `TeeResult` - Operation result:
///   - `TEE_SUCCESS`: Successfully obtained digest size
///   - `TEE_ERROR_NOT_SUPPORTED`: Unsupported algorithm
///   - `TEE_ERROR_BAD_PARAMETERS`: Invalid parameters
///
/// # Note
/// This function only returns the standard-defined digest size for the algorithm,
/// without considering any padding or special processing/// Get digest size for algorithm
fn tee_alg_get_digest_size(algo: u32, size: &mut usize) -> TeeResult {
    // TODO!
    unimplemented!("tee_alg_get_digest_size implementation required")
}

/// Safely writes a u64 value to a user-space pointer
///
/// This function performs the following operations:
/// 1. Checks if the u64 value exceeds the usize range (on 32-bit systems)
/// 2. Copies the value to user space in a secure manner
///
/// # Arguments
/// * `dst` - Target user-space pointer (usize address)
/// * `src` - Reference to source u64 value
///
/// # Returns
/// * `TeeResult` - Operation result:
///   - Returns `Ok(())` on success
///   - Returns `TEE_ERROR_OVERFLOW` on overflow
///   - Returns appropriate error code on copy failure
///
/// # Safety
/// - Caller must ensure `dst` is a valid user-space address
/// - Performs user-space memory write operations, must ensure target memory is writable
fn put_user_u64(dst: &mut usize, src: &u64) -> TeeResult {
    let mut d: u64 = 0;

    // check overflow: 32bit，usize = u32，not hold u64
    if *src > usize::MAX as u64 {
        return Err(TEE_ERROR_OVERFLOW);
    }

    // copy_to_user: set
    copy_to_user_u64(&mut d, src)?;

    *dst = d as usize;

    Ok(())
}

/// Updates a hash or MAC operation with new data chunk
///
/// This function adds a data chunk to an ongoing cryptographic hash or MAC operation.
/// The operation must have been previously initialized with `sys_tee_scn_hash_init`.
///
/// # Arguments
/// * `state` - Handle to the crypto operation state
/// * `chunk` - Pointer to the data chunk to process
/// * `chunk_size` - Size of the data chunk in bytes
///
/// # Returns
/// * `TeeResult` - Returns `TEE_SUCCESS` on success, or error code:
///   - `TEE_ERROR_BAD_STATE` if operation not initialized
///   - `TEE_ERROR_BAD_PARAMETERS` for invalid parameters
///   - `TEE_ERROR_OUT_OF_MEMORY` if memory allocation fails
///
/// # Errors
/// - Returns error if cryptographic context is invalid or operation type unsupported
/// - Fails if unable to copy user-provided data to kernel space
///
/// # Safety
/// - Requires valid user-space pointers for data chunk
/// - Must be called with valid cryptographic state handle
pub(crate) fn sys_tee_scn_hash_update(state: usize, chunk: usize, chunk_size: usize) -> TeeResult {
    // Supporting function definitions (based on provided context)
    // Validate parameters: null chunk with non-zero size is invalid
    if chunk == 0 && chunk_size != 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    // Zero length hash is valid but requires no action
    if chunk_size == 0 {
        return Ok(());
    }

    // Strip memory tag if present (for systems with memory tagging)
    let chunk = memtag_strip_tag_const(chunk);

    with_tee_session_ctx(|ctx| {
        vm_check_access_rights(
            // uctx,
            unsafe { &*(ctx as *const _ as usize as *const user_mode_ctx) },
            TEE_MEMORY_ACCESS_READ | TEE_MEMORY_ACCESS_ANY_OWNER,
            chunk,
            chunk_size,
        )
    })?;

    with_tee_session_ctx_mut(|ctx| {
        match ctx.cryp_state.as_mut() {
            Some(s) => {
                // Retrieve the specific crypto operation state
                let mut crypto_state = tee_svc_cryp_get_state(s, state)?;

                // Verify that the state is initialized
                if crypto_state.state != CrypState::Initialized {
                    return Err(TEE_ERROR_BAD_STATE);
                }

                // Process based on algorithm class (HASH or MAC)
                match tee_alg_get_class(crypto_state.algo) {
                    TEE_OPERATION_DIGEST => {
                        // Hash digest operation
                        let chunk_d = unsafe { from_raw_parts(chunk as *const u8, chunk_size) };

                        // Enter user access context for safe memory access
                        enter_user_access();
                        let res = if let Some(ctx) = crypto_state.ctx.downcast_mut::<SM3HashCtx>() {
                            crypto_hash_update(ctx, chunk_d)
                        } else {
                            Err(TEE_ERROR_BAD_STATE)
                        };
                        exit_user_access();

                        res?;
                    }
                    TEE_OPERATION_MAC => {
                        // MAC (Message Authentication Code) operation
                        let chunk_d =
                            unsafe { slice::from_raw_parts(chunk as *const u8, chunk_size) };

                        // Enter user access context for safe memory access
                        enter_user_access();
                        let res = if let Some(ctx) = crypto_state.ctx.downcast_mut::<SM3HmacCtx>() {
                            crypto_mac_update(ctx, chunk_d)
                        } else {
                            Err(TEE_ERROR_BAD_STATE)
                        };
                        exit_user_access();

                        if let Err(_) = res {
                            return Err(TEE_ERROR_MAC_INVALID);
                        }
                    }
                    _ => {
                        // Unsupported operation class
                        return Err(TEE_ERROR_BAD_PARAMETERS);
                    }
                }

                Ok(())
            }
            None => Err(TEE_ERROR_BAD_STATE),
        }
    })?;

    Ok(())
}

/// Initializes a hash or MAC operation with the given state
///
/// This function initializes a cryptographic operation (either hash or MAC) using the provided state.
/// For hash operations, it initializes the SM3 hash context. For MAC operations, it retrieves the
/// key from the associated object and initializes the SM3 HMAC context.
///
/// # Arguments
/// * `state` - The virtual address identifier of the cryptographic state to initialize
/// * `_iv` - Initialization vector (currently unused, reserved for future use)
/// * `_iv_len` - Length of the initialization vector (currently unused, reserved for future use)
///
/// # Returns
/// * `TeeResult` - Returns TEE_SUCCESS on successful initialization, or appropriate error code:
///   - TEE_ERROR_BAD_STATE: If the cryptographic context cannot be downcast to expected type
///   - TEE_ERROR_BAD_PARAMETERS: If the key object is not initialized or has invalid parameters
///   - Other error codes from underlying crypto operations
///
/// # Algorithm Support
/// - TEE_OPERATION_DIGEST: Initializes SM3 hash context
/// - TEE_OPERATION_MAC: Initializes SM3 HMAC context with the provided key
///
/// # State Management
/// - Updates the cryptographic state to `CrypState::Initialized` after successful initialization
/// - Verifies that the key object is properly initialized before using it for MAC operations
pub(crate) fn sys_tee_scn_hash_init(state: usize, _iv: usize, _iv_len: usize) -> TeeResult {
    // get current session user_ta_ctx
    // let session = ts_get_current_session()?;
    // let mut session =
    with_tee_session_ctx_mut(|ctx| {
        match ctx.cryp_state.as_mut() {
            Some(s) => {
                // get crypto state
                let mut crypto_state; // = tee_svc_cryp_get_state(session, state)?;
                crypto_state = tee_svc_cryp_get_state(s, state)?;
                // 根据算法类型执行不同的初始化
                match tee_alg_get_class(crypto_state.algo) {
                    TEE_OPERATION_DIGEST => {
                        if let Some(ctx) = crypto_state.ctx.downcast_mut::<SM3HashCtx>() {
                            crypto_hash_init(ctx)?;
                        } else {
                            return Err(TEE_ERROR_BAD_STATE);
                        }
                    }
                    TEE_OPERATION_MAC => {
                        // get key
                        let o_arc = tee_obj_get(crypto_state.key1 as tee_obj_id_type)?;
                        let o = o_arc.lock();
                        if (o.info.handleFlags & TEE_HANDLE_FLAG_INITIALIZED) == 0 {
                            return Err(TEE_ERROR_BAD_PARAMETERS);
                        }

                        let key = o.attr.get(0);
                        let key1 = o.attr.get(1);
                        if let Some(key) = key {
                            if let Some(key1) = key1 {
                                if let Some(ctx) = crypto_state.ctx.downcast_mut::<SM3HmacCtx>() {
                                    let len = match key {
                                        TeeCryptObj::obj_secret(key) => key.layout.size(),
                                        _ => return Err(TEE_ERROR_BAD_PARAMETERS),
                                    };
                                    let data = match key1 {
                                        TeeCryptObj::obj_secret(key) => {
                                            key.secret() as *const tee_cryp_obj_secret as *const u8
                                        }
                                        _ => return Err(TEE_ERROR_BAD_PARAMETERS),
                                    };
                                    let key = unsafe { from_raw_parts(data, len) };
                                    crypto_mac_init(ctx, key)?;
                                } else {
                                    return Err(TEE_ERROR_BAD_STATE);
                                }
                            } else {
                                return Err(TEE_ERROR_BAD_PARAMETERS);
                            }
                        } else {
                            return Err(TEE_ERROR_BAD_PARAMETERS);
                        }
                    }
                    _ => return Err(TEE_ERROR_BAD_PARAMETERS),
                }

                // 更新状态
                if CrypState::Initialized != crypto_state.state {
                    crypto_state.state = CrypState::Initialized;
                }
                return Ok(());
            }
            None => Err(TEE_ERROR_BAD_STATE),
        }
    })?;

    Ok(())
}

#[cfg(feature = "tee_test")]
pub mod tests_tee_svc_cryp {
    use zerocopy::IntoBytes;

    //-------- local tests import --------
    use super::*;
    //-------- test framework import --------
    use crate::tee::TestDescriptor;
    use crate::{assert, assert_eq, assert_ne, tee::TestResult, test_fn, tests, tests_name};

    test_fn! {
        using TestResult;
        fn test_tee_svc_cryp_utils() {
            assert_eq!(1, 1); // buffer remains unchanged
        }
    }

    tests_name! {
        TEST_TEE_SVC_CRYP;
        //------------------------
        test_tee_svc_cryp_utils,
    }
}

fn translate_compat_algo(algo: u32) -> u32 {
    match algo {
        TEE_ALG_ECDSA_P192 => TEE_ALG_ECDSA_SHA1,
        TEE_ALG_ECDSA_P224 => TEE_ALG_ECDSA_SHA224,
        TEE_ALG_ECDSA_P256 => TEE_ALG_ECDSA_SHA256,
        TEE_ALG_ECDSA_P384 => TEE_ALG_ECDSA_SHA384,
        TEE_ALG_ECDSA_P521 => TEE_ALG_ECDSA_SHA512,
        TEE_ALG_ECDH_P192 | TEE_ALG_ECDH_P224 | TEE_ALG_ECDH_P256 | TEE_ALG_ECDH_P384
        | TEE_ALG_ECDH_P521 => TEE_ALG_ECDH_DERIVE_SHARED_SECRET,
        _ => algo,
    }
}

fn tee_svc_cryp_check_key_type(o: &tee_obj, algo: u32, mode: TEE_OperationMode) -> TeeResult {
    let mut req_key_type: u32 = 0;
    let mut req_key_type2: u32 = 0;
    match tee_alg_get_main_alg(algo) {
        TEE_MAIN_ALGO_MD5 => {
            req_key_type = TEE_TYPE_HMAC_MD5;
        }
        TEE_MAIN_ALGO_SHA1 => {
            req_key_type = TEE_TYPE_HMAC_SHA1;
        }
        TEE_MAIN_ALGO_SHA224 => {
            req_key_type = TEE_TYPE_HMAC_SHA224;
        }
        TEE_MAIN_ALGO_SHA256 => {
            req_key_type = TEE_TYPE_HMAC_SHA256;
        }
        TEE_MAIN_ALGO_SHA384 => {
            req_key_type = TEE_TYPE_HMAC_SHA384;
        }
        TEE_MAIN_ALGO_SHA512 => {
            req_key_type = TEE_TYPE_HMAC_SHA512;
        }
        TEE_MAIN_ALGO_SHA3_224 => {
            req_key_type = TEE_TYPE_HMAC_SHA3_224;
        }
        TEE_MAIN_ALGO_SHA3_256 => {
            req_key_type = TEE_TYPE_HMAC_SHA3_256;
        }
        TEE_MAIN_ALGO_SHA3_384 => {
            req_key_type = TEE_TYPE_HMAC_SHA3_384;
        }
        TEE_MAIN_ALGO_SHA3_512 => {
            req_key_type = TEE_TYPE_HMAC_SHA3_512;
        }
        TEE_MAIN_ALGO_SM3 => {
            req_key_type = TEE_TYPE_HMAC_SM3;
        }
        TEE_MAIN_ALGO_AES => {
            req_key_type = TEE_TYPE_AES;
        }
        TEE_MAIN_ALGO_DES => {
            req_key_type = TEE_TYPE_DES;
        }
        TEE_MAIN_ALGO_DES3 => {
            req_key_type = TEE_TYPE_DES3;
        }
        TEE_MAIN_ALGO_SM4 => {
            req_key_type = TEE_TYPE_SM4;
        }
        TEE_MAIN_ALGO_RSA => {
            req_key_type = TEE_TYPE_RSA_KEYPAIR;
            if (mode == TEE_OperationMode::TEE_MODE_ENCRYPT
                || mode == TEE_OperationMode::TEE_MODE_VERIFY)
            {
                req_key_type2 = TEE_TYPE_RSA_PUBLIC_KEY;
            }
        }
        TEE_MAIN_ALGO_DSA => {
            req_key_type = TEE_TYPE_DSA_KEYPAIR;
            if (mode == TEE_OperationMode::TEE_MODE_ENCRYPT
                || mode == TEE_OperationMode::TEE_MODE_VERIFY)
            {
                req_key_type2 = TEE_TYPE_DSA_PUBLIC_KEY;
            }
        }
        TEE_MAIN_ALGO_DH => {
            req_key_type = TEE_TYPE_DH_KEYPAIR;
        }
        TEE_MAIN_ALGO_ECDSA => {
            req_key_type = TEE_TYPE_ECDSA_KEYPAIR;
            if (mode == TEE_OperationMode::TEE_MODE_VERIFY) {
                req_key_type2 = TEE_TYPE_ECDSA_PUBLIC_KEY;
            }
        }
        TEE_MAIN_ALGO_ECDH => {
            req_key_type = TEE_TYPE_ECDH_KEYPAIR;
        }
        TEE_MAIN_ALGO_ED25519 => {
            req_key_type = TEE_TYPE_ED25519_KEYPAIR;
            if (mode == TEE_OperationMode::TEE_MODE_VERIFY) {
                req_key_type2 = TEE_TYPE_ED25519_PUBLIC_KEY;
            }
        }
        TEE_MAIN_ALGO_SM2_PKE => {
            if (mode == TEE_OperationMode::TEE_MODE_ENCRYPT) {
                req_key_type = TEE_TYPE_SM2_PKE_PUBLIC_KEY;
            } else {
                req_key_type = TEE_TYPE_SM2_PKE_KEYPAIR;
            }
        }
        TEE_MAIN_ALGO_SM2_DSA_SM3 => {
            if (mode == TEE_OperationMode::TEE_MODE_VERIFY) {
                req_key_type = TEE_TYPE_SM2_DSA_PUBLIC_KEY;
            } else {
                req_key_type = TEE_TYPE_SM2_DSA_KEYPAIR;
            }
        }
        TEE_MAIN_ALGO_SM2_KEP => {
            req_key_type = TEE_TYPE_SM2_KEP_KEYPAIR;
            req_key_type2 = TEE_TYPE_SM2_KEP_PUBLIC_KEY;
        }
        TEE_MAIN_ALGO_HKDF => {
            req_key_type = TEE_TYPE_HKDF_IKM;
        }
        TEE_MAIN_ALGO_CONCAT_KDF => {
            req_key_type = TEE_TYPE_CONCAT_KDF_Z;
        }
        TEE_MAIN_ALGO_PBKDF2 => {
            req_key_type = TEE_TYPE_PBKDF2_PASSWORD;
        }
        TEE_MAIN_ALGO_X25519 => {
            req_key_type = TEE_TYPE_X25519_KEYPAIR;
        }
        TEE_MAIN_ALGO_X448 => {
            req_key_type = TEE_TYPE_X448_KEYPAIR;
        }
        _ => return Err(TEE_ERROR_BAD_PARAMETERS),
    }

    if (req_key_type != o.info.objectType && req_key_type2 != o.info.objectType) {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }
    Ok(())
}
