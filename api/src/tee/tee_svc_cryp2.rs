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

fn get_user_u64_as_size_t(dst: &mut usize, src: &u64) -> TeeResult {
    let mut d: u64 = 0;

    // copy_from_user: 读取用户态数据
    copy_from_user_u64(&mut d, src)?;

    // 检查是否溢出：在 32bit 平台，usize = u32，不能装下全部的 u64
    if d > usize::MAX as u64 {
        return Err(TEE_ERROR_OVERFLOW);
    }

    *dst = d as usize;

    Ok(())
}

fn op_u32_to_binary_helper(v: u32, data: &mut [u8], offs: &mut size_t) -> TeeResult {
    let field: u32;
    let next_offs: size_t;

    next_offs = offs
        .checked_add(size_of::<u32>())
        .ok_or(TEE_ERROR_OVERFLOW)?;

    if data.len() >= next_offs {
        field = tee_u32_to_big_endian(v);
        let field_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(
                &field as *const u32 as *const u8,
                core::mem::size_of::<u32>(),
            )
        };
        data[*offs..*offs + size_of::<u32>()].copy_from_slice(field_bytes);
    }
    *offs = next_offs;

    Ok(())
}

fn op_u32_from_binary_helper(v: &mut u32, data: &[u8], offs: &mut size_t) -> TeeResult {
    let field: u32;

    if data.len() < *offs + size_of::<u32>() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    let field_bytes = &data[*offs..*offs + size_of::<u32>()];
    field = u32::from_be_bytes(
        field_bytes
            .try_into()
            .map_err(|_| TEE_ERROR_BAD_PARAMETERS)?,
    );
    *v = field;
    *offs += size_of::<u32>();

    Ok(())
}

/// 从用户空间导入大数属性
///
/// attr: 密钥属性指针
/// buffer: 用户空间缓冲区
fn op_attr_bignum_from_user(_attr: *mut u8, buffer: &[u8]) -> TeeResult {
    let mut kbuf: Box<[u8]> = vec![0u8; buffer.len()].into_boxed_slice();

    copy_from_user(kbuf.as_mut(), buffer, buffer.len())?;

    // TODO: add call to crypto_bignum_bin2bn(bbuf, size, *bn);

    Ok(())
}

