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
use mbedtls::{
    cipher::raw::Cipher,
    hash::{Hmac, Md},
    pk::Pk,
};
use spin::Mutex;
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
        utee_defines::{
            TEE_CHAIN_MODE_XTS, tee_alg_get_chain_mode, tee_alg_get_class, tee_alg_get_main_alg,
            tee_u32_to_big_endian,
        },
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
    tee::{
        self, TEE_ALG_DES3_CMAC, TEE_ALG_SHA3_224, TEE_ALG_SHA3_256, TEE_ALG_SHA3_384,
        TEE_ALG_SHA3_512, TEE_ALG_SHAKE128, TEE_ALG_SHAKE256, TEE_ERROR_NODE_DISABLED,
        TEE_TYPE_CONCAT_KDF_Z, TEE_TYPE_HKDF_IKM, TEE_TYPE_PBKDF2_PASSWORD,
        crypto::{
            self,
            crypto::{crypto_cipher_final, crypto_cipher_init, crypto_cipher_update},
        },
        libmbedtls::bignum::BigNum,
        memtag::{memtag_strip_tag, memtag_strip_tag_const},
        tee_session::{with_tee_session_ctx, with_tee_session_ctx_mut},
        tee_svc_cryp::{CryptoAttrRef, TeeCryptObjAttrOps, tee_crypto_ops},
        utee_defines::{
            TEE_AES_BLOCK_SIZE, TEE_DES_BLOCK_SIZE, TEE_MD5_HASH_SIZE, TEE_SHA1_HASH_SIZE,
            TEE_SHA224_HASH_SIZE, TEE_SHA256_HASH_SIZE, TEE_SHA384_HASH_SIZE, TEE_SHA512_HASH_SIZE,
            TEE_SM3_HASH_SIZE,
        },
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
pub(crate) enum CrypState {
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
    pub mode: TEE_OperationMode,
    pub key1: Option<u32>,
    pub key2: Option<u32>,
    pub ctx: CrypCtx,
    pub ctx_finalize: Option<TeeCrypCtxFinalizeFunc>,
    pub state: CrypState,
    pub id: u32,
}

pub(crate) enum CrypCtx {
    CipherCtx(Cipher),
    HashCtx(Md),
    AsyCtx(Pk),
    HmacCtx(Hmac),
    Others,
}

impl Default for TeeCrypState {
    fn default() -> Self {
        Self {
            algo: 0,
            mode: TEE_OperationMode::TEE_MODE_DECRYPT,
            key1: None,
            key2: None,
            ctx: CrypCtx::Others,
            ctx_finalize: None,
            state: CrypState::Uninitialized,
            id: 0,
        }
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
    _state: usize,
    _chunk: usize,
    _chunk_size: usize,
    _hash: usize,
    _hash_len: vaddr_t,
) -> TeeResult {
    // TODO
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
    _crypto_state: &mut TeeCrypState,
    _chunk: usize,
    _chunk_size: usize,
    _hash: usize,
    _hlen: &mut usize,
) -> TeeResult {
    // TODO
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
    _crypto_state: &mut TeeCrypState,
    _chunk: usize,
    _chunk_size: usize,
    _hash: usize,
    _hlen: &mut usize,
) -> TeeResult {
    // TODO
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
    match algo {
        TEE_ALG_MD5 | TEE_ALG_HMAC_MD5 => {
            *size = TEE_MD5_HASH_SIZE;
        }
        TEE_ALG_SHA1 | TEE_ALG_HMAC_SHA1 | TEE_ALG_DSA_SHA1 | TEE_ALG_ECDSA_SHA1 => {
            *size = TEE_SHA1_HASH_SIZE;
        }
        TEE_ALG_SHA224
        | TEE_ALG_SHA3_224
        | TEE_ALG_HMAC_SHA224
        | TEE_ALG_HMAC_SHA3_224
        | TEE_ALG_DSA_SHA224
        | TEE_ALG_ECDSA_SHA224 => {
            *size = TEE_SHA224_HASH_SIZE;
        }
        TEE_ALG_SHA256
        | TEE_ALG_SHA3_256
        | TEE_ALG_HMAC_SHA256
        | TEE_ALG_HMAC_SHA3_256
        | TEE_ALG_DSA_SHA256
        | TEE_ALG_ECDSA_SHA256 => {
            *size = TEE_SHA256_HASH_SIZE;
        }
        TEE_ALG_SHA384
        | TEE_ALG_SHA3_384
        | TEE_ALG_HMAC_SHA384
        | TEE_ALG_HMAC_SHA3_384
        | TEE_ALG_ECDSA_SHA384 => {
            *size = TEE_SHA384_HASH_SIZE;
        }
        TEE_ALG_SHA512
        | TEE_ALG_SHA3_512
        | TEE_ALG_HMAC_SHA512
        | TEE_ALG_HMAC_SHA3_512
        | TEE_ALG_ECDSA_SHA512 => {
            *size = TEE_SHA512_HASH_SIZE;
        }
        TEE_ALG_SM3 | TEE_ALG_HMAC_SM3 => {
            *size = TEE_SM3_HASH_SIZE;
        }
        TEE_ALG_AES_CBC_MAC_NOPAD | TEE_ALG_AES_CBC_MAC_PKCS5 | TEE_ALG_AES_CMAC => {
            *size = TEE_AES_BLOCK_SIZE;
        }
        TEE_ALG_DES_CBC_MAC_NOPAD
        | TEE_ALG_DES_CBC_MAC_PKCS5
        | TEE_ALG_DES3_CBC_MAC_NOPAD
        | TEE_ALG_DES3_CBC_MAC_PKCS5
        | TEE_ALG_DES3_CMAC => {
            *size = TEE_DES_BLOCK_SIZE;
        }
        _ => {
            return Err(TEE_ERROR_NOT_SUPPORTED);
        }
    }
    Ok(())
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
pub(crate) fn sys_tee_scn_hash_update(
    _state: usize,
    _chunk: usize,
    _chunk_size: usize,
) -> TeeResult {
    // TODO
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
pub(crate) fn sys_tee_scn_hash_init(_state: usize, _iv: usize, _iv_len: usize) -> TeeResult {
    // TODO
    Ok(())
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

// 创建一个TeeCrypState
pub fn syscall_cryp_state_alloc(
    algo: u32,
    mode: TEE_OperationMode,
    key1: Option<u32>,
    key2: Option<u32>,
    state: &mut u32,
) -> TeeResult {
    let mut cs = TeeCrypState::default();
    let mut cs_id: u32 = 0;
    let mut res: TeeResult = Ok(());

    // 判断密钥对象是否存在，并取出密钥对象
    let mut o1_ok = false;
    let mut o2_ok = false;
    if let Some(key1) = key1 {
        if let Ok(obj1_arc) = tee_obj_get(key1 as tee_obj_id_type) {
            o1_ok = true;
            let mut o1 = obj1_arc.lock();
            if o1.busy {
                return Err(TEE_ERROR_BUSY);
            }
            o1.busy = true;
            cs.key1 = Some(o1.info.objectId);
            tee_svc_cryp_check_key_type(&*o1, algo, mode)?;
        }
    }

    if let Some(key2) = key2 {
        if let Ok(obj2_arc) = tee_obj_get(key2 as tee_obj_id_type) {
            o2_ok = true;
            let mut o2 = obj2_arc.lock();
            if o2.busy {
                return Err(TEE_ERROR_BUSY);
            }
            o2.busy = true;
            cs.key2 = Some(o2.info.objectId);
            tee_svc_cryp_check_key_type(&*o2, algo, mode)?;
        }
    }

    // 判断密钥是否符合算法要求
    match tee_alg_get_class(algo) {
        TEE_OPERATION_CIPHER => {
            if ((tee_alg_get_chain_mode(algo) == TEE_CHAIN_MODE_XTS && (!o1_ok || !o2_ok))
                || (tee_alg_get_chain_mode(algo) != TEE_CHAIN_MODE_XTS && (!o1_ok || o2_ok)))
            {
                return Err(TEE_ERROR_NODE_DISABLED);
            }
        }
        TEE_OPERATION_AE => {
            if !o1_ok || o2_ok {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        }
        TEE_OPERATION_MAC => {
            if !o1_ok || o2_ok {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        }
        TEE_OPERATION_DIGEST => {
            if o1_ok || o2_ok {
                return Err(TEE_ERROR_NODE_DISABLED);
            }
        }
        TEE_OPERATION_ASYMMETRIC_CIPHER | TEE_OPERATION_ASYMMETRIC_SIGNATURE => {
            if !o1_ok || o2_ok {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        }
        TEE_OPERATION_KEY_DERIVATION => {
            if algo == TEE_ALG_SM2_KEP {
                if !o1_ok || !o2_ok {
                    return Err(TEE_ERROR_BAD_PARAMETERS);
                }
            } else {
                if !o1_ok || o2_ok {
                    return Err(TEE_ERROR_BAD_PARAMETERS);
                }
            }
        }
        _ => {
            return Err(TEE_ERROR_NOT_SUPPORTED);
        }
    }

    with_tee_session_ctx_mut(|ctx| {
        let vacant = ctx.cryp_state.vacant_entry();
        let id = vacant.key();
        let cs_id = id as u32;
        cs.id = cs_id;
        cs.algo = algo;
        cs.mode = mode;
        *state = cs_id;

        // 插入TeeCrypState
        let arc_cs = Arc::new(Mutex::new(cs));
        let _ = vacant.insert(arc_cs);
        Ok(())
    });
    Ok(())
}

// 复制一个TeeCrypState
pub fn syscall_cryp_state_copy(_dst_id: u32, _src_id: u32) -> TeeResult {
    // TODO:需要改动mbedtls，后续再进行实现
    // with_tee_session_ctx_mut(|ctx|{
    // let cs_dst = tee_cryp_state_get(dst_id)?;
    // let cs_src = tee_cryp_state_get(src_id)?;
    //
    // if (cs_dst.lock().algo != cs_src.lock().algo || cs_dst.lock().mode != cs_src.lock().mode) {
    // return Err(TEE_ERROR_BAD_PARAMETERS);
    // }
    //
    // cs_dst.lock().ctx = cs_src.lock().ctx.clone();
    // cs_dst.lock().state = cs_src.lock().state;
    // cs_dst.lock().ctx_finalize = cs_src.lock().ctx_finalize;
    // Ok(())
    // })?;
    Ok(())
}

// 删除一个TeeCrypState
pub fn syscall_cryp_state_free(id: u32) -> TeeResult {
    cryp_state_free(id)
}

// 根据id获取一个TeeCrypState
pub fn tee_cryp_state_get(id: u32) -> TeeResult<Arc<Mutex<TeeCrypState>>> {
    with_tee_session_ctx(|ctx| match ctx.cryp_state.get(id as _) {
        Some(cs) => Ok(Arc::clone(&cs)),
        None => Err(TEE_ERROR_ITEM_NOT_FOUND),
    })
}

// 根据id删除一个TeeCrypState
fn cryp_state_free(id: u32) -> TeeResult {
    with_tee_session_ctx_mut(|ctx| {
        if let Some(cs) = ctx.cryp_state.try_remove(id as usize) {
            tee_debug!("Remove cryp state {}", id);
            return Ok(());
        } else {
            tee_debug!("Remove cryp state failed");
            return Err(TEE_ERROR_BAD_STATE);
        }
    })?;
    Ok(())
}

pub fn syscall_hash_init(id: u32) -> TeeResult {
    let mut cs = tee_cryp_state_get(id)?;
    let cs_guard = cs.lock();
    let algo = cs_guard.algo;
    let key1 = cs_guard.key1;
    drop(cs_guard);

    match tee_alg_get_class(algo) {
        TEE_OPERATION_DIGEST => crypto_hash_init(cs.clone()),
        TEE_OPERATION_MAC => {
            let key1 = if let Some(k) = key1 {
                k
            } else {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            };
            let o = tee_obj_get(key1 as tee_obj_id_type)?;
            let mut o_guard = o.lock();
            if o_guard.attr.is_empty() {
                return Err(TEE_ERROR_BAD_STATE);
            }

            // 从tee_obj中读取密钥
            if let TeeCryptObj::obj_secret(k) = &o_guard.attr[0] {
                let mut key = k.key();
                crypto_mac_init(cs.clone(), key)
            } else {
                Err(TEE_ERROR_BAD_STATE)
            }
        }
        _ => {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }
    }
}

pub fn syscall_hash_update(id: u32, chunk: &[u8]) -> TeeResult {
    memtag_strip_tag_const()?;
    vm_check_access_rights(&mut user_mode_ctx::default(), 0, 0, 0)?;

    let mut cs = tee_cryp_state_get(id)?;
    let cs_guard = cs.lock();
    let algo = cs_guard.algo;

    if cs_guard.state != CrypState::Initialized {
        return Err(TEE_ERROR_BAD_STATE);
    }
    drop(cs_guard);

    match tee_alg_get_class(algo) {
        TEE_OPERATION_DIGEST => crypto_hash_update(cs.clone(), chunk),
        TEE_OPERATION_MAC => crypto_mac_update(cs.clone(), chunk),
        _ => {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }
    }
}

pub fn syscall_hash_final(id: u32, chunk: &[u8], hash: &mut [u8]) -> TeeResult<usize> {
    memtag_strip_tag_const()?;
    memtag_strip_tag()?;
    vm_check_access_rights(&mut user_mode_ctx::default(), 0, 0, 0)?;

    let mut cs = tee_cryp_state_get(id)?;
    let cs_guard = cs.lock();
    let algo = cs_guard.algo;

    if cs_guard.state != CrypState::Initialized {
        return Err(TEE_ERROR_BAD_STATE);
    }
    drop(cs_guard);

    let mut hash_size = 0;
    match tee_alg_get_class(algo) {
        TEE_OPERATION_DIGEST => {
            tee_alg_get_digest_size(algo, &mut hash_size)?;
            if hash.len() < hash_size {
                return Err(TEE_ERROR_SHORT_BUFFER);
            }

            if chunk.len() != 0 {
                crypto_hash_update(cs.clone(), chunk)?;
            }
            return crypto_hash_final(cs.clone(), hash);
        }
        TEE_OPERATION_MAC => {
            tee_alg_get_digest_size(algo, &mut hash_size)?;
            if hash.len() < hash_size {
                return Err(TEE_ERROR_SHORT_BUFFER);
            }

            if chunk.len() != 0 {
                crypto_mac_update(cs.clone(), chunk)?;
            }
            return crypto_mac_final(cs.clone(), hash);
        }
        _ => {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }
    }

    Ok(hash_size)
}

/// optee中只支持NoPad模式
/// NoPad模式下，需要严格保证输入数据长度与算法block_size对齐
pub fn syscall_cipher_init(id: u32, iv: Option<&[u8]>) -> TeeResult {
    let mut cs = tee_cryp_state_get(id)?;
    let cs_guard = cs.lock();
    let algo = cs_guard.algo;
    let key1 = cs_guard.key1;
    let key2 = cs_guard.key2;

    // 当key1和key2都有效时，将key1和key2密钥拼接
    // 在XTS模式下key1和key2都有效
    let mut key: Vec<u8> = Vec::new();

    // 获取key1密钥
    if let Some(k) = key1 {
        let obj_key1 = tee_obj_get(k as _)?;
        let obj_key1_guard = obj_key1.lock();

        if obj_key1_guard.attr.is_empty() {
            return Err(TEE_ERROR_BAD_STATE);
        }

        // 从tee_obj中读取密钥
        if let TeeCryptObj::obj_secret(k) = &obj_key1_guard.attr[0] {
            key.extend_from_slice(k.key());
        } else {
            return Err(TEE_ERROR_BAD_STATE);
        }
    } else {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    };

    // 如果key2存在，则获取key2密钥
    if let Some(k) = key2 {
        // 获取key2_obj
        if let Ok(obj_key2) = tee_obj_get(k as _) {
            let obj_key2_guard = obj_key2.lock();
            if obj_key2_guard.attr.is_empty() {
                return Err(TEE_ERROR_BAD_STATE);
            }

            // 从tee_obj中读取密钥
            if let TeeCryptObj::obj_secret(k) = &obj_key2_guard.attr[0] {
                key.extend_from_slice(k.key());
            } else {
                return Err(TEE_ERROR_BAD_STATE);
            }
        }
    };

    crypto_cipher_init(cs.clone(), key.as_slice(), iv)
}

/// 注意：
/// 对于ECB模式而言，每次只能传入一个块数据，即input.len() == block_size
/// 需要多次调用syscall_cipher_update()函数
/// 对于其他加密模式，可以一次性传入所有加密数据，也可以多次传入
/// 多次调用时，输出区域不要重叠
pub fn syscall_cipher_update(id: u32, input: &[u8], output: &mut [u8]) -> TeeResult<usize> {
    memtag_strip_tag_const()?;
    memtag_strip_tag()?;
    vm_check_access_rights(&mut user_mode_ctx::default(), 0, 0, 0)?;

    let mut cs = tee_cryp_state_get(id)?;
    let cs_guard = cs.lock();

    if cs_guard.state != CrypState::Initialized {
        return Err(TEE_ERROR_BAD_STATE);
    }

    if output.len() < input.len() {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    crypto_cipher_update(cs.clone(), input, output)
}

pub fn syscall_cipher_final(id: u32, output: &mut [u8]) -> TeeResult<usize> {
    memtag_strip_tag_const()?;
    memtag_strip_tag()?;
    vm_check_access_rights(&mut user_mode_ctx::default(), 0, 0, 0)?;

    let mut cs = tee_cryp_state_get(id)?;
    let cs_guard = cs.lock();

    if cs_guard.state != CrypState::Initialized {
        return Err(TEE_ERROR_BAD_STATE);
    }

    crypto_cipher_final(cs.clone(), output)
}

#[cfg(feature = "tee_test")]
pub mod tests_cryp {
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{
            TestDescriptor, TestResult,
            tee_svc_cryp::{syscall_cryp_obj_alloc, syscall_obj_generate_key},
        },
        test_fn, tests, tests_name,
    };

    test_fn! {
        using TestResult;

        fn test_cryp_state(){
            let mut state1: u32 = 0;
            let mut state2: u32 = 0;
            let mut test_obj = tee_obj::default();

            let res = tee_obj_add(test_obj);
            assert!(res.is_ok());
            let id = res.unwrap() as u32;

            let res = syscall_cryp_state_alloc(TEE_ALG_SM3, TEE_OperationMode::TEE_MODE_DIGEST, None, None, &mut state1);
            assert!(res.is_ok());

            let res = syscall_cryp_state_alloc(TEE_ALG_AES_ECB_NOPAD, TEE_OperationMode::TEE_MODE_DECRYPT, Some(id), None, &mut state2);
            assert!(res.is_ok());

            let res = tee_cryp_state_get(state1);
            assert!(res.is_ok());
            let cs1 = res.unwrap();

            let guard1 = cs1.lock();
            assert_eq!(guard1.id, state1);
            assert_eq!(guard1.algo, TEE_ALG_SM3);
            assert!(guard1.mode == TEE_OperationMode::TEE_MODE_DIGEST);
            drop(guard1);

            let res = tee_cryp_state_get(state2);
            assert!(res.is_ok());
            let cs2 = res.unwrap();

            let guard2 = cs2.lock();
            assert_eq!(guard2.id, state2);
            assert_eq!(guard2.algo, TEE_ALG_AES_ECB_NOPAD);
            assert!(guard2.mode == TEE_OperationMode::TEE_MODE_DECRYPT);
            drop(guard2);

            let res = syscall_cryp_state_free(state1);
            assert!(res.is_ok());

            let res = syscall_cryp_state_free(state2);
            assert!(res.is_ok());

            match tee_cryp_state_get(state1) {
                Err(e) => assert_eq!(e, TEE_ERROR_ITEM_NOT_FOUND),
                Ok(_) => panic!("Expected error, but got Ok"),
            }
            match tee_cryp_state_get(state2) {
                Err(e) => assert_eq!(e, TEE_ERROR_ITEM_NOT_FOUND),
                Ok(_) => panic!("Expected error, but got Ok"),
            }
        }
    }

    test_fn! {
        using TestResult;

        fn test_cryp_hash_sm3(){
            let mut state: u32 = 0;
            let res = syscall_cryp_state_alloc(TEE_ALG_SM3, TEE_OperationMode::TEE_MODE_DIGEST, None, None, &mut state);
            assert!(res.is_ok());

            let res = syscall_hash_init(state);
            assert!(res.is_ok());

            let data = b"abc";

            let res = syscall_hash_update(state, &data[..]);
            assert!(res.is_ok());

            let mut hash: [u8; 32] = [0; 32];
            let res = syscall_hash_final(state, &[], &mut hash);
            assert!(res.is_ok());
            let hash_size = res.unwrap();

            assert_eq!(hash_size, 32);
            assert_eq!(hash, [0x66, 0xc7, 0xf0, 0xf4, 0x62, 0xee, 0xed, 0xd9, 0xd1, 0xf2, 0xd4, 0x6b, 0xdc, 0x10, 0xe4, 0xe2, 0x41, 0x67,
                0xc4, 0x87, 0x5c, 0xf2, 0xf7, 0xa2, 0x29, 0x7d, 0xa0, 0x2b, 0x8f, 0x4b, 0xa8, 0xe0]);
        }
    }

    test_fn! {
        using TestResult;

        fn test_cryp_hmac_sm3(){
            let mut state: u32 = 0;
            let mut obj_id: c_uint = 0;
            let result = syscall_cryp_obj_alloc(TEE_TYPE_HMAC_SM3 as _, 128, &mut obj_id);
            assert!(result.is_ok());

            // 随机生成密钥
            let result = syscall_obj_generate_key(obj_id as c_ulong, 128, core::ptr::null(), 0);
            assert!(result.is_ok());

            let obj_arc = tee_obj_get(obj_id as tee_obj_id_type);
            assert!(obj_arc.is_ok());
            let obj_arc = obj_arc.unwrap();
            let mut obj = obj_arc.lock();

            assert_eq!(obj.info.objectType, TEE_TYPE_HMAC_SM3);
            assert_eq!(obj.info.maxObjectSize, 128);
            assert_eq!(obj.info.objectUsage, TEE_USAGE_DEFAULT);
            assert_eq!(obj.attr.len(), 1);
            assert!(matches!(obj.attr[0], TeeCryptObj::obj_secret(_)));

            let key = b"abcdefghabcdefgh";
            let mut secret = tee_cryp_obj_secret_wrapper::new(32);
            secret.set_secret_data(key as &[u8]);
            assert_eq!(secret.key(), key);

            // 赋值固定的key用于验证结果
            let _ = core::mem::replace(&mut obj.attr[0], TeeCryptObj::obj_secret(secret));
            drop(obj);

            let res = syscall_cryp_state_alloc(TEE_ALG_HMAC_SM3, TEE_OperationMode::TEE_MODE_MAC, Some(obj_id as _), None, &mut state);
            assert!(res.is_ok());

            let res = syscall_hash_init(state);
            assert!(res.is_ok());

            let data = b"abc";

            let res = syscall_hash_update(state, &data[..]);
            assert!(res.is_ok());

            let mut hash: [u8; 32] = [0; 32];
            let res = syscall_hash_final(state, &[], &mut hash);
            assert!(res.is_ok());
            let hash_size = res.unwrap();

            assert_eq!(hash_size, 32);
            assert_eq!(hash, [0x99, 0x67, 0xaf, 0x42, 0x68, 0xd7, 0xf6, 0x96, 0x40, 0xca, 0xb9, 0x99, 0x35, 0x18, 0x0f,
                0xb3, 0xc6, 0x9b, 0xc5, 0x82, 0xa2, 0xb9, 0x7f, 0xa7, 0x53, 0xb2, 0x6c, 0x58, 0x10, 0xaa, 0xa0, 0x37]);

        }
    }

    tests_name! {
        TEST_TEE_CRYP;
        //------------------------
        test_cryp_state,
        test_cryp_hash_sm3,
        test_cryp_hmac_sm3,
    }
}
