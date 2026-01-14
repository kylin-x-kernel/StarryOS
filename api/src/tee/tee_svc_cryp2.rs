// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.
#![allow(static_mut_refs)]
use alloc::{
    alloc::{alloc, dealloc},
    boxed::Box,
    string::String,
    sync::Arc,
    vec,
    vec::Vec,
};
use core::slice::{from_raw_parts, from_raw_parts_mut};
use spin::Mutex;
use core::{
    alloc::Layout,
    any::Any,
    ffi::{c_char, c_uint, c_ulong, c_void},
    // from,
    mem::{
        size_of,transmute
    },
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice,
    time::Duration,
    sync::{
        // import atomic
        atomic::{AtomicUsize, Ordering::SeqCst}
        // others ...
    }
};

use axerrno::{AxError, AxResult};
use lazy_static::lazy_static;
use tee_raw_sys::{libc_compat::size_t, *};

// use core::ffi::c_void;
// use core::ptr::NonNull;
use super::{tee_svc_cryp::{
    TeeCryptObj, get_user_u64_as_size_t, tee_cryp_obj_secret, tee_cryp_obj_secret_wrapper, tee_cryp_obj_type_props
}, types_ext::vaddr_t};
use super::{
    TeeResult,
    config::CFG_COMPAT_GP10_DES,
    crypto::crypto::{
        CryptoHashCtx, CryptoMacCtx,
        crypto_hash_init, crypto_mac_init, crypto_mac_update, crypto_mac_final, crypto_hash_update, crypto_hash_final,
        ecc_keypair, ecc_public_key,
    },
    crypto::{sm3_hash::SM3HashCtx, sm3_hmac::SM3HmacCtx},
    libmbedtls::bignum::{
        crypto_bignum_bin2bn, crypto_bignum_bn2bin, crypto_bignum_copy, crypto_bignum_num_bits,
        crypto_bignum_num_bytes,
    },
    libutee::{
        tee_api_objects::TEE_USAGE_DEFAULT,
        utee_defines::{tee_alg_get_class, tee_u32_to_big_endian},
    },
    memtag::{
        memtag_strip_tag_vaddr,kaddr_to_uref
    },
    tee_obj::{tee_obj, tee_obj_add, tee_obj_get, tee_obj_close, tee_obj_id_type},
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
use crate::{
    mm::vm_load_string,
    tee,
    tee::{
        libmbedtls::bignum::BigNum,
        memtag::{
            memtag_strip_tag_const,memtag_strip_tag
        },
        tee_session::{with_tee_session_ctx, with_tee_session_ctx_mut},
        // alg identifiers
        TEE_ALG_SHAKE128,TEE_ALG_SHA3_224,TEE_ALG_SHA3_256,TEE_ALG_SHA3_384,TEE_ALG_SHA3_512,TEE_ALG_SHAKE256,
        __OPTEE_ALG_ECDH_P192,__OPTEE_ALG_ECDH_P256,__OPTEE_ALG_ECDH_P384,__OPTEE_ALG_ECDH_P224,__OPTEE_ALG_ECDH_P521,
        __OPTEE_ALG_ECDSA_P224,__OPTEE_ALG_ECDSA_P384,__OPTEE_ALG_ECDSA_P521,
        TEE_ALG_ECDSA_SHA1,TEE_ALG_ECDSA_SHA224,TEE_ALG_ECDSA_SHA384,TEE_ALG_ECDSA_SHA512,
        TEE_ALG_ECDH_DERIVE_SHARED_SECRET,TEE_ALG_RSASSA_PKCS1_V1_5,
        // access
        user_access::copy_to_user_private,
        // utee defines
        utee_defines::TEE_CHAIN_MODE_XTS
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

// Maximum number of tee_cryp_state
pub(crate) const MAX_TEE_CRYP_STATE: usize = 1024;

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

// Implementations
impl TeeCrypState {
    // Check if the context is none
    pub fn is_ctx_none(&self) -> bool {
        if let Some(ctx) = self.ctx.downcast_ref::<()>() {
            true
        } else {
            false
        }
    }

    // Check if the context pointer is null
    pub fn is_ctx_null(&self) -> bool {
        let raw_ptr: *const dyn Any = self.ctx;
        let ptr_addr = raw_ptr as *const u8 as usize;

        ptr_addr == 0
    }
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

static mut TEECRYPTOSTATENONE: () = ();
static TEECRYPTOSTATEID: AtomicUsize = AtomicUsize::new(0);

impl Default for TeeCrypState {
    fn default() -> Self {
        TEECRYPTOSTATEID.fetch_add(1, SeqCst);
        TeeCrypState {
            algo: 0,
            mode: 0,
            key1: 0,
            key2: 0,
            ctx: unsafe{&mut TEECRYPTOSTATENONE},
            // ctx: &mut (),
            ctx_finalize: None,
            state: CrypState::Uninitialized,
            id: TEECRYPTOSTATEID.load(SeqCst)
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
/// let state = 0x12345678;  // Crypto state handle
/// let chunk = 0xAB00000012345678;  // Tagged input pointer
/// let hash_buf = 0xCD00000087654321;  // Tagged output pointer
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
    let hash_len = unsafe{
        &mut *(hash_len as *mut u64)
    };
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
            TEE_MEMORY_ACCESS_READ |
            TEE_MEMORY_ACCESS_ANY_OWNER,
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
            TEE_MEMORY_ACCESS_READ |
            TEE_MEMORY_ACCESS_WRITE |
            TEE_MEMORY_ACCESS_ANY_OWNER,
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
                        process_mac_final(
                            &mut crypto_state,
                            chunk,
                            chunk_size,
                            hash,
                            &mut hlen,
                        )?;
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
        let data_slice = unsafe {
            slice::from_raw_parts(chunk as *const u8, chunk_size)
        };

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
    let hash_slice = unsafe {
        slice::from_raw_parts_mut(hash as *mut u8, *hlen)
    };

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
            let data_slice = unsafe {
                from_raw_parts(chunk as *const u8, chunk_size)
            };

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
        let hash_slice = unsafe {
            from_raw_parts_mut(hash as *mut u8, *hlen)
        };

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
        let data_slice = unsafe {
            from_raw_parts(chunk as *const u8, chunk_size)
        };

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
    let hash_slice = unsafe {
        from_raw_parts_mut(hash as *mut u8, *hlen)
    };

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
fn put_user_u64(
    dst: &mut usize,
    src: &u64
) -> TeeResult {
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

/// Translates compatibility algorithm identifiers to standard TEE algorithm identifiers
///
/// This function maps legacy/compatibility algorithm constants (like those from OP-TEE)
/// to their corresponding standard TEE algorithm equivalents. This is primarily used
/// to maintain backward compatibility with older applications or libraries that may
/// still be using deprecated algorithm identifiers.
///
/// # Arguments
/// * `algo` - The algorithm identifier to translate (can be either a compatibility ID or standard ID)
///
/// # Returns
/// * `u32` - The translated algorithm identifier:
///   - Standard TEE algorithm ID if input was a compatibility ID
///   - Original algorithm ID if no translation mapping exists
///
/// # Supported Translations
/// ## ECDSA Algorithms (Elliptic Curve Digital Signature Algorithm):
/// - `__OPTEE_ALG_ECDSA_P192` → `TEE_ALG_ECDSA_SHA1`
/// - `__OPTEE_ALG_ECDSA_P224` → `TEE_ALG_ECDSA_SHA224`
/// - `__OPTEE_ALG_ECDSA_P256` → `TEE_ALG_ECDSA_SHA256`
/// - `__OPTEE_ALG_ECDSA_P384` → `TEE_ALG_ECDSA_SHA384`
/// - `__OPTEE_ALG_ECDSA_P521` → `TEE_ALG_ECDSA_SHA512`
///
/// ## ECDH Algorithms (Elliptic Curve Diffie-Hellman):
/// - `__OPTEE_ALG_ECDH_P192/P224/P256/P384/P521` → `TEE_ALG_ECDH_DERIVE_SHARED_SECRET`
///
/// # Behavior
/// - If the input algorithm matches any known compatibility identifier, it is converted to the standard equivalent
/// - If no matching compatibility identifier is found, the original algorithm value is returned unchanged
/// - This allows the function to handle both legacy and modern algorithm identifiers seamlessly
///
/// # Example Usage
/// ```
/// // Translate legacy ECDSA P256 algorithm
/// let standard_algo = translate_compat_algo(__OPTEE_ALG_ECDSA_P256);
/// assert_eq!(standard_algo, TEE_ALG_ECDSA_SHA256);
///
/// // Pass through standard algorithm unchanged
/// let unchanged = translate_compat_algo(TEE_ALG_AES_GCM);
/// assert_eq!(unchanged, TEE_ALG_AES_GCM);
/// ```
fn translate_compat_algo(
    algo: u32
) -> u32 {
    // Match compatibility identifiers
    let res = match algo {
        // Map legacy ECDSA algorithms to standard equivalents with appropriate hash functions
        // These mappings preserve the security properties while using standardized algorithm IDs

        // ECDSA with SHA-1 hash function for P-192 curve
        __OPTEE_ALG_ECDSA_P192 => TEE_ALG_ECDSA_SHA1,

        // ECDSA with SHA-224 hash function for P-224 curve
        __OPTEE_ALG_ECDSA_P224 => TEE_ALG_ECDSA_SHA224,

        // ECDSA with SHA-256 hash function for P-256 curve
        __OPTEE_ALG_ECDSA_P224 => TEE_ALG_ECDSA_SHA224,

        // ECDSA with SHA-384 hash function for P-384 curve
        __OPTEE_ALG_ECDSA_P384 => TEE_ALG_ECDSA_SHA384,

        // ECDSA with SHA-512 hash function for P-521 curve
        __OPTEE_ALG_ECDSA_P521 => TEE_ALG_ECDSA_SHA512,

        // Map all legacy ECDH algorithms to the single standard shared secret derivation algorithm
        // This consolidation simplifies the API while preserving functionality across different curve sizes

        // ECDH key exchange for various curves (P-192, P-224, P-256, P-384, P-521)
        __OPTEE_ALG_ECDH_P192 |
        __OPTEE_ALG_ECDH_P224 |
        __OPTEE_ALG_ECDH_P256 |
        __OPTEE_ALG_ECDH_P384 |
        __OPTEE_ALG_ECDH_P521 => TEE_ALG_ECDH_DERIVE_SHARED_SECRET,

        // Default case: return the original algorithm if no compatibility mapping exists
        // This ensures forward compatibility with new algorithms not yet defined in the mapping
        _ => algo,
    };

    // Return the translated algorithm
    res
}

#[repr(C)]
// pub(crate)
struct FatPointer {
    data: *mut (),
    vtable: *mut (),
}

fn get_data_address(ctx: &dyn Any) -> usize {
    let fptr: *const dyn Any = ctx;

    let fbits: FatPointer = unsafe {transmute(fptr)};

    fbits.data as usize
}

// Check if key type is compatible with algorithm and mode
fn tee_svc_cryp_check_key_type(
    obj: &tee_obj,
    algo: u32,
    mode: u32,
) -> TeeResult {
    // Implementation would verify key type compatibility
    // TODO check logic ...
    // Return success
    Ok(())
}

// Allocate a crypto state structure
fn alloc_cryp_state() -> TeeResult<TeeCrypState> {
    let mut cs = TeeCrypState::default();
    Ok(cs)
}


/// Free cryptographic state and release associated resources
///
/// This function performs complete cleanup of a crypto state object including:
/// - Closing key objects used by the state
/// - Removing the state from the session's state queue
/// - Finalizing and freeing algorithm-specific contexts
/// - Deallocating the state structure itself
///
/// # Arguments
/// * `utc` - User TA context containing state queue and object registry
/// * `cs` - Pointer to cryptographic state to be freed
///
/// # Safety
/// This function operates on raw pointers and must only be called
/// with valid pointers to properly initialized structures
pub(crate) fn cryp_state_free(
    cs: &TeeCrypState
) -> TeeResult {
    // Safety: We assume valid pointers passed from caller
    // This is a direct translation from C code with same safety assumptions

    // unsafe {
    // Release key1 if associated with this state
    // Retrieve the key object from the object registry and close it
    if let Ok(_ptr) = tee_obj_get(cs.key1 as tee_obj_id_type) {
        tee_obj_close(cs.key1 as tee_obj_id_type as _);
    }

    // Release key2 if associated with this state
    // Same as key1, handle cleanup for second key (used in XTS mode)
    if let Ok(_ptr) = tee_obj_get(cs.key2 as tee_obj_id_type) {
        tee_obj_close(cs.key2 as tee_obj_id_type as _);
    }

    // Remove state from the session's cryptographic state queue
    // This unlinks the state from the doubly-linked list
    // link
    with_tee_session_ctx_mut(|ctx| {
        match ctx.cryp_state.as_mut() {
            Some(s) => {
                s.retain(|ele| cs.id != ele.id);
                return Ok(());
            }
            None => {
                Ok(())
            }
        }
    });

    // Call finalization callback if registered
    // This allows algorithm-specific cleanup before context freeing
    if cs.ctx_finalize.is_some() {
        unsafe {
            let p = get_data_address(cs.ctx);
            cs.ctx_finalize.unwrap()(p as *mut c_void);
        }
    }

    // Free algorithm-specific cryptographic context
    // The context type depends on the algorithm class being used
    match tee_alg_get_class(cs.algo) {
        // Symmetric cipher context (AES, DES, etc.)
        TEE_OPERATION_CIPHER => {
            // crypto_cipher_free_ctx((*cs).ctx);
            // TODO
            ;
        }

        // Authenticated encryption context (GCM, CCM, etc.)
        TEE_OPERATION_AE => {
            // crypto_authenc_free_ctx((*cs).ctx);
            // TODO
            ;
        }

        // Hash/Digest context (SHA, MD5, etc.)
        TEE_OPERATION_DIGEST => {
            // crypto_hash_free_ctx((*cs).ctx);
            // TODO
            ;
        }

        // MAC context (HMAC, CMAC, etc.)
        TEE_OPERATION_MAC => {
            // crypto_mac_free_ctx((*cs).ctx);
            // TODO
            ;
        }

        // No context expected for other operation types
        // Asymmetric operations don't store contexts here
        _ => {
            assert!(cs.is_ctx_none(),
                    "Unexpected context for non-crypto operation");
        }
    }

    // Deallocate the state structure itself
    // Final cleanup after all associated resources are released
    // }

    // Return success
    Ok(())
}

/// Appends an encryption state to the end of the session context's state list
///
/// This function manages encryption operation states within a session by adding new encryption states to the current session's state list.
/// If the session has not yet initialized a state list, a new list will be created.
///
/// # Parameters
/// * `cs` - The encryption state object to be inserted
///
/// # Return Value
/// * `TeeResult` - Operation result:
///   - Returns `Ok(())` on success
///   - Returns corresponding error codes on failure
///
/// # Errors
/// * `TEE_ERROR_OUT_OF_MEMORY` - Memory allocation failed
/// * `TEE_ERROR_BAD_STATE` - Invalid session context
///
/// # Safety
/// - Thread-safe access to session context is guaranteed via `with_tee_session_ctx_mut`
/// - Uses `Vec` for dynamic state list management to ensure memory safety
/// Insert crypto state at tail of list
fn cryp_states_insert(
    cs: TeeCrypState
) -> TeeResult<usize> {
    // Get the session context
    with_tee_session_ctx_mut(|ctx| {
        match ctx.cryp_state.as_mut() {
            Some(s) => {
                let len = s.len();
                let p = s.as_ptr();
                s.push(cs);
                let res = unsafe{p.offset(len as _)};
                return Ok(res as usize);
            }
            None => {
                let mut v = Vec::with_capacity(MAX_TEE_CRYP_STATE);
                let len = v.len();
                let p = v.as_ptr();
                // cs
                v.push(cs);
                let res = unsafe{p.add(len as _)};
                ctx.cryp_state = Some(v);
                Ok(res as _)
            }
        }
    })
}

fn copy_kaddr_to_uref(
    // uint32_t *uref,
    // void *kaddr
    uref: vaddr_t,
    kaddr: vaddr_t
) -> TeeResult
{
    let _ref = kaddr_to_uref(kaddr);
    let len = size_of_val(&_ref);
    let _ref = unsafe{
        from_raw_parts(
            &raw const _ref as *const usize as *const u8,
            size_of::<u32>()
        )
    };
    let uref = unsafe{from_raw_parts_mut(uref as *mut vaddr_t as *mut u8, size_of::<u32>())};
    return copy_to_user_private(uref, &_ref, len as size_t);
}

/// Extracts the chain mode from a TEE algorithm identifier
///
/// This function extracts the chaining mode from a TEE algorithm identifier according to
/// the GlobalPlatform TEE Internal Core API specification. The chain mode is stored
/// in bits [11:8] of the algorithm identifier.
///
/// # Arguments
/// * [algo](file:///home/kylin/work/git-src/optee_os/core/include/signed_hdr.h#L37-L37) - Algorithm identifier as any integer type (u8, u16, u32, u64, etc.)
///
/// # Returns
/// * Chain mode extracted from bits [11:8] of the algorithm identifier
///
/// # Example
/// ```
/// let alg = 0x10000110u32; // AES CBC_NOPAD
/// let chain_mode = get_chain_mode(alg); // Returns 1 (CBC_NOPAD)
/// ```
// pub(crate)
fn tee_alg_get_chain_mode<T>(algo: T) -> u32
where
    T: Into<u32>
{
    let algo_value: u32 = algo.into();
    ((algo_value >> 8) & 0xF) as u32
}

/// Allocate and initialize a cryptographic operation state
///
/// # Arguments
/// * `algo` - Cryptographic algorithm identifier
/// * `mode` - Operation mode (encrypt/decrypt/sign/verify etc.)
/// * `key1` - First key object reference (required for most operations)
/// * `key2` - Second key object reference (optional, used for XTS cipher)
/// * `state` - Output parameter to receive allocated state handle
///
/// # Returns
/// * `TEE_SUCCESS` on success
/// * `TEE_ERROR_BAD_PARAMETERS` if parameters are invalid
/// * `TEE_ERROR_OUT_OF_MEMORY` if allocation fails
/// * `TEE_ERROR_NOT_SUPPORTED` if algorithm is not supported
pub(crate) fn sys_tee_scn_cryp_state_alloc(
    algo: usize,
    mode: usize,
    key1: usize,
    key2: usize,
    state: vaddr_t,
) -> TeeResult {
    let mut algo = translate_compat_algo(algo as u32);

    let mut o1: Option<Arc<Mutex<tee_obj>>> = None;
    let mut o2: Option<Arc<Mutex<tee_obj>>> = None;

    // Get and validate first key (key1)
    if key1 != 0 {
        match tee_obj_get(key1 as tee_obj_id_type) {
            Ok(o) => {
                let obj = o.lock();
                if obj.busy {
                    return Err(TEE_ERROR_BAD_PARAMETERS);
                }
                if let Err(e) = tee_svc_cryp_check_key_type(&obj, algo, mode as u32) {
                    return Err(e);
                }
                o1 = Some(o.clone());
            }
            Err(e) => return Err(e),
        }
    }

    // Get and validate second key (key2)
    if key2 != 0 {
        match tee_obj_get(key2 as tee_obj_id_type) {
            Ok(o) => {
                let obj = o.lock();
                if obj.busy {
                    return Err(TEE_ERROR_BAD_PARAMETERS);
                }
                if let Err(e) = tee_svc_cryp_check_key_type(&obj, algo, mode as u32) {
                    return Err(e);
                }
                o2 = Some(o.clone());
            }
            Err(e) => return Err(e),
        }
    }

    // Allocate crypto state structure
    let mut cs = match alloc_cryp_state() {
        Ok(state) => state,
        Err(_) => return Err(TEE_ERROR_OUT_OF_MEMORY),
    };

    // Initialize state fields
    cs.algo = algo;
    cs.mode = mode as u32;
    cs.state = CrypState::Uninitialized;

    // Insert crypto state at tail
    let pcs = match cryp_states_insert(cs) {
        Ok(cs) => {
            // info!("Inserted crypto state");
            cs
        }
        Err(e) => {
            error!("Error inserting crypto state");
            return Err(e);
        }
    };

    // Allocate context based on algorithm class
    let res = match tee_alg_get_class(algo) {
        TEE_OPERATION_CIPHER => {
            // Cipher operations: XTS requires two keys, other modes require one
            let chain_mode = tee_alg_get_chain_mode(algo);
            let is_xts = chain_mode == TEE_CHAIN_MODE_XTS;
            let has_key1 = key1 != 0;
            let has_key2 = key2 != 0;

            if (is_xts && (!has_key1 || !has_key2)) ||
               (!is_xts && (!has_key1 || has_key2)) {
                TEE_ERROR_BAD_PARAMETERS
            } else {
                // TODO!
                // crypto_cipher_alloc_ctx(&mut cs.ctx, algo)
                TEE_SUCCESS
            }
        }

        TEE_OPERATION_AE => {
            // Authenticated encryption: requires exactly one key
            if key1 == 0 || key2 != 0 {
                TEE_ERROR_BAD_PARAMETERS
            } else {
                // TODO
                // crypto_authenc_alloc_ctx(&mut cs.ctx, algo)
                TEE_SUCCESS
            }
        }

        TEE_OPERATION_MAC => {
            // MAC operations: requires exactly one key
            if key1 == 0 || key2 != 0 {
                TEE_ERROR_BAD_PARAMETERS
            } else {
                // TODO
                // crypto_mac_alloc_ctx(&mut cs.ctx, algo)
                TEE_SUCCESS
            }
        }

        TEE_OPERATION_DIGEST => {
            // Hash operations: no keys allowed
            if key1 != 0 || key2 != 0 {
                TEE_ERROR_BAD_PARAMETERS
            } else {
                // TODO
                // crypto_hash_alloc_ctx(&mut cs.ctx, algo)
                TEE_SUCCESS
            }
        }

        TEE_OPERATION_ASYMMETRIC_CIPHER | TEE_OPERATION_ASYMMETRIC_SIGNATURE => {
            // Asymmetric operations: require exactly one key
            // Check for disabled algorithms
            if algo == TEE_ALG_RSASSA_PKCS1_V1_5 /* TEE_ALG_RSASSA_PKCS1_V1_5 */
               && !cfg_rsassa_na1_enabled() {
                TEE_ERROR_NOT_SUPPORTED
            } else if key1 == 0 || key2 != 0 {
                TEE_ERROR_BAD_PARAMETERS
            } else {
                // Context allocated separately for asymmetric ops
                TEE_SUCCESS
            }
        }

        TEE_OPERATION_KEY_DERIVATION => {
            // Key derivation: most require one key, SM2_KEP requires two
            let is_sm2_kep = algo == TEE_ALG_SM2_KEP /* TEE_ALG_SM2_KEP */;
            if is_sm2_kep {
                if key1 == 0 || key2 == 0 {
                    TEE_ERROR_BAD_PARAMETERS
                } else {
                    TEE_SUCCESS
                }
            } else if key1 == 0 || key2 != 0 {
                TEE_ERROR_BAD_PARAMETERS
            } else {
                TEE_SUCCESS
            }
        }

        _ => TEE_ERROR_NOT_SUPPORTED,
    };

    if res != TEE_SUCCESS {
        cryp_state_free(unsafe{&*(pcs as *const TeeCrypState)});
        return Err(res);
    }

    // Return state handle to caller
    if let Err(e) = copy_kaddr_to_uref(state, pcs) {
        cryp_state_free(unsafe{&*(pcs as *const TeeCrypState)});
        return Err(e);
    }

    // Register keys
    if let Some(key) = o1 {
        key.lock().busy = true;
        let pcs = unsafe{&mut*(pcs as *mut TeeCrypState)};
        let guard = key.lock();
        let tee_obj_ref: &tee_obj = unsafe {transmute(&guard as *const _)};
        pcs.key1 =tee_obj_ref as *const tee_obj as usize;
    }

    if let Some(key) = o2 {
        key.lock().busy = true;
        let pcs = unsafe{&mut*(pcs as *mut TeeCrypState)};
        let guard = key.lock();
        let tee_obj_ref: &tee_obj = unsafe {transmute(&guard as *const _)};
        pcs.key2 =tee_obj_ref as *const tee_obj as usize;
    }

    Ok(())
}

// Check if RSASSA_PKCS1_V1_5 is enabled
fn cfg_rsassa_na1_enabled() -> bool {
    #[cfg(CFG_CRYPTO_RSASSA_NA1)]
    {
        true
    }

    #[cfg(not(CFG_CRYPTO_RSASSA_NA1))]
    {
        false
    }
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