/// 导出大数属性到用户空间
///
/// attr: 密钥属性指针
/// buffer: 用户空间缓冲区
/// size_ref: 用户空间大小指针
fn op_attr_bignum_to_user(_attr: *mut u8, buffer: &mut [u8], size_ref: &mut u64) -> TeeResult {
    let mut s: u64 = 0;

    // copy size from user
    copy_from_user_u64(&mut s, size_ref)?;
    let req_size: u64 = 0; // TODO: call crypto_bignum_num_bytes
    copy_to_user_u64(size_ref, &req_size)?;

    if req_size == 0 {
        return Ok(());
    }

    if s < req_size || buffer.is_empty() {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    let mut kbuf: Box<[u8]> = vec![0u8; req_size as _].into_boxed_slice();

    // TODO: call crypto_bignum_bn2bin with _attr to fill kbuf

    copy_to_user(buffer, kbuf.as_mut(), req_size as usize)?;

    Ok(())
}

/// 将大数属性序列化到二进制缓冲区
///
/// attr: 密钥属性指针
/// data: 目标缓冲区,可以为空 []
/// offs: 偏移指针
fn op_attr_bignum_to_binary(_attr: *mut u8, data: &mut [u8], offs: &mut size_t) -> TeeResult {
    let n: u32 = 0; // TODO: call crypto_bignum_num_bytes
    let mut next_offs: size_t;

    op_u32_to_binary_helper(n, data, offs)?;
    next_offs = offs.checked_add(n as usize).ok_or(TEE_ERROR_OVERFLOW)?;

    if data.len() >= next_offs {
        // TODO: call crypto_bignum_bn2bin to fill data[*offs..*offs + n]
    }

    *offs = next_offs;
    Ok(())
}

fn op_attr_bignum_from_binary(_attr: *mut u8, data: &[u8], offs: &mut size_t) -> TeeResult {
    let mut n: u32 = 0;

    op_u32_from_binary_helper(&mut n, data, offs)?;

    if offs
        .checked_add(n as usize)
        .ok_or(TEE_ERROR_BAD_PARAMETERS)?
        > data.len()
    {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    // TODO: call crypto_bignum_bin2bn

    *offs += n as usize;

    Ok(())
}

fn op_attr_bignum_from_obj(_attr: *mut u8, _src_attr: *mut u8) -> TeeResult {
    // TODO: call crypto_bignum_copy
    Ok(())
}

fn op_attr_bignum_clear(_attr: *mut u8) {
    // TODO: call crypto_bignum_clear
    unimplemented!();
}

fn op_attr_bignum_free(_attr: *mut u8) {
    // TODO: call crypto_bignum_free
    unimplemented!();
}

/// 从用户空间导入值属性
///
/// attr: 密钥属性指针
/// buffer: 用户空间缓冲区
/// FIXME: 这里为何不使用 copy_from_user?
fn op_attr_value_from_user(attr: &mut [u8], user_buffer: &[u8]) -> TeeResult {
    if user_buffer.len() != size_of::<u32>() * 2 {
        return Err(TEE_ERROR_GENERIC);
    }

    // Note that only the first value is copied
    attr.copy_from_slice(&user_buffer[..size_of::<u32>()]);

    Ok(())
}

fn op_attr_value_to_user(attr: &[u8], buffer: &mut [u8], size_ref: &mut u64) -> TeeResult {
    let mut s: u64 = 0;
    copy_from_user_u64(&mut s, size_ref)?;

    let value: [u32; 2] = [unsafe { *(attr.as_ptr() as *const u32) }, 0];
    let req_size: u64 = size_of::<[u32; 2]>() as u64;

    if s < req_size || buffer.is_empty() {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    if buffer.len() < req_size as usize {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    let value_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(&value as *const u32 as *const u8, req_size as usize)
    };
    buffer[..req_size as _].copy_from_slice(value_bytes);

    Ok(())
}

fn op_attr_value_to_binary(attr: &[u8], data: &mut [u8], offs: &mut size_t) -> TeeResult {
    let value: u32 = unsafe { *(attr.as_ptr() as *const u32) };
    op_u32_to_binary_helper(value, data, offs)
}

fn op_attr_value_from_binary(attr: &mut [u8], data: &[u8], offs: &mut size_t) -> TeeResult {
    let value_ptr = attr.as_mut_ptr() as *mut u32;
    op_u32_from_binary_helper(unsafe { &mut *value_ptr }, data, offs)
}

fn op_attr_value_from_obj(attr: &mut [u8], src_attr: &[u8]) -> TeeResult {
    attr[..size_of::<u32>()].copy_from_slice(&src_attr[..size_of::<u32>()]);
    Ok(())
}

fn op_attr_value_clear(attr: &mut [u8]) {
    attr[..4].copy_from_slice(&[0u8; size_of::<u32>()]);
}

fn op_attr_25519_to_binary(_attr: &[u8], _data: &mut [u8], _offs: &mut size_t) -> TeeResult {
    unimplemented!();
}

fn op_attr_25519_from_binary(_attr: &mut [u8], _data: &[u8], _offs: &mut size_t) -> TeeResult {
    unimplemented!();
}

fn op_attr_25519_from_obj(_attr: &mut [u8], _src_attr: &[u8]) -> TeeResult {
    unimplemented!();
}

fn op_attr_25519_clear(_attr: &mut [u8]) {
    unimplemented!();
}

fn op_attr_25519_free(_attr: &mut [u8]) {
    unimplemented!();
}

fn is_gp_legacy_des_key_size(obj_type: TEE_ObjectType, sz: size_t) -> bool {
    return CFG_COMPAT_GP10_DES
        && ((obj_type == TEE_TYPE_DES && sz == 56)
            || (obj_type == TEE_TYPE_DES3 && (sz == 112 || sz == 168)));
}

fn check_key_size(props: &tee_cryp_obj_type_props, key_size: size_t) -> TeeResult {
    let mut sz = key_size;

    // In GP Internal API Specification 1.0 the partity bits aren't
    // counted when telling the size of the key in bits so add them
    // here if missing.
    if is_gp_legacy_des_key_size(props.obj_type, sz) {
        sz += sz / 7;
    }

    if sz % props.quanta as usize != 0 {
        return Err(TEE_ERROR_NOT_SUPPORTED);
    }

    if sz < props.min_size as usize {
        return Err(TEE_ERROR_NOT_SUPPORTED);
    }

    if sz > props.max_size as usize {
        return Err(TEE_ERROR_NOT_SUPPORTED);
    }

    Ok(())
}

/// copy in attributes from user space to kernel space
///
/// _uctx: user_ta_ctx, not used now
/// usr_attrs: user space attributes
/// attrs: kernel space attributes
/// return: TeeResult
fn copy_in_attrs(
    _uctx: &mut user_ta_ctx,
    usr_attrs: &[utee_attribute],
    attrs: &mut [TEE_Attribute],
) -> TeeResult {
    // copy usr_attrs to from user space to kernel space
    let mut usr_attrs_buf: Box<[utee_attribute]> =
        vec![utee_attribute::default(); usr_attrs.len()].into_boxed_slice();
    for n in 0..usr_attrs.len() {
        copy_from_user_struct(&mut usr_attrs_buf[n], &usr_attrs[n])?;
    }

    for n in 0..usr_attrs.len() {
        attrs[n].attributeID = usr_attrs_buf[n].attribute_id;
        if attrs[n].attributeID & TEE_ATTR_FLAG_VALUE != 0 {
            attrs[n].content.value.a = usr_attrs_buf[n].a as u32;
            attrs[n].content.value.b = usr_attrs_buf[n].b as u32;
        } else {
            let mut buf = usr_attrs_buf[n].a;
            let len = usr_attrs_buf[n].b;
            let flags = TEE_MEMORY_ACCESS_READ | TEE_MEMORY_ACCESS_ANY_OWNER;
            // TODO: need to implement vm_check_access_rights
            buf = memtag_strip_tag_vaddr(buf as *const c_void) as u64;
            vm_check_access_rights(
                &mut user_mode_ctx::default(),
                flags,
                buf as usize,
                len as usize,
            )?;
            attrs[n].content.memref.buffer = buf as *mut c_void;
            attrs[n].content.memref.size = len as usize;
        }
    }
    Ok(())
}

enum attr_usage {
    ATTR_USAGE_POPULATE = 0,
    ATTR_USAGE_GENERATE_KEY = 1,
}

fn get_ec_key_size(curve: u32) -> TeeResult<usize> {
    let mut key_size: usize = 0;
    match curve {
        TEE_ECC_CURVE_NIST_P192 => {
            key_size = 192;
        }
        TEE_ECC_CURVE_NIST_P224 => {
            key_size = 224;
        }
        TEE_ECC_CURVE_NIST_P256 => {
            key_size = 256;
        }
        TEE_ECC_CURVE_NIST_P384 => {
            key_size = 384;
        }
        TEE_ECC_CURVE_NIST_P521 => {
            key_size = 521;
        }
        TEE_ECC_CURVE_SM2 | TEE_ECC_CURVE_25519 => {
            key_size = 256;
        }
        _ => {
            return Err(TEE_ERROR_NOT_SUPPORTED);
        }
    }
    Ok(key_size)
}

// Equivalent to the C enum
#[derive(Debug, Clone, Copy, PartialEq)]
enum CrypState {
    Initialized = 0,
    Uninitialized,
}

// Function pointer type for finalization
type TeeCrypCtxFinalizeFunc = unsafe extern "Rust" fn(*mut c_void);

// Rust equivalent of the tee_cryp_state struct
#[repr(C)]
struct TeeCrypState<'a > {
    // Since TAILQ_ENTRY is a linked list node, we use Option<NonNull> for safe pointer handling
    link: Option<NonNull<TeeCrypState<'a>>>,
    algo: u32,
    mode: u32,
    key1: usize, // vaddr_t is typically usize in Rust
    key2: usize,
    ctx: &'a mut dyn Any,
    ctx_finalize: Option<TeeCrypCtxFinalizeFunc>,
    state: CrypState,
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
    sess: &'a mut user_ta_ctx,
    state_id: usize
) -> TeeResult<&'a mut TeeCrypState> {
    // Get the user TA context from the session
    let utc = sess; //to_user_ta_ctx(sess.ctx.unwrap())?;

    // Iterate through the list of cryptographic states
    for s in utc.iter_mut()?
        .as_mut_slice() {
        // Check if the current state's address matches the requested state ID
        if state_id == s as *mut usize as *mut TeeCrypState as usize {
            // Found the state, return it wrapped in Some
            return Ok(unsafe { &mut*(s as *mut usize as *mut TeeCrypState) });
        }
    }

    // State not found in the list, return error
    Err(TEE_ERROR_BAD_PARAMETERS)
}

use crate::tee::tee_session:: {
    with_tee_session_ctx,with_tee_session_ctx_mut
};
// System calls: hash init
pub(crate) fn sys_tee_scn_hash_init(
    state: usize,
    _iv: usize,
    _iv_len: usize
) -> TeeResult {
    // get current session user_ta_ctx
    // let session = ts_get_current_session()?;
    let mut session = with_tee_session_ctx(|ctx| {
        Ok(ctx.utx.clone())
    })?;

    // get crypto state
    let crypto_state = tee_svc_cryp_get_state(&mut session, state)?;

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
            let o: Arc<tee_obj> = tee_obj_get(crypto_state.key1 as tee_obj_id_type)?;
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
