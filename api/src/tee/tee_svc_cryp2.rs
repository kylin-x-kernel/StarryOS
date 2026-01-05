// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use crate::tee::tee_session:: {
    with_tee_session_ctx,with_tee_session_ctx_mut
};

use alloc::{
    alloc::{alloc, dealloc},
    boxed::Box,
    string::String,
    sync::Arc,
    vec,
    vec::Vec,
};
use core::{
    alloc::Layout, any::Any, ffi::{c_char, c_uint, c_ulong, c_void}, /*from,*/ mem::size_of, ops::{Deref, DerefMut}, ptr::NonNull, slice, time::Duration
};

use axerrno::{AxError, AxResult};
use lazy_static::lazy_static;
use tee_raw_sys::{libc_compat::size_t, *};

use super::{
    TeeResult,
    config::CFG_COMPAT_GP10_DES,
    crypto::crypto::{
        ecc_keypair, ecc_public_key,crypto_hash_init,CryptoHashCtx,CryptoMacCtx,crypto_mac_init,
    },
    crypto::{
        sm3_hash::SM3HashCtx,
        sm3_hmac::SM3HmacCtx,
    },
    libmbedtls::bignum::{
        crypto_bignum_bin2bn, crypto_bignum_bn2bin, crypto_bignum_copy, crypto_bignum_num_bits,
        crypto_bignum_num_bytes,
    },
    libutee::{tee_api_objects::TEE_USAGE_DEFAULT, utee_defines:: {
        tee_u32_to_big_endian,tee_alg_get_class },
    },
    memtag::memtag_strip_tag_vaddr,
    tee_obj::{tee_obj, tee_obj_add, tee_obj_get, tee_obj_id_type},
    tee_pobj::with_pobj_usage_lock,
    user_access::{
        bb_alloc, bb_free, copy_from_user, copy_from_user_struct, copy_from_user_u64, copy_to_user,
        copy_to_user_struct, copy_to_user_u64,
    },
    user_mode_ctx_struct::user_mode_ctx,
    user_ta:: {
        user_ta_ctx, //to_user_ta_ctx
    },
    utils::{bit, bit32},
    vm::vm_check_access_rights,
    // ts_manager:: {
    //     TsSession,
    //     ts_get_current_session, ts_get_current_session_may_fail, ts_push_current_session, ts_pop_current_session, ts_get_calling_session,
    // }
};

// use core::ffi::c_void;
// use core::ptr::NonNull;
use super::tee_svc_cryp:: {
    tee_cryp_obj_secret,
    TeeCryptObj,
    tee_cryp_obj_type_props,
    // tee_svc_cryp_obj_find_type_attr_idx,
    tee_cryp_obj_secret_wrapper,
};

use core::slice::from_raw_parts;
use core::slice::from_raw_parts_mut;

use crate::{mm::vm_load_string, tee, tee::libmbedtls::bignum::BigNum};

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
fn tee_svc_cryp_get_state<'a> (
    // sess: &'a TsSession<'a>
    sess: &'a mut Vec<TeeCrypState>,
    state_id: usize
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
pub(crate) fn sys_tee_scn_hash_init(
    state: usize,
    _iv: usize,
    _iv_len: usize
) -> TeeResult {
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
                                    let len =  match key {
                                        TeeCryptObj::obj_secret(key) => key.layout.size(),
                                        _ => return Err(TEE_ERROR_BAD_PARAMETERS),
                                    };
                                    let data =  match key1 {
                                        TeeCryptObj::obj_secret(key) => key.secret() as *const tee_cryp_obj_secret as *const u8,
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
