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
    alloc::Layout,
    any::Any,
    ffi::{c_char, c_uint, c_ulong, c_void},
    fmt,
    fmt::Debug,
    mem::size_of,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice,
    time::Duration,
};

use axerrno::{AxError, AxResult};
use lazy_static::lazy_static;
use mbedtls::bignum::Mpi;
use mbedtls_sys_auto::*;
use tee_raw_sys::{libc_compat::size_t, *};

use super::{
    TeeResult,
    config::{CFG_COMPAT_GP10_DES, CFG_CORE_BIGNUM_MAX_BITS, CFG_RSA_PUB_EXPONENT_3},
    crypto::{
        crypto::{crypto_acipher_gen_ecc_key, ecc_keypair, ecc_public_key, rsa_keypair},
        crypto_impl::EccAlgoKeyPair,
    },
    libmbedtls::{
        bignum::{
            crypto_bignum_bin2bn, crypto_bignum_bn2bin, crypto_bignum_copy, crypto_bignum_num_bits,
            crypto_bignum_num_bytes,
        },
        rsa::crypto_acipher_gen_rsa_key,
    },
    libutee::{
        tee_api_objects::TEE_USAGE_DEFAULT,
        utee_defines::{tee_u32_from_big_endian, tee_u32_to_big_endian},
    },
    memtag::memtag_strip_tag_vaddr,
    rng_software::crypto_rng_read,
    tee_obj::{tee_obj, tee_obj_add, tee_obj_close, tee_obj_get, tee_obj_id_type},
    tee_pobj::with_pobj_usage_lock,
    tee_svc_storage::tee_svc_storage_write_usage,
    user_access::{
        bb_alloc, bb_free, copy_from_user, copy_from_user_struct, copy_from_user_u64, copy_to_user,
        copy_to_user_struct, copy_to_user_u64,
    },
    user_mode_ctx_struct::user_mode_ctx,
    user_ta::user_ta_ctx,
    utils::{bit, bit32, slice_fmt},
    vm::vm_check_access_rights,
};
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

#[repr(C)]
pub(crate) struct tee_cryp_obj_type_attrs {
    attr_id: u32,
    flags: u16,
    ops_index: u16,
    // raw_offs: u16,
    // raw_size: u16,
}

pub trait TeeCryptObjAttrOps {
    fn from_user(&mut self, buffer: &[u8]) -> TeeResult;

    fn to_user(&self, buffer: &mut [u8], size: &mut u64) -> TeeResult;

    fn to_binary(&self, data: &mut [u8], offs: &mut usize) -> TeeResult;

    fn from_binary(&mut self, data: &[u8], offs: &mut usize) -> TeeResult;

    fn from_obj(&mut self, src_obj: &TeeCryptObjAttr) -> TeeResult;

    fn from_crypto_attr_ref(&mut self, src_obj: &CryptoAttrRef) -> TeeResult;

    fn free(&mut self) {
        // default do nothing
    }

    fn clear(&mut self) {
        // default do nothing
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AttrValue(u32);

impl AttrValue {
    /// 创建一个新的 AttrValue
    pub fn new(value: u32) -> Self {
        AttrValue(value)
    }

    /// 获取内部值
    pub fn get(self) -> u32 {
        self.0
    }

    /// 获取内部值的引用
    pub fn as_u32(&self) -> &u32 {
        &self.0
    }

    /// 获取内部值的可变引用
    pub fn as_mut_u32(&mut self) -> &mut u32 {
        &mut self.0
    }
}

impl Deref for AttrValue {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AttrValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<u32> for AttrValue {
    fn from(value: u32) -> Self {
        AttrValue(value)
    }
}

impl From<AttrValue> for u32 {
    fn from(attr: AttrValue) -> Self {
        attr.0
    }
}

impl AsRef<u32> for AttrValue {
    fn as_ref(&self) -> &u32 {
        &self.0
    }
}

impl AsMut<u32> for AttrValue {
    fn as_mut(&mut self) -> &mut u32 {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
pub enum TeeCryptObjAttr {
    secret_value(tee_cryp_obj_secret_wrapper),
    bignum(BigNum),
    value(AttrValue),
}

/// 用于包装不同类型的属性值引用
pub enum CryptoAttrRef<'a> {
    BigNum(&'a mut BigNum),
    U32(&'a mut u32),
    SecretValue(&'a mut tee_cryp_obj_secret_wrapper),
}

impl TeeCryptObjAttrOps for CryptoAttrRef<'_> {
    fn from_user(&mut self, user_buffer: &[u8]) -> TeeResult {
        match self {
            CryptoAttrRef::BigNum(bn) => bn.from_user(user_buffer),
            CryptoAttrRef::U32(val) => {
                let mut attr = AttrValue::from(**val);
                attr.from_user(user_buffer)?;
                **val = *attr.as_u32();
                Ok(())
            }
            CryptoAttrRef::SecretValue(attr) => attr.from_user(user_buffer),
        }
    }

    fn to_user(&self, buffer: &mut [u8], size_ref: &mut u64) -> TeeResult {
        match self {
            CryptoAttrRef::BigNum(bn) => bn.to_user(buffer, size_ref),
            CryptoAttrRef::U32(val) => {
                let attr = AttrValue::from(**val);
                attr.to_user(buffer, size_ref)
            }
            CryptoAttrRef::SecretValue(attr) => attr.to_user(buffer, size_ref),
        }
    }

    fn from_obj(&mut self, src_obj: &TeeCryptObjAttr) -> TeeResult {
        match self {
            CryptoAttrRef::BigNum(bn) => bn.from_obj(src_obj),
            CryptoAttrRef::U32(val) => {
                let mut attr = AttrValue::from(**val);
                attr.from_obj(src_obj)?;
                **val = *attr.as_u32();
                Ok(())
            }
            CryptoAttrRef::SecretValue(attr) => attr.from_obj(src_obj),
        }
    }

    fn from_crypto_attr_ref(&mut self, src_obj: &CryptoAttrRef) -> TeeResult {
        match self {
            CryptoAttrRef::BigNum(bn) => bn.from_crypto_attr_ref(src_obj),
            CryptoAttrRef::U32(val) => {
                let mut attr = AttrValue::from(**val);
                attr.from_crypto_attr_ref(src_obj)?;
                **val = *attr.as_u32();
                Ok(())
            }
            CryptoAttrRef::SecretValue(attr) => attr.from_crypto_attr_ref(src_obj),
        }
    }

    fn to_binary(&self, data: &mut [u8], offs: &mut usize) -> TeeResult {
        match self {
            CryptoAttrRef::BigNum(bn) => bn.to_binary(data, offs),
            CryptoAttrRef::U32(val) => {
                let attr = AttrValue::from(**val);
                attr.to_binary(data, offs)
            }
            CryptoAttrRef::SecretValue(attr) => attr.to_binary(data, offs),
        }
    }

    fn from_binary(&mut self, data: &[u8], offs: &mut usize) -> TeeResult {
        match self {
            CryptoAttrRef::BigNum(bn) => bn.from_binary(data, offs),
            CryptoAttrRef::U32(val) => {
                let mut attr = AttrValue::from(**val);
                attr.from_binary(data, offs)?;
                **val = *attr.as_u32();
                Ok(())
            }
            CryptoAttrRef::SecretValue(attr) => attr.from_binary(data, offs),
        }
    }

    fn free(&mut self) {
        match self {
            CryptoAttrRef::BigNum(bn) => bn.free(),
            CryptoAttrRef::U32(val) => **val = 0,
            CryptoAttrRef::SecretValue(attr) => attr.free(),
        }
    }

    fn clear(&mut self) {
        match self {
            CryptoAttrRef::BigNum(bn) => bn.clear(),
            CryptoAttrRef::U32(val) => **val = 0,
            CryptoAttrRef::SecretValue(attr) => attr.clear(),
        }
    }
}

impl<'a> CryptoAttrRef<'a> {
    /// 尝试转换为 &mut BigNum，如果不是 BigNum 类型则返回 None
    pub fn as_bignum_mut(&mut self) -> Option<&mut BigNum> {
        match self {
            CryptoAttrRef::BigNum(bn) => Some(bn),
            _ => None,
        }
    }

    /// 尝试转换为 &BigNum，如果不是 BigNum 类型则返回 None
    pub fn as_bignum(&self) -> Option<&BigNum> {
        match self {
            CryptoAttrRef::BigNum(bn) => Some(bn),
            _ => None,
        }
    }

    /// 尝试转换为 &mut u32，如果不是 u32 类型则返回 None
    pub fn as_u32_mut(&mut self) -> Option<&mut u32> {
        match self {
            CryptoAttrRef::U32(val) => Some(val),
            _ => None,
        }
    }

    /// 尝试转换为 &u32，如果不是 u32 类型则返回 None
    pub fn as_u32(&self) -> Option<&u32> {
        match self {
            CryptoAttrRef::U32(val) => Some(val),
            _ => None,
        }
    }
}

pub trait tee_crypto_ops {
    // const TEE_TYPE : u32;
    fn new(key_type: u32, key_size_bits: usize) -> TeeResult<Self>
    where
        Self: Sized;

    fn get_attr_by_id(&mut self, attr_id: c_ulong) -> TeeResult<CryptoAttrRef<'_>>
    where
        Self: Sized;
}

/// 加密对象类型
///
/// 对应类型 TEE_TYPE_*
pub enum TeeCryptObj {
    rsa_keypair(rsa_keypair),
    ecc_keypair(ecc_keypair),
    ecc_public_key(ecc_public_key),
    obj_secret(tee_cryp_obj_secret_wrapper),
    // obj_value(AttrValue),
    // obj_bignum(BigNum),
    None,
}

impl Debug for TeeCryptObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TeeCryptObj::rsa_keypair(key) => write!(f, "TeeCryptObj::rsa_keypair:{:#?}", key),
            TeeCryptObj::ecc_keypair(keypair) => {
                write!(f, "TeeCryptObj::ecc_keypair:{:#?}", keypair)
            }
            TeeCryptObj::ecc_public_key(_) => write!(f, "TeeCryptObj::ecc_public_key"),
            TeeCryptObj::obj_secret(_) => write!(f, "TeeCryptObj::obj_secret"),
            TeeCryptObj::None => write!(f, "TeeCryptObj::None"),
        }
    }
}

impl Default for TeeCryptObj {
    fn default() -> Self {
        TeeCryptObj::None
    }
}

// impl TeeCryptObj {
//     pub fn new(obj_type: TEE_ObjectType) -> Self {
//         match obj_type {
//             TEE_TYPE_ECC_PUBLIC_KEY => TeeCryptObj::ecc_public_key(ecc_public_key::default()),
//             TEE_TYPE_ECC_KEYPAIR => TeeCryptObj::ecc_keypair(ecc_keypair::default()),
//             _ => TeeCryptObj::None,
//         }
//     }
// }
impl tee_crypto_ops for TeeCryptObj {
    fn new(key_type: u32, key_size_bits: usize) -> TeeResult<Self>
    where
        Self: Sized,
    {
        match key_type {
            TEE_TYPE_RSA_KEYPAIR => {
                rsa_keypair::new(key_type, key_size_bits).map(TeeCryptObj::rsa_keypair)
            }
            TEE_TYPE_ECDSA_PUBLIC_KEY
            | TEE_TYPE_ECDH_PUBLIC_KEY
            | TEE_TYPE_SM2_DSA_PUBLIC_KEY
            | TEE_TYPE_SM2_PKE_PUBLIC_KEY
            | TEE_TYPE_SM2_KEP_PUBLIC_KEY => {
                ecc_public_key::new(key_type, key_size_bits).map(TeeCryptObj::ecc_public_key)
            }
            TEE_TYPE_ECDSA_KEYPAIR
            | TEE_TYPE_ECDH_KEYPAIR
            | TEE_TYPE_SM2_DSA_KEYPAIR
            | TEE_TYPE_SM2_PKE_KEYPAIR
            | TEE_TYPE_SM2_KEP_KEYPAIR => {
                ecc_keypair::new(key_type, key_size_bits).map(TeeCryptObj::ecc_keypair)
            }
            TEE_TYPE_DATA => Ok(TeeCryptObj::None),
            TEE_TYPE_AES
            | TEE_TYPE_DES
            | TEE_TYPE_DES3
            | TEE_TYPE_SM4
            | TEE_TYPE_HMAC_MD5
            | TEE_TYPE_HMAC_SHA1
            | TEE_TYPE_HMAC_SHA224
            | TEE_TYPE_HMAC_SHA256
            | TEE_TYPE_HMAC_SHA384
            | TEE_TYPE_HMAC_SHA512
            // | TEE_TYPE_HMAC_SHA3_224
            // | TEE_TYPE_HMAC_SHA3_256
            // | TEE_TYPE_HMAC_SHA3_384
            // | TEE_TYPE_HMAC_SHA3_512
            | TEE_TYPE_HMAC_SM3
            | TEE_TYPE_GENERIC_SECRET
            // | TEE_TYPE_HKDF_IKM
            // | TEE_TYPE_CONCAT_KDF_Z
            // | TEE_TYPE_PBKDF2_PASSWORD
            => {
                <tee_cryp_obj_secret_wrapper as tee_crypto_ops>::new(key_type, key_size_bits).map(TeeCryptObj::obj_secret)
            }
            _ => Err(TEE_ERROR_NOT_SUPPORTED),
        }
    }

    fn get_attr_by_id(&mut self, attr_id: c_ulong) -> TeeResult<CryptoAttrRef<'_>> {
        match self {
            TeeCryptObj::rsa_keypair(key) => key.get_attr_by_id(attr_id),
            TeeCryptObj::ecc_public_key(key) => key.get_attr_by_id(attr_id),
            TeeCryptObj::ecc_keypair(keypair) => keypair.get_attr_by_id(attr_id),
            TeeCryptObj::obj_secret(secret) => secret.get_attr_by_id(attr_id),
            _ => Err(TEE_ERROR_ITEM_NOT_FOUND),
        }
    }
}

impl TeeCryptObjAttrOps for TeeCryptObjAttr {
    fn from_user(&mut self, user_buffer: &[u8]) -> TeeResult {
        match self {
            TeeCryptObjAttr::secret_value(attr) => attr.from_user(user_buffer),
            TeeCryptObjAttr::bignum(attr) => attr.from_user(user_buffer),
            TeeCryptObjAttr::value(attr) => attr.from_user(user_buffer),
        }
    }

    fn to_user(&self, buffer: &mut [u8], size_ref: &mut u64) -> TeeResult {
        match self {
            TeeCryptObjAttr::secret_value(attr) => attr.to_user(buffer, size_ref),
            TeeCryptObjAttr::bignum(attr) => attr.to_user(buffer, size_ref),
            TeeCryptObjAttr::value(attr) => attr.to_user(buffer, size_ref),
        }
    }

    fn from_obj(&mut self, src_obj: &TeeCryptObjAttr) -> TeeResult {
        // TeeCryptObjAttr 需要根据 src_obj 的类型来提取对应的属性
        match self {
            TeeCryptObjAttr::secret_value(attr) => attr.from_obj(src_obj),
            TeeCryptObjAttr::bignum(attr) => attr.from_obj(src_obj),
            TeeCryptObjAttr::value(attr) => attr.from_obj(src_obj),
        }
    }

    fn from_crypto_attr_ref(&mut self, src_obj: &CryptoAttrRef) -> TeeResult {
        match self {
            TeeCryptObjAttr::secret_value(attr) => attr.from_crypto_attr_ref(src_obj),
            TeeCryptObjAttr::bignum(attr) => attr.from_crypto_attr_ref(src_obj),
            TeeCryptObjAttr::value(attr) => attr.from_crypto_attr_ref(src_obj),
        }
    }

    fn to_binary(&self, data: &mut [u8], offs: &mut usize) -> TeeResult {
        match self {
            TeeCryptObjAttr::secret_value(attr) => attr.to_binary(data, offs),
            TeeCryptObjAttr::bignum(attr) => attr.to_binary(data, offs),
            TeeCryptObjAttr::value(attr) => attr.to_binary(data, offs),
        }
    }

    fn from_binary(&mut self, data: &[u8], offs: &mut usize) -> TeeResult {
        match self {
            TeeCryptObjAttr::secret_value(attr) => attr.from_binary(data, offs),
            TeeCryptObjAttr::bignum(attr) => attr.from_binary(data, offs),
            TeeCryptObjAttr::value(attr) => attr.from_binary(data, offs),
        }
    }

    fn free(&mut self) {
        // 根据类型释放资源
        match self {
            TeeCryptObjAttr::secret_value(attr) => attr.free(),
            TeeCryptObjAttr::bignum(attr) => attr.free(),
            TeeCryptObjAttr::value(attr) => attr.free(),
        }
    }

    fn clear(&mut self) {
        match self {
            TeeCryptObjAttr::secret_value(attr) => attr.clear(),
            TeeCryptObjAttr::bignum(attr) => attr.clear(),
            TeeCryptObjAttr::value(attr) => attr.clear(),
        }
    }
}

impl TeeCryptObjAttrOps for AttrValue {
    fn from_user(&mut self, user_buffer: &[u8]) -> TeeResult {
        if user_buffer.len() != size_of::<u32>() * 2 {
            return Err(TEE_ERROR_GENERIC);
        }

        // Note that only the first value is copied
        // 从用户缓冲区读取字节到 AttrValue 的内部 u32
        // 使用 unsafe 直接写入，与 to_user 的实现保持一致
        let value_ptr = self.as_mut_u32() as *mut u32 as *mut u8;
        let value_slice = unsafe { slice::from_raw_parts_mut(value_ptr, size_of::<u32>()) };
        copy_from_user(
            value_slice,
            &user_buffer[..size_of::<u32>()],
            size_of::<u32>(),
        )?;

        Ok(())
    }

    fn to_user(&self, buffer: &mut [u8], size_ref: &mut u64) -> TeeResult {
        let mut s: u64 = 0;
        copy_from_user_u64(&mut s, size_ref)?;

        let value: [u32; 2] = [unsafe { *(self.as_u32() as *const u32) }, 0];
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
        // buffer[..req_size as _].copy_from_slice(value_bytes);
        // copy_to_user_u64(size_ref, &req_size)?;
        copy_to_user(buffer, value_bytes, req_size as usize)?;

        Ok(())
    }

    fn from_obj(&mut self, src_obj: &TeeCryptObjAttr) -> TeeResult {
        match src_obj {
            TeeCryptObjAttr::value(value) => {
                *self = *value;
                Ok(())
            }
            _ => Err(TEE_ERROR_BAD_PARAMETERS),
        }
    }

    fn from_crypto_attr_ref(&mut self, src_obj: &CryptoAttrRef) -> TeeResult {
        match src_obj {
            CryptoAttrRef::U32(val) => {
                *self = AttrValue::from(**val);
                Ok(())
            }
            _ => Err(TEE_ERROR_BAD_PARAMETERS),
        }
    }

    fn to_binary(&self, data: &mut [u8], offs: &mut usize) -> TeeResult {
        let value: u32 = *self.as_u32();
        op_u32_to_binary_helper(value, data, offs)
    }

    fn from_binary(&mut self, data: &[u8], offs: &mut usize) -> TeeResult {
        let value_ref = self.as_mut_u32();
        op_u32_from_binary_helper(value_ref, data, offs)
    }

    fn free(&mut self) {
        // set value to 0
        self.clear();
    }

    fn clear(&mut self) {
        // set value to 0
        *self.as_mut_u32() = 0;
    }
}

impl TeeCryptObjAttrOps for BigNum {
    fn from_user(&mut self, user_buffer: &[u8]) -> TeeResult {
        let mut kbuf: Box<[u8]> = vec![0u8; user_buffer.len()].into_boxed_slice();

        copy_from_user(kbuf.as_mut(), user_buffer, user_buffer.len())?;

        // TODO: add call to crypto_bignum_bin2bn(bbuf, size, *bn);
        crypto_bignum_bin2bn(kbuf.as_ref(), self)?;
        Ok(())
    }

    fn to_user(&self, buffer: &mut [u8], size_ref: &mut u64) -> TeeResult {
        tee_debug!(
            "BigNum::to_user: buffer.len(): {:#x?}, size_ref: {:x?}",
            buffer.len(),
            size_ref
        );
        let mut s: u64 = 0;

        // copy size from user
        copy_from_user_u64(&mut s, size_ref)?;
        let req_size: u64 = 0; // TODO: call crypto_bignum_num_bytes
        let req_size = crypto_bignum_num_bytes(self)? as u64;
        copy_to_user_u64(size_ref, &req_size)?;

        if req_size == 0 {
            return Ok(());
        }

        if s < req_size || buffer.is_empty() {
            return Err(TEE_ERROR_SHORT_BUFFER);
        }

        let mut kbuf: Box<[u8]> = vec![0u8; req_size as _].into_boxed_slice();

        // TODO: call crypto_bignum_bn2bin with _attr to fill kbuf
        crypto_bignum_bn2bin(self, kbuf.as_mut())?;

        copy_to_user(buffer, kbuf.as_ref(), req_size as usize)?;

        Ok(())
    }

    fn from_obj(&mut self, src_obj: &TeeCryptObjAttr) -> TeeResult {
        match src_obj {
            TeeCryptObjAttr::bignum(value) => {
                crypto_bignum_copy(self, &value);
                Ok(())
            }
            _ => Err(TEE_ERROR_BAD_PARAMETERS),
        }
    }

    fn from_crypto_attr_ref(&mut self, src_obj: &CryptoAttrRef) -> TeeResult {
        match src_obj {
            CryptoAttrRef::BigNum(bn) => {
                crypto_bignum_copy(self, bn);
                Ok(())
            }
            _ => Err(TEE_ERROR_BAD_PARAMETERS),
        }
    }

    fn to_binary(&self, data: &mut [u8], offs: &mut usize) -> TeeResult {
        let n: u32 = crypto_bignum_num_bytes(self)? as u32;
        let mut next_offs: usize;

        op_u32_to_binary_helper(n, data, offs)?;
        next_offs = offs.checked_add(n as usize).ok_or(TEE_ERROR_OVERFLOW)?;

        if data.len() >= next_offs {
            crypto_bignum_bn2bin(self, &mut data[*offs..*offs + n as usize])?;
        }

        *offs = next_offs;
        Ok(())
    }

    fn from_binary(&mut self, data: &[u8], offs: &mut usize) -> TeeResult {
        let mut n: u32 = 0;

        op_u32_from_binary_helper(&mut n, data, offs)?;

        if offs
            .checked_add(n as usize)
            .ok_or(TEE_ERROR_BAD_PARAMETERS)?
            > data.len()
        {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }

        crypto_bignum_bin2bn(&data[*offs..*offs + n as usize], self)?;

        *offs += n as usize;

        Ok(())
    }

    fn clear(&mut self) {
        self.as_mpi_mut().clear();
    }
}

impl TeeCryptObjAttrOps for tee_cryp_obj_secret_wrapper {
    fn from_user(&mut self, user_buffer: &[u8]) -> TeeResult {
        let size = user_buffer.len();

        // 1. 长度检查 —— 与 C 完全一致
        if size > self.secret().alloc_size as usize {
            return Err(TEE_ERROR_SHORT_BUFFER);
        }

        // 2. 获取尾随数组可写 slice
        let data_slice = self.data_mut();

        // 3. 拷贝 user_buffer 到尾随数组
        // data_slice[..size].copy_from_slice(user_buffer);
        copy_from_user(&mut data_slice[..size], user_buffer, size as size_t)?;

        // 4. 更新 key_size
        self.secret_mut().key_size = size as u32;

        Ok(())
    }

    fn to_user(&self, buffer: &mut [u8], size_ref: &mut u64) -> TeeResult {
        // --- 1. copy_from_user(&s, size, sizeof(s)) ---
        let mut s: u64 = 0;
        copy_from_user_u64(&mut s, size_ref)?;

        let key_size = self.secret().key_size as u64;

        // --- 2. 将 key_size 回写到用户的 size 指针 ---
        copy_to_user_u64(size_ref, &key_size)?;

        // --- 3. 检查 buffer 是否足够大 ---
        let data = self.data(); // 尾随数组 &[u8]

        if s < key_size || buffer.len() == 0 {
            return Err(TEE_ERROR_SHORT_BUFFER);
        }

        if buffer.len() < key_size as usize {
            return Err(TEE_ERROR_SHORT_BUFFER);
        }

        // --- 4. 将尾随数据 copy_to_user(buffer, key + 1, key_size) ---
        copy_to_user(buffer, data, key_size as usize)?;

        Ok(())
    }

    fn from_obj(&mut self, src_obj: &TeeCryptObjAttr) -> TeeResult {
        // 从 TeeCryptObjAttr 中提取 tee_cryp_obj_secret_wrapper
        match src_obj {
            TeeCryptObjAttr::secret_value(secret) => self.from(secret),
            _ => Err(TEE_ERROR_BAD_PARAMETERS),
        }
    }

    fn from_crypto_attr_ref(&mut self, src_obj: &CryptoAttrRef) -> TeeResult {
        match src_obj {
            CryptoAttrRef::SecretValue(secret) => self.from(secret),
            _ => Err(TEE_ERROR_BAD_PARAMETERS),
        }
    }

    fn to_binary(&self, data: &mut [u8], offs: &mut usize) -> TeeResult {
        let key = self.secret();
        let mut next_offs: usize;

        op_u32_to_binary_helper(key.key_size, data, offs)?;

        next_offs = offs
            .checked_add(key.key_size as usize)
            .ok_or(TEE_ERROR_OVERFLOW)?;

        if data.len() >= next_offs {
            data[*offs..*offs + key.key_size as usize]
                .copy_from_slice(&self.data()[..key.key_size as usize]);
        }
        *offs = next_offs;

        Ok(())
    }

    fn from_binary(&mut self, data: &[u8], offs: &mut usize) -> TeeResult {
        let key = self.secret();
        let mut s: u32 = 0;

        op_u32_from_binary_helper(&mut s, data, offs)?;

        if offs
            .checked_add(s as usize)
            .ok_or(TEE_ERROR_BAD_PARAMETERS)?
            > data.len()
        {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }

        // 数据大小必须适合分配的缓冲区
        if s > key.alloc_size {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }

        self.secret_mut().key_size = s;

        let data_slice = self.data_mut();
        data_slice[..s as usize].copy_from_slice(&data[*offs..*offs + s as usize]);

        *offs += s as usize;

        Ok(())
    }

    fn free(&mut self) {
        self.clear();
    }

    fn clear(&mut self) {
        // set key_size to 0
        self.secret_mut().key_size = 0;
        // set data to 0
        self.data_mut().fill(0);
    }
}

pub const tee_cryp_obj_ecc_pub_key_attrs: &[tee_cryp_obj_type_attrs] = &[
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_ECC_PUBLIC_VALUE_X,
        flags: TEE_TYPE_ATTR_REQUIRED as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_ECC_PUBLIC_VALUE_Y,
        flags: TEE_TYPE_ATTR_REQUIRED as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_ECC_CURVE,
        flags: (TEE_TYPE_ATTR_REQUIRED | TEE_TYPE_ATTR_SIZE_INDICATOR) as _,
        ops_index: ATTR_OPS_INDEX_VALUE as _,
    },
];

pub const tee_cryp_obj_rsa_keypair_attrs: &[tee_cryp_obj_type_attrs] = &[
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_RSA_MODULUS,
        flags: (TEE_TYPE_ATTR_REQUIRED | TEE_TYPE_ATTR_SIZE_INDICATOR) as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_RSA_PUBLIC_EXPONENT,
        flags: (TEE_TYPE_ATTR_REQUIRED | TEE_TYPE_ATTR_GEN_KEY_OPT) as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_RSA_PRIVATE_EXPONENT,
        flags: TEE_TYPE_ATTR_REQUIRED as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_RSA_PRIME1,
        flags: TEE_TYPE_ATTR_OPTIONAL_GROUP as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_RSA_PRIME2,
        flags: TEE_TYPE_ATTR_OPTIONAL_GROUP as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_RSA_EXPONENT1,
        flags: TEE_TYPE_ATTR_OPTIONAL_GROUP as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_RSA_EXPONENT2,
        flags: TEE_TYPE_ATTR_OPTIONAL_GROUP as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_RSA_COEFFICIENT,
        flags: TEE_TYPE_ATTR_OPTIONAL_GROUP as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
];

pub const tee_cryp_obj_ecc_keypair_attrs: &[tee_cryp_obj_type_attrs] = &[
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_ECC_PRIVATE_VALUE,
        flags: TEE_TYPE_ATTR_REQUIRED as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_ECC_PUBLIC_VALUE_X,
        flags: TEE_TYPE_ATTR_REQUIRED as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_ECC_PUBLIC_VALUE_Y,
        flags: TEE_TYPE_ATTR_REQUIRED as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_ECC_CURVE,
        flags: (TEE_TYPE_ATTR_REQUIRED | TEE_TYPE_ATTR_SIZE_INDICATOR | TEE_TYPE_ATTR_GEN_KEY_REQ)
            as _,
        ops_index: ATTR_OPS_INDEX_VALUE as _,
    },
];

pub const tee_cryp_obj_sm2_keypair_attrs: &[tee_cryp_obj_type_attrs] = &[
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_ECC_PRIVATE_VALUE,
        flags: TEE_TYPE_ATTR_REQUIRED as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_ECC_PUBLIC_VALUE_X,
        flags: TEE_TYPE_ATTR_REQUIRED as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
    tee_cryp_obj_type_attrs {
        attr_id: TEE_ATTR_ECC_PUBLIC_VALUE_Y,
        flags: TEE_TYPE_ATTR_REQUIRED as _,
        ops_index: ATTR_OPS_INDEX_BIGNUM as _,
    },
];

#[repr(C)]
pub struct tee_cryp_obj_type_props {
    pub obj_type: TEE_ObjectType,
    pub min_size: u16,
    pub max_size: u16,
    pub alloc_size: u16,
    pub quanta: u8,
    pub num_type_attrs: u8,
    pub type_attrs: &'static [tee_cryp_obj_type_attrs],
}

impl Debug for tee_cryp_obj_type_props {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tee_cryp_obj_type_props{{obj_type: {:#06X?}, min_size: {:#04X?}, max_size: {:#04X?}, \
             alloc_size: {:#04X?}, quanta: {:#03X?}, num_type_attrs: {:#03X?}, type_attrs.id: \
             {:X?}}}",
            self.obj_type,
            self.min_size,
            self.max_size,
            self.alloc_size,
            self.quanta,
            self.num_type_attrs,
            self.type_attrs
                .iter()
                .map(|attr| attr.attr_id)
                .collect::<Vec<_>>()
        )
    }
}

#[repr(C)]
pub(crate) struct tee_cryp_obj_secret {
    pub key_size: u32,
    alloc_size: u32,
    // Pseudo code visualize layout of structure
    // Next follows data, such as:
    // 	uint8_t data[alloc_size]
    // key_size must never exceed alloc_size
}
pub struct tee_cryp_obj_secret_wrapper {
    ptr: NonNull<tee_cryp_obj_secret>,
    pub layout: Layout,
}

impl Debug for tee_cryp_obj_secret_wrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tee_cryp_obj_secret_wrapper{{ptr: {:#010X?}, layout: {:#?}}}, key: {:#?}, \
             alloc_size: {:#06X?}",
            self.ptr.as_ptr(),
            self.layout,
            slice_fmt(self.key()),
            self.secret().alloc_size
        )
    }
}

impl tee_cryp_obj_secret_wrapper {
    /// 分配一个结构体 + 后面变长数组的内存
    pub fn new(alloc_size: usize) -> Self {
        let total_size = size_of::<tee_cryp_obj_secret>() + alloc_size;
        let layout = Layout::from_size_align(total_size, align_of::<tee_cryp_obj_secret>())
            .expect("invalid layout");

        let raw_ptr = unsafe { alloc(layout) as *mut tee_cryp_obj_secret };
        if raw_ptr.is_null() {
            panic!("allocation failed");
        }

        unsafe {
            (*raw_ptr).key_size = 0;
            (*raw_ptr).alloc_size = alloc_size as u32;
        }

        Self {
            ptr: unsafe { NonNull::new_unchecked(raw_ptr) },
            layout,
        }
    }

    /// 获取结构体引用
    pub fn secret(&self) -> &tee_cryp_obj_secret {
        unsafe { self.ptr.as_ref() }
    }

    /// 获取结构体可变引用
    pub fn secret_mut(&mut self) -> &mut tee_cryp_obj_secret {
        unsafe { self.ptr.as_mut() }
    }

    /// 获取尾随数组 `[u8]` 可变引用
    pub fn data_mut(&mut self) -> &mut [u8] {
        let s = self.secret();
        let data_ptr =
            unsafe { (self.ptr.as_ptr() as *mut u8).add(size_of::<tee_cryp_obj_secret>()) };
        unsafe { slice::from_raw_parts_mut(data_ptr, s.alloc_size as usize) }
    }

    /// 获取尾随数组 `[u8]` 不可变引用
    pub fn data(&self) -> &[u8] {
        let s = self.secret();
        let data_ptr =
            unsafe { (self.ptr.as_ptr() as *const u8).add(size_of::<tee_cryp_obj_secret>()) };
        unsafe { slice::from_raw_parts(data_ptr, s.alloc_size as usize) }
    }

    pub fn key(&self) -> &[u8] {
        let s = self.secret();
        let data_ptr =
            unsafe { (self.ptr.as_ptr() as *const u8).add(size_of::<tee_cryp_obj_secret>()) };
        unsafe { slice::from_raw_parts(data_ptr, s.key_size as usize) }
    }

    pub fn set_secret_data(&mut self, data: &[u8]) -> TeeResult {
        if data.len() > self.secret().alloc_size as usize {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }
        let data_slice = self.data_mut();
        data_slice[..data.len()].copy_from_slice(data);
        self.secret_mut().key_size = data.len() as u32;
        Ok(())
    }

    pub fn from(&mut self, secret: &tee_cryp_obj_secret_wrapper) -> TeeResult {
        let key = self.secret();
        let src_key = secret.secret();
        let src_key_size = src_key.key_size;

        if src_key_size > key.alloc_size {
            return Err(TEE_ERROR_BAD_STATE);
        }

        let key_data = self.data_mut();
        let src_key_data = secret.data();

        key_data[..src_key_size as usize].copy_from_slice(&src_key_data[..src_key_size as usize]);
        self.secret_mut().key_size = src_key_size;

        Ok(())
    }
}

impl Drop for tee_cryp_obj_secret_wrapper {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr.as_ptr() as *mut u8, self.layout);
        }
    }
}

impl Clone for tee_cryp_obj_secret_wrapper {
    fn clone(&self) -> Self {
        // 获取源数据
        let src_secret = self.secret();
        let alloc_size = src_secret.alloc_size as usize;
        let key_size = src_secret.key_size as usize;

        // 创建新的实例
        let mut new_wrapper = Self::new(alloc_size);

        // set key size
        new_wrapper.secret_mut().key_size = key_size as u32;

        // 复制尾随数组数据
        if key_size > 0 {
            let src_data = self.data();
            let dst_data = new_wrapper.data_mut();
            dst_data[..key_size].copy_from_slice(&src_data[..key_size]);
        }

        new_wrapper
    }
}

impl tee_crypto_ops for tee_cryp_obj_secret_wrapper {
    fn new(key_type: u32, key_size_bits: usize) -> TeeResult<Self> {
        Ok(Self::new(key_size_bits))
    }

    fn get_attr_by_id(&mut self, _attr_id: c_ulong) -> TeeResult<CryptoAttrRef<'_>> {
        Ok(CryptoAttrRef::SecretValue(self))
    }
}

pub static TEE_CRYP_OBJ_SECRET_VALUE_ATTRS: [tee_cryp_obj_type_attrs; 1] =
    [tee_cryp_obj_type_attrs {
        attr_id: 1,
        flags: 0,
        ops_index: 1,
    }];

pub const fn prop(
    obj_type: TEE_ObjectType,
    quanta: u8,
    min_size: u16,
    max_size: u16,
    alloc_size: u16,
    type_attrs: &'static [tee_cryp_obj_type_attrs],
) -> tee_cryp_obj_type_props {
    tee_cryp_obj_type_props {
        obj_type,
        min_size,
        max_size,
        alloc_size,
        quanta,
        num_type_attrs: type_attrs.len() as u8,
        type_attrs,
    }
}

pub static TEE_CRYP_OBJ_PROPS: [tee_cryp_obj_type_props; 10] = [
    // AES
    prop(
        TEE_TYPE_AES,
        64,
        128,
        256,
        256 / 8,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
    // DES
    prop(
        TEE_TYPE_DES,
        64,
        64,
        64,
        64 / 8,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
    // DES3
    prop(
        TEE_TYPE_DES3,
        64,
        128,
        192,
        192 / 8,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
    // SM4
    prop(
        TEE_TYPE_SM4,
        128,
        128,
        128,
        128 / 8,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
    // HMAC-MD5
    prop(
        TEE_TYPE_HMAC_MD5,
        8,
        64,
        512,
        512 / 8,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
    // HMAC-SM3
    prop(
        TEE_TYPE_HMAC_SM3,
        8,
        80,
        1024,
        512 / 8,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
    // RSA keypair
    prop(
        TEE_TYPE_RSA_KEYPAIR,
        1,
        256,
        CFG_CORE_BIGNUM_MAX_BITS as _,
        0,
        tee_cryp_obj_rsa_keypair_attrs,
    ),
    prop(
        TEE_TYPE_ECDSA_KEYPAIR,
        1,
        192,
        521,
        0,
        tee_cryp_obj_ecc_keypair_attrs,
    ),
    prop(
        TEE_TYPE_ECDSA_PUBLIC_KEY,
        1,
        192,
        521,
        0,
        tee_cryp_obj_ecc_pub_key_attrs,
    ),
    prop(
        TEE_TYPE_SM2_DSA_KEYPAIR,
        1,
        256,
        256,
        0,
        tee_cryp_obj_sm2_keypair_attrs,
    ),
];

pub(crate) fn get_user_u64_as_size_t(dst: &mut usize, src: &u64) -> TeeResult {
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

pub fn tee_obj_set_type(
    obj: &mut tee_obj,
    obj_type: u32,
    max_key_size: size_t,
) -> TeeResult<isize> {
    // Can only set type for newly allocated objs
    if !obj.attr.is_empty() {
        return Err(TEE_ERROR_BAD_STATE);
    }

    if obj_type == TEE_TYPE_DATA {
        if max_key_size != 0 {
            return Err(TEE_ERROR_NOT_SUPPORTED);
        }

        obj.attr.push(TeeCryptObj::None);
    } else {
        // Find description of object
        let type_props = tee_svc_find_type_props(obj_type).ok_or(TEE_ERROR_NOT_SUPPORTED)?;

        // Check that max_key_size follows restrictions
        check_key_size(type_props, max_key_size)?;

        // 检查是否有属性使用 SECRET 操作索引
        let mut alloc_size = max_key_size;
        if type_props
            .type_attrs
            .iter()
            .any(|attr| attr.ops_index == ATTR_OPS_INDEX_SECRET as u16)
        {
            alloc_size = type_props.alloc_size as usize;
        }
        obj.attr.push(TeeCryptObj::new(obj_type, alloc_size)?);
        // o->attr = calloc(1, type_props->alloc_size);
        // if (!o->attr)
        // 	return TEE_ERROR_OUT_OF_MEMORY;
    }

    obj.info.objectType = obj_type;
    obj.info.maxObjectSize = max_key_size as u32;
    if obj.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT != 0 {
        let pobj = obj.pobj.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;
        pobj.write().obj_info_usage = TEE_USAGE_DEFAULT;
    } else {
        obj.info.objectUsage = TEE_USAGE_DEFAULT;
    }

    Ok(0)
}

/// Allocate a new object
///
/// # Arguments
/// * `obj_type` - the type of the object
/// * `max_key_size` - the maximum key size of the object
/// # Returns
/// * `TeeResult` - the result of the operation
pub(crate) fn syscall_cryp_obj_alloc(
    obj_type: c_ulong,
    max_key_size: c_ulong,
    obj: *mut c_uint,
) -> TeeResult {
    let mut o = tee_obj::default();

    tee_obj_set_type(&mut o, obj_type as _, max_key_size as _)?;
    let obj_id: c_uint = tee_obj_add(o)? as c_uint;

    copy_to_user_struct(unsafe { &mut *obj }, &obj_id)?;

    Ok(())
}

/// Close an object
///
/// # Arguments
/// * `obj_id` - the object id
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_cryp_obj_close(obj_id: c_ulong) -> TeeResult {
    {
        let o = tee_obj_get(obj_id as tee_obj_id_type)?;
        let o_guard = o.lock();

        // If it's busy it's used by an operation, a client should never have
        // this handle.
        if o_guard.busy {
            return Err(TEE_ERROR_ITEM_NOT_FOUND);
        }
    }

    tee_obj_close(obj_id as u32)
}

/// reset the object
///
/// # Arguments
/// * `obj_id` - the object id
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_cryp_obj_reset(obj_id: c_ulong) -> TeeResult {
    let o_arc = tee_obj_get(obj_id as tee_obj_id_type)?;
    let mut o = o_arc.lock();

    if o.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT != 0 {
        tee_obj_attr_clear(&mut o);
        o.info.objectSize = 0;
        o.info.objectUsage = TEE_USAGE_DEFAULT;
    } else {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    // the object is no more initialized
    o.info.handleFlags &= !(TEE_HANDLE_FLAG_INITIALIZED as u32);

    Ok(())
}

fn tee_svc_cryp_obj_find_type_attr_idx(
    attr_id: u32,
    type_props: &tee_cryp_obj_type_props,
) -> isize {
    for (n, attr) in type_props.type_attrs.iter().enumerate() {
        if attr_id == attr.attr_id {
            return n as isize;
        }
    }
    -1
}

pub fn tee_svc_find_type_props(
    obj_type: TEE_ObjectType,
) -> Option<&'static tee_cryp_obj_type_props> {
    for props in TEE_CRYP_OBJ_PROPS.iter() {
        if props.obj_type == obj_type {
            return Some(props);
        }
    }
    None
}

// Set an attribute on an object
fn set_attribute(o: &mut tee_obj, props: &tee_cryp_obj_type_props, attr: u32) {
    let idx = tee_svc_cryp_obj_find_type_attr_idx(attr, props);
    if idx < 0 {
        return;
    }
    o.have_attrs |= bit(idx as u32);
}

// Get an attribute on an object
fn get_attribute(o: &tee_obj, props: &tee_cryp_obj_type_props, attr: u32) -> u32 {
    let idx = tee_svc_cryp_obj_find_type_attr_idx(attr, props);
    if idx < 0 {
        return 0;
    }
    o.have_attrs & bit(idx as u32)
}

/// 从用户空间导入密钥属性
///
/// attr: 密钥属性包装器
/// buffer: 用户空间缓冲区
fn op_attr_secret_value_from_user(
    attr: &mut tee_cryp_obj_secret_wrapper,
    user_buffer: &[u8],
) -> TeeResult {
    let size = user_buffer.len();

    // 1. 长度检查 —— 与 C 完全一致
    if size > attr.secret().alloc_size as usize {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    // 2. 获取尾随数组可写 slice
    let data_slice = attr.data_mut();

    // 3. 拷贝 user_buffer 到尾随数组
    // data_slice[..size].copy_from_slice(user_buffer);
    copy_from_user(&mut data_slice[..size], user_buffer, size as size_t)?;

    // 4. 更新 key_size
    attr.secret_mut().key_size = size as u32;

    Ok(())
}

fn op_attr_secret_value_to_user(
    attr: &tee_cryp_obj_secret_wrapper,
    buffer: Option<&mut [u8]>, // C: void *buffer
    size_ref: &mut u64,        // C: uint64_t *size
) -> TeeResult {
    // --- 1. copy_from_user(&s, size, sizeof(s)) ---
    let mut s: u64 = 0;
    copy_from_user_u64(&mut s, size_ref)?;

    let key_size = attr.secret().key_size as u64;

    // --- 2. 将 key_size 回写到用户的 size 指针 ---
    copy_to_user_u64(size_ref, &key_size)?;

    // --- 3. 检查 buffer 是否足够大 ---
    let data = attr.data(); // 尾随数组 &[u8]

    if s < key_size || buffer.is_none() {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    let buffer = buffer.unwrap();
    if buffer.len() < key_size as usize {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    // --- 4. 将尾随数据 copy_to_user(buffer, key + 1, key_size) ---
    copy_to_user(buffer, data, key_size as usize)?;

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

/// 将密钥属性序列化到二进制缓冲区
///
/// data: 目标缓冲区,可以为空 []
fn op_attr_secret_value_to_binary(
    attr: &tee_cryp_obj_secret_wrapper,
    data: &mut [u8],
    offs: &mut size_t,
) -> TeeResult {
    let key = attr.secret();
    let mut next_offs: size_t;

    op_u32_to_binary_helper(key.key_size, data, offs)?;

    next_offs = offs
        .checked_add(key.key_size as usize)
        .ok_or(TEE_ERROR_OVERFLOW)?;

    if data.len() >= next_offs {
        data[*offs..*offs + key.key_size as usize]
            .copy_from_slice(&attr.data()[..key.key_size as usize]);
    }
    *offs = next_offs;

    Ok(())
}

fn op_attr_secret_value_from_binary(
    attr: &mut tee_cryp_obj_secret_wrapper,
    data: &[u8],
    offs: &mut size_t,
) -> TeeResult {
    let key = attr.secret();
    let mut s: u32 = 0;

    op_u32_from_binary_helper(&mut s, data, offs)?;

    if offs
        .checked_add(s as usize)
        .ok_or(TEE_ERROR_BAD_PARAMETERS)?
        > data.len()
    {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    // 数据大小必须适合分配的缓冲区
    if s > key.alloc_size {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    attr.secret_mut().key_size = s;

    let data_slice = attr.data_mut();
    data_slice[..s as usize].copy_from_slice(&data[*offs..*offs + s as usize]);

    *offs += s as usize;

    Ok(())
}

fn op_attr_secret_value_from_obj(
    attr: &mut tee_cryp_obj_secret_wrapper,
    src_attr: &tee_cryp_obj_secret_wrapper,
) -> TeeResult {
    let key = attr.secret();
    let src_key = src_attr.secret();

    if src_key.key_size > key.alloc_size {
        return Err(TEE_ERROR_BAD_STATE);
    }

    let key_data = attr.data_mut();
    let src_key_data = src_attr.data();

    key_data[..src_key.key_size as usize]
        .copy_from_slice(&src_key_data[..src_key.key_size as usize]);
    attr.secret_mut().key_size = src_key.key_size;

    Ok(())
}

fn op_attr_secret_value_clear(attr: &mut tee_cryp_obj_secret_wrapper) {
    attr.secret_mut().key_size = 0;
    let data_slice = attr.data_mut();
    for byte in data_slice.iter_mut() {
        *byte = 0;
    }
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

/// convert the attributes of the object to binary data
/// the order is defined by TEE_CRYP_OBJ_PROPS table
///
/// # Arguments
/// * `o` - the object
/// * `data` - the data to store the binary data
/// * `data_len` - the length of the data
///
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn tee_obj_attr_to_binary(
    o: &mut tee_obj,
    data: &mut [u8],
    data_len: &mut size_t,
) -> TeeResult {
    if o.info.objectType == TEE_TYPE_DATA {
        *data_len = 0;
        return Ok(()); /* pure data object */
    }
    if o.attr.is_empty() {
        return Err(TEE_ERROR_BAD_STATE);
    }

    let tp = tee_svc_find_type_props(o.info.objectType).ok_or(TEE_ERROR_BAD_STATE)?;

    let mut offs: size_t = 0;
    for ta in tp.type_attrs.iter() {
        let mut attr = o.attr[0].get_attr_by_id(ta.attr_id as _)?;
        attr.to_binary(data, &mut offs)?;
    }

    *data_len = offs;

    if (!data.is_empty() && offs > data.len()) {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    Ok(())
}

/// construct the attributes of the object from the binary data
/// the order is defined by TEE_CRYP_OBJ_PROPS table
///
/// # Arguments
/// * `o` - the object
/// * `data` - the data to convert the attributes
/// * `data_len` - the length of the data
///
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn tee_obj_attr_from_binary(o: &mut tee_obj, data: &[u8]) -> TeeResult {
    if o.info.objectType == TEE_TYPE_DATA {
        return Ok(()); /* pure data object */
    }
    if o.attr.is_empty() {
        return Err(TEE_ERROR_BAD_STATE);
    }

    let tp = tee_svc_find_type_props(o.info.objectType).ok_or(TEE_ERROR_BAD_STATE)?;

    let mut offs: size_t = 0;
    for ta in tp.type_attrs.iter() {
        let mut attr = o.attr[0].get_attr_by_id(ta.attr_id as _)?;
        attr.from_binary(data, &mut offs)?;
    }

    if offs != data.len() {
        return Err(TEE_ERROR_CORRUPT_OBJECT);
    }

    Ok(())
}

pub fn tee_obj_attr_copy_from(dst: &mut tee_obj, src: &mut tee_obj) -> TeeResult {
    let mut have_atts: u32 = 0;
    if dst.info.objectType == TEE_TYPE_DATA {
        return Ok(());
    }
    if dst.attr.is_empty() {
        return Err(TEE_ERROR_BAD_STATE);
    }

    let tp = tee_svc_find_type_props(dst.info.objectType).ok_or(TEE_ERROR_BAD_STATE)?;

    if dst.info.objectType == src.info.objectType {
        have_atts = src.have_attrs;
        for ta in tp.type_attrs.iter() {
            let attr_id = ta.attr_id;
            let mut attr_ref = dst.attr[0].get_attr_by_id(attr_id as c_ulong)?;
            let mut attr_src_ref = src.attr[0].get_attr_by_id(attr_id as c_ulong)?;
            attr_ref.from_crypto_attr_ref(&mut attr_src_ref)?;
        }
    } else {
        if (dst.info.objectType == TEE_TYPE_RSA_PUBLIC_KEY) {
            if (src.info.objectType != TEE_TYPE_RSA_KEYPAIR) {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        } else if (dst.info.objectType == TEE_TYPE_DSA_PUBLIC_KEY) {
            if (src.info.objectType != TEE_TYPE_DSA_KEYPAIR) {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        } else if (dst.info.objectType == TEE_TYPE_ECDSA_PUBLIC_KEY) {
            if (src.info.objectType != TEE_TYPE_ECDSA_KEYPAIR) {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        } else if (dst.info.objectType == TEE_TYPE_ECDH_PUBLIC_KEY) {
            if (src.info.objectType != TEE_TYPE_ECDH_KEYPAIR) {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        } else if (dst.info.objectType == TEE_TYPE_SM2_DSA_PUBLIC_KEY) {
            if (src.info.objectType != TEE_TYPE_SM2_DSA_KEYPAIR) {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        } else if (dst.info.objectType == TEE_TYPE_SM2_PKE_PUBLIC_KEY) {
            if (src.info.objectType != TEE_TYPE_SM2_PKE_KEYPAIR) {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        } else if (dst.info.objectType == TEE_TYPE_SM2_KEP_PUBLIC_KEY) {
            if (src.info.objectType != TEE_TYPE_SM2_KEP_KEYPAIR) {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        } else if (dst.info.objectType == TEE_TYPE_ED25519_PUBLIC_KEY) {
            if (src.info.objectType != TEE_TYPE_ED25519_KEYPAIR) {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        } else if (dst.info.objectType == TEE_TYPE_X25519_PUBLIC_KEY) {
            if (src.info.objectType != TEE_TYPE_X25519_KEYPAIR) {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        } else if (dst.info.objectType == TEE_TYPE_X448_PUBLIC_KEY) {
            if (src.info.objectType != TEE_TYPE_X448_KEYPAIR) {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        } else {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }

        let tp_src = tee_svc_find_type_props(src.info.objectType).ok_or(TEE_ERROR_BAD_STATE)?;
        have_atts = bit32(tp.num_type_attrs as u32) - 1;
        for ta in tp.type_attrs.iter() {
            let attr_id = ta.attr_id;
            let mut attr_ref = dst.attr[0].get_attr_by_id(attr_id as c_ulong)?;
            let mut attr_src_ref = src.attr[0].get_attr_by_id(attr_id as c_ulong)?;
            attr_ref.from_crypto_attr_ref(&mut attr_src_ref)?;
        }
    }

    dst.have_attrs = have_atts;
    Ok(())
}

pub fn is_gp_legacy_des_key_size(obj_type: TEE_ObjectType, sz: size_t) -> bool {
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

/// Get the information of the object
///
/// # Arguments
/// * `obj_id` - the object id
/// * `info` - the information to store the object information
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_cryp_obj_get_info(obj_id: c_ulong, info: *mut utee_object_info) -> TeeResult {
    tee_debug!(
        "syscall_cryp_obj_get_info: obj_id: {:#010X?}, info: {:#010X?}",
        obj_id,
        info
    );
    let info = unsafe { &mut *info };

    let mut o_info: utee_object_info = utee_object_info::default();
    let o_arc = tee_obj_get(obj_id as tee_obj_id_type)?;
    let o = o_arc.lock();

    o_info.obj_type = o.info.objectType;
    o_info.obj_size = o.info.objectSize;
    o_info.max_obj_size = o.info.maxObjectSize;
    if o.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT != 0 {
        let pobj = o.pobj.as_ref().ok_or(TEE_ERROR_BAD_STATE)?;

        with_pobj_usage_lock(pobj.read().flags, || {
            o_info.obj_usage = pobj.read().obj_info_usage;
        });
    } else {
        o_info.obj_usage = o.info.objectUsage;
    }
    o_info.data_size = o.info.dataSize as _;
    o_info.data_pos = o.info.dataPosition as _;
    o_info.handle_flags = o.info.handleFlags as _;

    copy_to_user_struct(info, &o_info)?;
    Ok(())
}

/// restrict the usage of the object
///
/// # Arguments
/// * `obj_id` - the object id
/// * `usage` - the usage to restrict
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_cryp_obj_restrict_usage(obj_id: c_ulong, usage: c_ulong) -> TeeResult {
    let mut o_info = utee_object_info::default();

    let o_arc = tee_obj_get(obj_id as tee_obj_id_type)?;
    let mut o = o_arc.lock();
    if o.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT != 0 {
        // get pobj arc and flags in the closure, avoid multiple borrows in the closure
        let pobj_arc = o.pobj.as_ref().ok_or(TEE_ERROR_BAD_STATE)?.clone();
        let pobj_flags = {
            let pobj_guard = pobj_arc.read();
            pobj_guard.flags
        };

        let mut new_usage: u32 = 0;
        let write_res = with_pobj_usage_lock(pobj_flags, || -> TeeResult {
            // get pobj arc in the closure, avoid multiple borrows in the closure
            let pobj_guard = pobj_arc.read();
            new_usage = pobj_guard.obj_info_usage & usage as u32;
            drop(pobj_guard);

            // call write_usage（need &mut o，now can borrow safely，because pobj's lock is released）
            tee_svc_storage_write_usage(&mut o, new_usage)?;

            // get write lock to update obj_info_usage
            let mut pobj_guard = pobj_arc.write();
            pobj_guard.obj_info_usage = new_usage;
            Ok(())
        });

        write_res?;
    } else {
        o.info.objectUsage &= usage as u32;
    }

    Ok(())
}

/// Get the attribute of the object
///
/// # Arguments
/// * `obj_id` - the object id
/// * `attr_id` - the attribute id
/// * `buffer` - the buffer to store the attribute
/// * `size` - the size of the attribute
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_cryp_obj_get_attr(
    obj_id: c_ulong,
    attr_id: c_ulong,
    buffer: *mut c_void,
    size: *mut c_ulong,
) -> TeeResult {
    tee_debug!(
        "syscall_cryp_obj_get_attr: obj_id: {:x?}, attr_id: {:x?}, buffer: {:x?}, size: {:x?}",
        obj_id,
        attr_id,
        buffer,
        size
    );
    let mut obj_usage = 0;
    let o_arc = tee_obj_get(obj_id as tee_obj_id_type)?;
    let mut o = o_arc.lock();
    let size: &mut c_ulong = unsafe {
        debug_assert!(!size.is_null());
        &mut *size
    };
    let buffer: &mut [u8] = unsafe {
        debug_assert!(!buffer.is_null());
        let len = *size;
        core::slice::from_raw_parts_mut(buffer as *mut u8, len as usize)
    };

    if o.info.handleFlags & TEE_HANDLE_FLAG_INITIALIZED == 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    tee_debug!(
        "attr_id: {:x?}, handleFlags: {:x?}",
        attr_id,
        o.info.handleFlags
    );
    if attr_id & TEE_ATTR_FLAG_PUBLIC as c_ulong == 0 {
        if o.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT != 0 {
            let pobj = o.pobj.as_ref().ok_or(TEE_ERROR_BAD_STATE)?.read();
            with_pobj_usage_lock(pobj.flags, || {
                obj_usage = pobj.obj_info_usage;
            });
        } else {
            obj_usage = o.info.objectUsage;
        }
        if obj_usage & TEE_USAGE_EXTRACTABLE == 0 {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }
    }

    let type_props = tee_svc_find_type_props(o.info.objectType).ok_or(TEE_ERROR_BAD_STATE)?;

    let idx = tee_svc_cryp_obj_find_type_attr_idx(attr_id as u32, type_props);
    tee_debug!("idx: {}, have_attrs: {:x?}", idx, o.have_attrs);
    if idx < 0 || (o.have_attrs & (1 << idx)) == 0 {
        return Err(TEE_ERROR_ITEM_NOT_FOUND);
    }

    // let ops = type_props.type_attrs[idx].ops_index;
    // let attr = (o.attr[idx] as *const u8) as *const u8;
    // return ops.to_user(attr, sess, buffer, size);
    if !o.attr.is_empty() {
        let attr_ref = o.attr[0].get_attr_by_id(attr_id)?;
        attr_ref.to_user(buffer, size)?;
    }

    Ok(())
}

pub fn tee_obj_attr_clear(o: &mut tee_obj) -> TeeResult {
    let tp = tee_svc_find_type_props(o.info.objectType).ok_or(TEE_ERROR_BAD_STATE)?;
    if o.attr.is_empty() {
        return Ok(());
    }

    for ta in tp.type_attrs.iter() {
        let mut attr = o.attr[0].get_attr_by_id(ta.attr_id as _)?;
        attr.clear();
    }

    Ok(())
}

/// Copy in attributes from user space to kernel space
/// If use memref attribute, the attr.content.memref.buffer will be the pointer to the memory in user space,
/// later, in functions like from_user, we need to copy the memory from user space to kernel space.
///
/// # Arguments
/// * `_uctx` - user_ta_ctx, not used now
/// * `usr_attrs` - user space attributes
/// * `attrs` - kernel space attributes
/// # Returns
/// * `TeeResult` - the result of the operation
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
    tee_debug!(
        "copy_in_attrs: usr_attrs: {:#?}, attrs: {:#?}",
        usr_attrs,
        attrs
    );
    Ok(())
}

enum attr_usage {
    ATTR_USAGE_POPULATE = 0,
    ATTR_USAGE_GENERATE_KEY = 1,
}

fn tee_svc_cryp_check_attr(
    usage: attr_usage,
    type_props: &tee_cryp_obj_type_props,
    attrs: &[TEE_Attribute],
) -> TeeResult {
    let mut required_flag = 0;
    let mut opt_flag = 0;
    let mut all_opt_needed = false;
    let mut req_attrs: u32 = 0;
    let mut opt_grp_attrs: u32 = 0;
    let mut attrs_found: u32 = 0;
    let mut n: usize = 0;
    let mut bit: u32 = 0;
    let mut flags: u32 = 0;
    let mut idx: isize = 0;

    match usage {
        attr_usage::ATTR_USAGE_POPULATE => {
            required_flag = TEE_TYPE_ATTR_REQUIRED;
            opt_flag = TEE_TYPE_ATTR_OPTIONAL_GROUP;
            all_opt_needed = true;
        }
        attr_usage::ATTR_USAGE_GENERATE_KEY => {
            required_flag = TEE_TYPE_ATTR_GEN_KEY_REQ;
            opt_flag = TEE_TYPE_ATTR_GEN_KEY_OPT;
            all_opt_needed = false;
        }
    }

    // First find out which attributes are required and which belong to
    // the optional group
    for n in 0..type_props.num_type_attrs as usize {
        bit = 1 << n;
        flags = type_props.type_attrs[n].flags as u32;

        if flags & required_flag != 0 {
            req_attrs |= bit;
        } else if flags & opt_flag != 0 {
            opt_grp_attrs |= bit;
        }
    }

    // Verify that all required attributes are in place and
    // that the same attribute isn't repeated.
    for n in 0..attrs.len() {
        idx = tee_svc_cryp_obj_find_type_attr_idx(attrs[n].attributeID as u32, type_props);

        // attribute not defined in current object type
        if idx < 0 {
            return Err(TEE_ERROR_ITEM_NOT_FOUND);
        }

        bit = 1 << idx;

        // attribute not repeated
        if (attrs_found & bit) != 0 {
            return Err(TEE_ERROR_ITEM_NOT_FOUND);
        }

        // Attribute not defined in current object type for this
        // usage.
        if (bit & (req_attrs | opt_grp_attrs)) == 0 {
            return Err(TEE_ERROR_ITEM_NOT_FOUND);
        }

        attrs_found |= bit;
    }

    // Required attribute missing
    if (attrs_found & req_attrs) != req_attrs {
        return Err(TEE_ERROR_ITEM_NOT_FOUND);
    }

    // If the flag says that "if one of the optional attributes are included
    // all of them has to be included" this must be checked.
    if all_opt_needed
        && (attrs_found & opt_grp_attrs) != 0
        && (attrs_found & opt_grp_attrs) != opt_grp_attrs
    {
        return Err(TEE_ERROR_ITEM_NOT_FOUND);
    }

    Ok(())
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

fn tee_svc_cryp_obj_populate_type(
    obj: &mut tee_obj,
    type_props: &tee_cryp_obj_type_props,
    attrs: &[TEE_Attribute],
) -> TeeResult {
    let mut have_attrs: u32 = 0;
    let mut obj_size: usize = 0;
    let mut idx: isize = 0;

    if obj.attr.is_empty() {
        return Err(TEE_ERROR_BAD_STATE);
    }

    for attr in attrs {
        // find attribute index in type properties
        tee_debug!(
            "tee_svc_cryp_obj_populate_type, find attribute index: attr_id: {:06X?}, type_props: \
             {:#?}",
            attr.attributeID,
            type_props
        );
        idx = tee_svc_cryp_obj_find_type_attr_idx(attr.attributeID, type_props);
        tee_debug!(
            "tee_svc_cryp_obj_populate_type, attribute index: {:#X?}",
            idx
        );
        // attribute not defined in current object type
        if idx < 0 {
            return Err(TEE_ERROR_ITEM_NOT_FOUND);
        }
        have_attrs |= bit32(idx as u32);

        let mut attr_ref = obj.attr[0].get_attr_by_id(attr.attributeID as c_ulong)?;
        if attr.attributeID & TEE_ATTR_FLAG_VALUE != 0 {
            // change attrs.content.value to &[]
            let value: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    (&attr.content.value as *const tee_raw_sys::Value) as *const u8,
                    core::mem::size_of::<tee_raw_sys::Value>(),
                )
            };
            attr_ref.from_user(value)?;
        } else {
            // change attrs.content.ref to &[]
            let buffer: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    (attr.content.memref.buffer as *const u8) as *const u8,
                    attr.content.memref.size,
                )
            };
            attr_ref.from_user(buffer)?;
        }

        // The attribute that gives the size of the object is
        // flagged with TEE_TYPE_ATTR_SIZE_INDICATOR.
        if type_props.type_attrs[idx as usize].flags & TEE_TYPE_ATTR_SIZE_INDICATOR as u16 != 0 {
            // There should be only one
            if obj_size != 0 {
                return Err(TEE_ERROR_BAD_STATE);
            }

            // For ECDSA/ECDH we need to translate curve into
            // object size
            if attr.attributeID == TEE_ATTR_ECC_CURVE {
                // get ECC curve size
                obj_size = get_ec_key_size(unsafe { attr.content.value.a })?;
            } else {
                let obj_type: TEE_ObjectType = obj.info.objectType;
                let sz: usize = obj.info.maxObjectSize as usize;

                obj_size = unsafe { attr.content.memref.size } * 8;
                if is_gp_legacy_des_key_size(obj_type, sz) {
                    obj_size -= obj_size / 8;
                }
            }
            if obj_size > obj.info.maxObjectSize as usize {
                return Err(TEE_ERROR_BAD_STATE);
            }
            check_key_size(type_props, obj_size)?;
        }
        // Bignum attributes limited by the number of bits in
        // o->info.objectSize are flagged with
        // TEE_TYPE_ATTR_BIGNUM_MAXBITS.
        if type_props.type_attrs[idx as usize].flags & TEE_TYPE_ATTR_BIGNUM_MAXBITS as u16 != 0 {
            if crypto_bignum_num_bits(attr_ref.as_bignum().ok_or(TEE_ERROR_BAD_STATE)?)?
                > obj.info.maxObjectSize as usize
            {
                return Err(TEE_ERROR_BAD_STATE);
            }
        }

        obj.have_attrs |= have_attrs;
        obj.info.objectSize = obj_size as u32;
        // In GP Internal API Specification 1.0 the partity bits aren't
        // counted when telling the size of the key in bits so remove the
        // parity bits here.
        if is_gp_legacy_des_key_size(obj.info.objectType, obj.info.maxObjectSize as usize) {
            obj.info.objectSize -= obj.info.objectSize / 8;
        }
    }

    Ok(())
}

/// Populate a transient object
///
/// # Arguments
/// * `obj_id` - the object id
/// * `user_attrs` - the user attributes
/// * `attr_count` - the number of attributes
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_cryp_obj_populate(
    obj_id: c_ulong,
    user_attrs: *mut utee_attribute,
    attr_count: c_ulong,
) -> TeeResult {
    let usr_attrs: &[utee_attribute] = unsafe {
        core::slice::from_raw_parts(user_attrs as *const utee_attribute, attr_count as usize)
    };

    let o_arc = tee_obj_get(obj_id as tee_obj_id_type)?;
    let mut o = o_arc.lock();

    // Must be a transient object
    if o.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT != 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    // Must not be initialized already
    if o.info.handleFlags & TEE_HANDLE_FLAG_INITIALIZED != 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    let type_props = tee_svc_find_type_props(o.info.objectType).ok_or(TEE_ERROR_NOT_IMPLEMENTED)?;

    let attr_null: TEE_Attribute = TEE_Attribute::default();
    let mut attrs: Box<[TEE_Attribute]> =
        vec![attr_null; usr_attrs.len() as usize].into_boxed_slice();
    copy_in_attrs(&mut user_ta_ctx::default(), usr_attrs, &mut attrs)?;

    tee_svc_cryp_check_attr(attr_usage::ATTR_USAGE_POPULATE, type_props, &attrs)?;

    tee_svc_cryp_obj_populate_type(&mut o, type_props, &attrs)?;

    o.info.handleFlags |= TEE_HANDLE_FLAG_INITIALIZED;

    Ok(())
}

/// Copy an object from source to destination
///
/// # Arguments
/// * `dst` - the destination object id
/// * `src` - the source object id
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_cryp_obj_copy(dst: c_ulong, src: c_ulong) -> TeeResult {
    let dst_o_arc = tee_obj_get(dst as tee_obj_id_type)?;
    let mut dst_o = dst_o_arc.lock();
    let src_o_arc = tee_obj_get(src as tee_obj_id_type)?;
    let mut src_o = src_o_arc.lock();

    if src_o.info.handleFlags & TEE_HANDLE_FLAG_INITIALIZED == 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }
    if dst_o.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT != 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }
    if dst_o.info.handleFlags & TEE_HANDLE_FLAG_INITIALIZED != 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    tee_obj_attr_copy_from(&mut dst_o, &mut src_o)?;
    dst_o.info.handleFlags |= TEE_HANDLE_FLAG_INITIALIZED;
    dst_o.info.objectSize = src_o.info.objectSize;
    if src_o.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT != 0 {
        let pobj = src_o.pobj.as_ref().ok_or(TEE_ERROR_BAD_STATE)?.read();
        with_pobj_usage_lock(pobj.flags, || {
            dst_o.info.objectUsage = pobj.obj_info_usage;
        });
    } else {
        dst_o.info.objectUsage = src_o.info.objectUsage;
    }
    Ok(())
}

fn check_pub_rsa_key(e: &BigNum) -> TeeResult {
    let n = crypto_bignum_num_bytes(e)?;
    let mut bin_key = [0u8; 256 / 8];

    // NIST SP800-56B requires public RSA key to be an odd integer in
    // the range 65537 <= e < 2^256. AOSP requires implementations to
    // support public exponents >= 3, which can be allowed by enabling
    // CFG_RSA_PUB_EXPONENT_3.
    if n > bin_key.len() || n < 1 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    crypto_bignum_bn2bin(e, &mut bin_key)?;

    if (bin_key[n - 1] & 1) == 0 {
        // key must be odd
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    if n <= 3 {
        let mut min_key: u32 = 65537;
        let mut key: u32 = 0;

        if CFG_RSA_PUB_EXPONENT_3 {
            min_key = 3;
        }

        for m in 0..n {
            key <<= 8;
            key |= bin_key[m] as u32;
        }

        if key < min_key {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }
    }

    Ok(())
}

pub fn tee_svc_obj_generate_key_rsa(
    o: &mut tee_obj,
    type_props: &tee_cryp_obj_type_props,
    key_size: u32,
    params: &[TEE_Attribute],
    object_type: u32,
) -> TeeResult {
    tee_debug!("tee_svc_obj_generate_key_rsa: params: {:#?}", params);
    tee_svc_cryp_obj_populate_type(o, type_props, params)?;

    if o.attr.is_empty() {
        return Err(TEE_ERROR_BAD_STATE);
    }
    let pub_exp = get_attribute(o, type_props, TEE_ATTR_RSA_PUBLIC_EXPONENT);

    let mut rsa_key = match &mut o.attr[0] {
        TeeCryptObj::rsa_keypair(key) => key,
        _ => return Err(TEE_ERROR_BAD_STATE),
    };

    tee_debug!("tee_svc_obj_generate_key_rsa: pub_exp: {:X?}", pub_exp);
    if pub_exp != 0 {
        check_pub_rsa_key(&rsa_key.e)?;
    } else {
        // set default public exponent to 65537 (big endian)
        // Use to_be_bytes() directly on the original value to get big-endian bytes
        let e_bytes = 65537u32.to_be_bytes();
        crypto_bignum_bin2bn(&e_bytes, &mut rsa_key.e)?;
    }

    crypto_acipher_gen_rsa_key(rsa_key, key_size as usize)?;

    // Set bits for all known attributes for this object type
    o.have_attrs = (1 << type_props.num_type_attrs) - 1;

    Ok(())
}

pub fn tee_svc_obj_generate_key_ecc(
    o: &mut tee_obj,
    type_props: &tee_cryp_obj_type_props,
    key_size: u32,
    params: &[TEE_Attribute],
    object_type: u32,
) -> TeeResult {
    tee_svc_cryp_obj_populate_type(o, type_props, params)?;

    if o.attr.is_empty() {
        return Err(TEE_ERROR_BAD_STATE);
    }

    let mut tee_ecc_key = match &mut o.attr[0] {
        TeeCryptObj::ecc_keypair(key) => key,
        _ => return Err(TEE_ERROR_BAD_STATE),
    };

    crypto_acipher_gen_ecc_key(tee_ecc_key, key_size as usize, object_type)?;

    set_attribute(o, type_props, TEE_ATTR_ECC_PRIVATE_VALUE);
    set_attribute(o, type_props, TEE_ATTR_ECC_PUBLIC_VALUE_X);
    set_attribute(o, type_props, TEE_ATTR_ECC_PUBLIC_VALUE_Y);
    set_attribute(o, type_props, TEE_ATTR_ECC_CURVE);

    Ok(())
}

/// Generates a cryptographic key for the specified secure object.
/// The attributes of key is stored in the object attr(tee_obj.attr).
///
/// # Parameters
/// - `obj`: Handle of the object (object ID).
/// - `key_size`: The length of the key to be generated (in bits).
/// - `usr_params`: Pointer to an array of user-supplied key attributes.
/// - `param_count`: The number of attributes in the array.
///
/// # Returns
/// Returns a `TeeResult` indicating success or failure.
pub fn syscall_obj_generate_key(
    obj: c_ulong,
    key_size: c_ulong,
    usr_params: *const utee_attribute,
    param_count: c_ulong,
) -> TeeResult {
    let mut byte_size: usize = 0;
    let o = tee_obj_get(obj as tee_obj_id_type)?;

    let type_props = {
        let o_guard = o.lock();
        // Must be a transient object
        if o_guard.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT != 0 {
            return Err(TEE_ERROR_BAD_STATE);
        }

        // Must not be initialized already
        if o_guard.info.handleFlags & TEE_HANDLE_FLAG_INITIALIZED != 0 {
            return Err(TEE_ERROR_BAD_STATE);
        }

        // Find description of object
        tee_svc_find_type_props(o_guard.info.objectType).ok_or(TEE_ERROR_NOT_SUPPORTED)?
    };

    // Check that key_size follows restrictions
    check_key_size(type_props, key_size as _)?;

    let attr_null: TEE_Attribute = TEE_Attribute::default();
    let mut attrs: Box<[TEE_Attribute]> = vec![attr_null; param_count as usize].into_boxed_slice();
    let usr_attrs_slice: &[utee_attribute] = unsafe {
        core::slice::from_raw_parts(usr_params as *const utee_attribute, param_count as usize)
    };
    copy_in_attrs(&mut user_ta_ctx::default(), usr_attrs_slice, &mut attrs)?;
    tee_svc_cryp_check_attr(attr_usage::ATTR_USAGE_GENERATE_KEY, type_props, &attrs).inspect_err(
        |e| {
            tee_debug!("tee_svc_cryp_check_attr error: {:X?}", e);
        },
    )?;

    let mut o_guard = o.lock();
    let object_type = o_guard.info.objectType;
    match object_type {
        TEE_TYPE_AES
        | TEE_TYPE_DES
        | TEE_TYPE_DES3
        | TEE_TYPE_SM4
        | TEE_TYPE_HMAC_MD5
        | TEE_TYPE_HMAC_SHA1
        | TEE_TYPE_HMAC_SHA224
        | TEE_TYPE_HMAC_SHA256
        | TEE_TYPE_HMAC_SHA384
        | TEE_TYPE_HMAC_SHA512
        | TEE_TYPE_HMAC_SHA3_224
        | TEE_TYPE_HMAC_SHA3_256
        | TEE_TYPE_HMAC_SHA3_384
        | TEE_TYPE_HMAC_SHA3_512
        | TEE_TYPE_HMAC_SM3
        | TEE_TYPE_GENERIC_SECRET => {
            byte_size = key_size as usize / 8;

            // In GP Internal API Specification 1.0 the partity bits
            // aren't counted when telling the size of the key in bits.
            if is_gp_legacy_des_key_size(object_type, key_size as _) {
                byte_size = (key_size as usize + key_size as usize / 7) / 8;
            }

            // check attr
            if o_guard.attr.is_empty() {
                return Err(TEE_ERROR_BAD_STATE);
            }

            // get secret value
            let mut secret_value = match &mut o_guard.attr[0] {
                TeeCryptObj::obj_secret(secret_value) => secret_value,
                _ => return Err(TEE_ERROR_BAD_STATE),
            };

            if byte_size > secret_value.secret().alloc_size as usize {
                return Err(TEE_ERROR_EXCESS_DATA);
            }

            // read random data
            crypto_rng_read(&mut secret_value.data_mut()[..byte_size])?;

            secret_value.secret_mut().key_size = byte_size as _;

            // Set bits for all known attributes for this object type
            o_guard.have_attrs = (1 << type_props.num_type_attrs as u32) - 1;
        }
        TEE_TYPE_RSA_KEYPAIR => {
            tee_svc_obj_generate_key_rsa(
                &mut o_guard,
                type_props,
                key_size as _,
                &attrs,
                object_type,
            )
            .inspect_err(|e| {
                tee_debug!("tee_svc_obj_generate_key_rsa error: {:X?}", e);
            })?;
        }
        TEE_TYPE_DSA_KEYPAIR => {
            // mbedtls do not support DSA key generation
            todo!()
        }
        TEE_TYPE_DH_KEYPAIR => {
            // mbedtls do not support DH key generation
            todo!()
        }
        TEE_TYPE_ECDSA_KEYPAIR
        | TEE_TYPE_ECDH_KEYPAIR
        | TEE_TYPE_SM2_DSA_KEYPAIR
        | TEE_TYPE_SM2_KEP_KEYPAIR
        | TEE_TYPE_SM2_PKE_KEYPAIR => {
            tee_svc_obj_generate_key_ecc(
                &mut o_guard,
                type_props,
                key_size as _,
                &attrs,
                object_type,
            )?;
        }
        TEE_TYPE_X25519_KEYPAIR => {
            todo!()
        }
        TEE_TYPE_X448_KEYPAIR => {
            todo!()
        }
        TEE_TYPE_ED25519_KEYPAIR => {
            todo!()
        }
        _ => {
            return Err(TEE_ERROR_BAD_FORMAT);
        }
    }

    o_guard.info.objectSize = key_size as _;
    o_guard.info.handleFlags |= TEE_HANDLE_FLAG_INITIALIZED;
    Ok(())
}

#[cfg(feature = "tee_test")]
fn long2byte(value: u64, ch: &mut [u8]) -> u32 {
    // Convert value to big-endian byte array
    // Store valid bytes from the beginning of the array (ch[0..len])
    // Example: e = 65539 (0x00010003) -> ch[0..3] = [0x01, 0x00, 0x03], len = 3
    if value == 0 {
        return 0;
    }

    // Calculate the number of bytes needed
    let mut num_bytes = 0;
    let mut temp = value;
    while temp > 0 {
        num_bytes += 1;
        temp >>= 8;
    }

    // Store bytes from most significant to least significant, starting at ch[0]
    let mut temp = value;
    for i in (0..num_bytes).rev() {
        ch[i] = (temp & 0xff) as u8;
        temp >>= 8;
    }

    num_bytes as u32
}

#[cfg(feature = "tee_test")]
fn tee_init_ref_attribute(attr: &mut utee_attribute, attributeID: u32, buffer: &[u8], length: u32) {
    if (attributeID & TEE_ATTR_FLAG_VALUE) != 0 {
        panic!("attributeID is value attribute");
    }
    attr.attribute_id = attributeID;
    attr.a = buffer.as_ptr() as u64;
    attr.b = length as u64;
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
            // test attr_bytes from u32
            let a_u32: u32 = 0xAABBCCDD;
            let attr = &a_u32;
            let attr_bytes: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    (attr as *const u32) as *const u8,
                    core::mem::size_of::<u32>(),
                )
            };
            let value: [u32; 2] = [unsafe { *(attr_bytes.as_ptr() as *const u32) }, 0];
            assert_eq!(value[0], 0xAABBCCDD as u32);
            assert_eq!(size_of_val(&value), 8);

            // test tee_u32_to_big_endian
            let val: u32 = 0x12345678;
            let be_val = tee_u32_to_big_endian(val);
            assert_eq!(be_val, 0x78563412);
            assert_eq!(be_val.as_bytes(), &[0x12, 0x34, 0x56, 0x78]);

            // test op_u32_to_binary_helper
            let mut buffer: [u8; 8] = [0; 8];
            let mut offs: size_t = 0;
            op_u32_to_binary_helper(0x11223344, &mut buffer, &mut offs).unwrap();
            assert_eq!(offs, 4);
            assert_eq!(&buffer[0..4], &[0x11, 0x22, 0x33, 0x44]);

            // test op_u32_to_binary_helper with offset
            op_u32_to_binary_helper(0x55667788, &mut buffer, &mut offs).unwrap();
            assert_eq!(offs, 8);
            assert_eq!(&buffer[4..8], &[0x55, 0x66, 0x77, 0x88]);

            // test overflow
            let mut small_buffer: [u8; 4] = [0; 4];
            let mut offs_overflow: size_t = usize::MAX - 2;
            let result = op_u32_to_binary_helper(0x99AABBCC, &mut small_buffer, &mut offs_overflow);
            assert_eq!(result.err(), Some(TEE_ERROR_OVERFLOW));

            // test insufficient buffer
            let mut insufficient_buffer: [u8; 4] = [0; 4];
            let mut offs_insufficient: size_t = 2;
            let result = op_u32_to_binary_helper(0x11223344, &mut insufficient_buffer, &mut offs_insufficient);
            assert!(result.is_ok());
            assert_eq!(offs_insufficient, 6);
            assert_eq!(&insufficient_buffer, &[0; 4]); // buffer remains unchanged
        }
    }

    test_fn! {
        using TestResult;

        fn test_tee_svc_find_type_props() {
            let props = tee_svc_find_type_props(TEE_TYPE_AES);
            assert!(props.is_some());
            let props = props.unwrap();
            assert_eq!(props.obj_type, TEE_TYPE_AES);
            assert_eq!(props.min_size, 128);
            assert_eq!(props.max_size, 256);
        }
    }

    test_fn! {
        using TestResult;

        fn test_op_attr_secret_value_from_user() {
            // 测试基础数据
            let user_key: [u8; 16] = [0xAA; 16];
            let mut secret_wrapper = tee_cryp_obj_secret_wrapper::new(32);

            // 从用户空间导入密钥
            op_attr_secret_value_from_user(&mut secret_wrapper, &user_key).unwrap();

            // 验证密钥大小和内容
            assert_eq!(secret_wrapper.secret().key_size, 16);
            assert_eq!(secret_wrapper.secret().alloc_size, 32);
            assert_eq!(&secret_wrapper.data()[..16], &user_key);

            // 测试长度超出分配大小的情况
            let long_user_key: [u8; 40] = [0xBB; 40];
            let result = op_attr_secret_value_from_user(&mut secret_wrapper, &long_user_key);
            assert_eq!(result.err(), Some(TEE_ERROR_SHORT_BUFFER));
        }
    }

    test_fn! {
        using TestResult;

        fn test_op_attr_secret_value_to_user() {
            // 准备测试数据
            let mut secret_wrapper = tee_cryp_obj_secret_wrapper::new(32);
            let key_data: [u8; 16] = [0xCC; 16];
            // 手动设置密钥数据和大小
            {
                let data_slice = secret_wrapper.data_mut();
                data_slice[..16].copy_from_slice(&key_data);
                secret_wrapper.secret_mut().key_size = 16;
            }
            // 测试函数
            let mut size: u64 = 0;
            // 第一次调用，size 为 0，应该返回 TEE_ERROR_SHORT_BUFFER
            let result = op_attr_secret_value_to_user(&secret_wrapper, None, &mut size);
            assert_eq!(result.err(), Some(TEE_ERROR_SHORT_BUFFER));

            // 第二次调用，提供足够大的 buffer
            let mut user_buffer: [u8; 32] = [0; 32];
            size = 32;
            let result = op_attr_secret_value_to_user(
                &secret_wrapper,
                Some(&mut user_buffer),
                &mut size,
            );
            assert!(result.is_ok());
            // 验证返回的 size 和数据内容
            assert_eq!(size, 16);
            assert_eq!(&user_buffer[0..16], &key_data[0..16]);
        }
    }

    test_fn! {
        using TestResult;
        fn test_op_attr_secret_value_to_binary() {
            // 准备测试数据
            let mut secret_wrapper = tee_cryp_obj_secret_wrapper::new(32);
            let key_data: [u8; 16] = [0xDD; 16];
            // 手动设置密钥数据和大小
            {
                let data_slice = secret_wrapper.data_mut();
                data_slice[..16].copy_from_slice(&key_data);
                secret_wrapper.secret_mut().key_size = 16;
            }
            // 准备目标缓冲区
            let mut buffer: [u8; 64] = [0; 64];
            let mut offs: size_t = 0;
            // 调用函数进行序列化
            let result = op_attr_secret_value_to_binary(&secret_wrapper, &mut buffer, &mut offs);
            assert!(result.is_ok());
            // 验证偏移量
            assert_eq!(offs, 4 + 16); // 4 bytes for key_size + 16 bytes for key data
            // 验证序列化内容
            let expected_key_size_bytes: [u8; 4] = [0x00, 0x00, 0x00, 0x10]; // big-endian
            assert_eq!(&buffer[0..4], &expected_key_size_bytes);
            assert_eq!(&buffer[4..20], &key_data);

            // test op_attr_secret_value_from_binary
            let mut new_secret_wrapper = tee_cryp_obj_secret_wrapper::new(32);
            let mut offs_from: size_t = 0;
            let result = op_attr_secret_value_from_binary(
                &mut new_secret_wrapper,
                &buffer,
                &mut offs_from,
            );
            assert!(result.is_ok());
            // 验证偏移量
            assert_eq!(offs_from, 4 + 16);
            // 验证反序列化内容
            assert_eq!(new_secret_wrapper.secret().key_size, 16);
            assert_eq!(new_secret_wrapper.secret().alloc_size, 32);
            assert_eq!(&new_secret_wrapper.data()[..16], &key_data);
        }
    }

    test_fn! {
        using TestResult;

        fn test_op_u32_from_binary_helper() {
            let data: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
            let mut offs: size_t = 0;
            let mut value: u32 = 0;

            // 第一次读取
            let result = op_u32_from_binary_helper(&mut value, &data, &mut offs);
            assert!(result.is_ok());
            assert_eq!(value, 0x11223344);
            assert_eq!(offs, 4);

            // 第二次读取
            let result = op_u32_from_binary_helper(&mut value, &data, &mut offs);
            assert!(result.is_ok());
            assert_eq!(value, 0x55667788);
            assert_eq!(offs, 8);

            // 测试读取超出边界
            let result = op_u32_from_binary_helper(&mut value, &data, &mut offs);
            assert_eq!(result.err(), Some(TEE_ERROR_BAD_PARAMETERS));

            // call op_u32_to_binary_helper
            let mut buffer: [u8; 4] = [0; 4];
            let mut offs_write: size_t = 0;
            op_u32_to_binary_helper(0x99AABBCC, &mut buffer, &mut offs_write).unwrap();
            assert_eq!(offs_write, 4);
            assert_eq!(&buffer, &[0x99, 0xAA, 0xBB, 0xCC]);
            // read back
            let mut read_value: u32 = 0;
            let mut offs_read: size_t = 0;
            let result = op_u32_from_binary_helper(&mut read_value, &buffer, &mut offs_read);
            assert!(result.is_ok());
            assert_eq!(read_value, 0x99AABBCC as u32);
            assert_eq!(offs_read, 4);
        }
    }

    test_fn! {
        using TestResult;

        fn test_op_attr_value_to_user() {
            let mut attr: [u8; 8] = [0; 8];
            // 设置属性值为 0x11223344
            let value: u32 = 0x11223344;
            let value_bytes: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    &value as *const u32 as *const u8,
                    core::mem::size_of::<u32>(),
                )
            };
            attr[..4].copy_from_slice(value_bytes);

            let mut size: u64 = 8;
            let mut user_buffer: [u8; 8] = [0; 8];

            let result = op_attr_value_to_user(&attr, &mut user_buffer, &mut size);
            assert!(result.is_ok());
            assert_eq!(size, 8);
            assert_eq!(&user_buffer[..4], value_bytes);
        }
    }

    test_fn! {
        using TestResult;

        fn test_op_attr_value_from_binary() {
            let mut attr: [u8; 8] = [0; 8];
            let value: u32 = 0x11223344;
            let value_bytes: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    &value as *const u32 as *const u8,
                    core::mem::size_of::<u32>(),
                )
            };

            //attr[..4].copy_from_slice(value_bytes);
            let mut offs: size_t = 0;
            let result = op_attr_value_from_binary(&mut attr, &value_bytes, &mut offs);
            // info!("result: {:?}, offs: {}, attr: {:?}", result, offs, attr);
            assert!(result.is_ok());
            assert_eq!(offs, 4);
            assert_eq!(&attr[..4], &[0x11, 0x22, 0x33, 0x44]);

            // // test op_attr_value_to_binary
            let mut buffer: [u8; 8] = [0; 8];
            let mut offs_write: size_t = 0;
            let result = op_attr_value_to_binary(&attr, &mut buffer, &mut offs_write);
            assert!(result.is_ok());
            assert_eq!(offs_write, 4);
            assert_eq!(&buffer[..4], value_bytes);
        }
    }

    test_fn! {
        using TestResult;

        fn test_tee_obj_set_type() {
            // test with TEE_TYPE_AES
            let mut obj = tee_obj::default();
            let result = tee_obj_set_type(&mut obj, TEE_TYPE_AES, 256);
            assert!(result.is_ok());
            assert_eq!(obj.info.objectType, TEE_TYPE_AES);
            assert_eq!(obj.info.maxObjectSize, 256);
            assert_eq!(obj.info.objectUsage, TEE_USAGE_DEFAULT);
            assert_eq!(obj.info.handleFlags, 0);
            assert_eq!(obj.info.dataSize, 0);
            assert_eq!(obj.info.dataPosition, 0);
            assert_eq!(obj.attr.len(), 1);
            assert!(matches!(obj.attr[0], TeeCryptObj::obj_secret(_)));

            let mut obj = tee_obj::default();
            let result = tee_obj_set_type(&mut obj, TEE_TYPE_ECDSA_PUBLIC_KEY, 256);
            assert!(result.is_ok());
            assert_eq!(obj.info.objectType, TEE_TYPE_ECDSA_PUBLIC_KEY);
            assert_eq!(obj.info.maxObjectSize, 256);
            assert_eq!(obj.info.objectUsage, TEE_USAGE_DEFAULT);
            assert_eq!(obj.info.handleFlags, 0);
            assert_eq!(obj.info.dataSize, 0);
            assert_eq!(obj.info.dataPosition, 0);
            assert_eq!(obj.attr.len(), 1);
            assert!(matches!(obj.attr[0], TeeCryptObj::ecc_public_key(_)));
        }
    }

    test_fn! {
        using TestResult;

        fn test_cryptoattrref_u32() {
            // test CryptoAttrRef::U32
            let mut value: u32 = 0;
            let value_c: [u32; 2] = [0x11223344, 0];
            let value_bytes: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    &value_c as *const [u32; 2] as *const u8,
                    core::mem::size_of::<[u32; 2]>(),
                )
            };
            {
                let mut attr_ref = CryptoAttrRef::U32(&mut value);
                let result = attr_ref.from_user(value_bytes);
                assert!(result.is_ok());
            }
            assert_eq!(value, 0x11223344);

            let mut buffer: [u8; 8] = [0; 8];
            let mut size: u64 = 8;
            {
                let attr_ref = CryptoAttrRef::U32(&mut value);
                let result = attr_ref.to_user(&mut buffer, &mut size);
                assert!(result.is_ok());
            }
            assert_eq!(size, 8);
            assert_eq!(&buffer, value_bytes);
        }
    }

    test_fn! {
        using TestResult;

        fn test_cryptoattrref_bignum() {
            // test CryptoAttrRef::BigNum
            let mut bn = BigNum::new(0x11223344).unwrap();
            let mut buffer: [u8; 4] = [0; 4];
            let mut size: u64 = 4;
            let result = bn.to_user(&mut buffer, &mut size);
            assert!(result.is_ok());
            assert_eq!(size, 4);
            // assert_eq!(&buffer, value_bytes);
            // from user with buffer
            let mut bn_from = BigNum::new(0).unwrap();
            let result = bn_from.from_user(&buffer);
            assert!(result.is_ok());
            assert_eq!(bn_from, bn);
        }
    }

    test_fn! {
        using TestResult;

        fn test_secret_value() {
            // set secret value data to
            let mut secret = tee_cryp_obj_secret_wrapper::new(16);
            secret.secret_mut().key_size = 16;
            secret.data_mut()[..16].copy_from_slice(&[0xaa; 16]);

            // 1. test tee_cryp_obj_secret_wrapper to user
            // - test to_user
            let mut buffer: [u8; 16] = [0; 16];
            let mut size: u64 = 16;
            let result = secret.to_user(&mut buffer, &mut size);
            assert!(result.is_ok());
            assert_eq!(size, 16);
            assert_eq!(&buffer[..16], &secret.data()[..16]);
            // - test from_user
            let mut secret_dest = tee_cryp_obj_secret_wrapper::new(16);
            let result = secret_dest.from_user(&buffer);
            assert!(result.is_ok());
            assert_eq!(secret_dest.secret().key_size, secret.secret().key_size);
            assert_eq!(&secret_dest.data()[..16], &secret.data()[..16]);
            //  - test to_binary
            let mut data: [u8; 16+size_of::<u32>()] = [0x55; 16+size_of::<u32>()];
            let mut offs: usize = 0;
            let result = secret_dest.to_binary(&mut data, &mut offs);
            assert!(result.is_ok());
            assert_eq!(offs, 16+size_of::<u32>());
            assert_eq!(&data[..size_of::<u32>()], &secret_dest.secret().key_size.to_be_bytes());
            assert_eq!(&data[size_of::<u32>()..16+size_of::<u32>()], &secret_dest.data()[..16]);
            //  - test from_binary
            let mut secret_from = tee_cryp_obj_secret_wrapper::new(16);
            offs = 0;
            let result = secret_from.from_binary(&data, &mut offs);
            assert!(result.is_ok());
            assert_eq!(offs, 16+size_of::<u32>());
            assert_eq!(secret_from.secret().key_size, secret_dest.secret().key_size);
            assert_eq!(&secret_from.data()[..16], &secret_dest.data()[..16]);

            // - test from_obj
            let mut secret_dest = tee_cryp_obj_secret_wrapper::new(16);
            let result = secret_dest.from_obj(&TeeCryptObjAttr::secret_value(secret_from.clone()));
            assert!(result.is_ok());
            assert_eq!(secret_dest.secret().key_size, secret_from.secret().key_size);
            assert_eq!(&secret_dest.data(), &secret_from.data());

            // 2. test CryptoAttrRef::SecretValue
            // clear buffer
            buffer.fill(0);
            size = 16;
            let mut attr_ref = CryptoAttrRef::SecretValue(&mut secret);
            let result = attr_ref.to_user(&mut buffer, &mut size);
            assert!(result.is_ok());
            assert_eq!(size, 16);
            assert_eq!(&buffer[..16], &secret.data()[..16]);
        }
    }

    test_fn! {
        using TestResult;

        fn test_syscall_cryp_obj_alloc() {
            let mut obj_id: c_uint = 0;
            let result = syscall_cryp_obj_alloc(TEE_TYPE_ECDSA_PUBLIC_KEY as _, 256, &mut obj_id);
            assert!(result.is_ok());
            let obj_arc = tee_obj_get(obj_id as tee_obj_id_type).unwrap();
            let obj = obj_arc.lock();
            assert_eq!(obj.info.objectType, TEE_TYPE_ECDSA_PUBLIC_KEY);
            assert_eq!(obj.info.maxObjectSize, 256);
            assert_eq!(obj.info.objectUsage, TEE_USAGE_DEFAULT);
            assert_eq!(obj.info.handleFlags, 0);
            assert_eq!(obj.info.dataSize, 0);
            assert_eq!(obj.info.dataPosition, 0);
            assert_eq!(obj.attr.len(), 1);
            assert!(matches!(obj.attr[0], TeeCryptObj::ecc_public_key(_)));
        }
    }

    test_fn! {
        using TestResult;

        fn test_syscall_cryp_obj_get_attr() {
            let mut obj_id: c_uint = 0;
            let result = syscall_cryp_obj_alloc(TEE_TYPE_ECDSA_PUBLIC_KEY as _, 256, &mut obj_id);
            assert!(result.is_ok());
            let mut buffer: [u8; 8] = [1; 8];
            let mut size: u64 = 8;
            // TODO: need to implement syscall_cryp_obj_get_attr
            // let result = syscall_cryp_obj_get_attr(obj_id, TEE_ATTR_ECC_CURVE as c_ulong, &mut buffer, &mut size);
            // info!("result: {:x?}, size: {}, buffer: {:?}", result, size, buffer);
            // assert!(result.is_ok());
            // assert_eq!(size, 8);
            // assert_eq!(&buffer[..4], &[0x00, 0x00, 0x00, 0x00]);
        }
    }

    test_fn! {
        using TestResult;

        fn test_syscall_cryp_generate_key_ecc_keypair() {
            // alloc sm2 key pair
            let mut obj_id: c_uint = 0;
            let result = syscall_cryp_obj_alloc(TEE_TYPE_SM2_DSA_KEYPAIR as _, 256, &mut obj_id);
            assert!(result.is_ok());
            // sm2 no need usr_params
            let result = syscall_obj_generate_key(obj_id as c_ulong, 256, core::ptr::null(), 0);
            assert!(result.is_ok());
            // get attr from obj
            let obj_arc = tee_obj_get(obj_id as tee_obj_id_type);
            assert!(obj_arc.is_ok());
            let obj_arc = obj_arc.unwrap();
            let obj = obj_arc.lock();
            assert_eq!(obj.info.objectType, TEE_TYPE_SM2_DSA_KEYPAIR);
            assert_eq!(obj.info.maxObjectSize, 256);
            assert_eq!(obj.info.objectUsage, TEE_USAGE_DEFAULT);
            assert_eq!(obj.attr.len(), 1);
            assert!(matches!(obj.attr[0], TeeCryptObj::ecc_keypair(_)));
            // get ecc_keypair from obj
            let ecc_keypair = match &obj.attr[0] {
                TeeCryptObj::ecc_keypair(ecc_keypair) => ecc_keypair,
                _ => panic!("ecc_keypair not found"),
            };
            assert_eq!(ecc_keypair.curve, TEE_ECC_CURVE_SM2);
            tee_debug!("ecc_keypair: {:#?}", ecc_keypair);
            let d_len = ecc_keypair.d.byte_length().unwrap();
            let x_len = ecc_keypair.x.byte_length().unwrap();
            let y_len = ecc_keypair.y.byte_length().unwrap();
            assert!(d_len == 31 || d_len == 32);
            assert!(x_len == 31 || x_len == 32);
            assert!(y_len == 31 || y_len == 32);
        }
    }

    // Helper function to test RSA keypair generation and verification
    fn test_rsa_keypair(key_size: usize, e: u64) -> TestResult {
        let mut e_bytes: [u8; 8] = [0; 8];
        let mut usr_params: [utee_attribute; 1] = [utee_attribute::default(); 1];

        let (mut usr_params, param_count) = {
            if e == 0 {
                (core::ptr::null(), 0)
            } else {
                let e_len = long2byte(e, &mut e_bytes);
                tee_init_ref_attribute(
                    &mut usr_params[0],
                    TEE_ATTR_RSA_PUBLIC_EXPONENT as u32,
                    &e_bytes[..e_len as usize],
                    e_len,
                );
                (usr_params.as_ptr(), 1)
            }
        };

        let mut obj_id: c_uint = 0;
        let result = syscall_cryp_obj_alloc(TEE_TYPE_RSA_KEYPAIR as _, key_size as _, &mut obj_id);
        assert!(result.is_ok());

        let result =
            syscall_obj_generate_key(obj_id as c_ulong, key_size as _, usr_params, param_count);
        assert!(result.is_ok());
        // get attr from obj
        let obj_arc = tee_obj_get(obj_id as tee_obj_id_type);
        assert!(obj_arc.is_ok());
        let obj_arc = obj_arc.unwrap();
        let obj = obj_arc.lock();
        assert_eq!(obj.info.objectType, TEE_TYPE_RSA_KEYPAIR);
        assert_eq!(obj.info.maxObjectSize, key_size as u32);
        assert_eq!(obj.info.objectUsage, TEE_USAGE_DEFAULT);
        assert_eq!(obj.attr.len(), 1);
        assert!(matches!(obj.attr[0], TeeCryptObj::rsa_keypair(_)));
        // get rsa_keypair from obj
        let rsa_keypair = match &obj.attr[0] {
            TeeCryptObj::rsa_keypair(rsa_keypair) => rsa_keypair,
            _ => panic!("rsa_keypair not found"),
        };
        assert_eq!(rsa_keypair.n.byte_length().unwrap(), key_size / 8);
        // print the keypai exponent
        tee_debug!("rsa_keypair: {:#?}", rsa_keypair);

        // test rsa_keypair.e equals to the exponent
        if e != 0 {
            assert_eq!(rsa_keypair.e.as_mpi(), &Mpi::new(e as i64).unwrap());
        } else {
            assert_eq!(rsa_keypair.e.as_mpi(), &Mpi::new(65537 as i64).unwrap());
        }

        TestResult::Ok
    }

    test_fn! {
        using TestResult;
        fn test_syscall_cryp_generate_key_rsa() {
            // step1: test without usr_params (use default exponent 65537)
            if let TestResult::Failed = test_rsa_keypair(2048, 0) {
                return TestResult::Failed;
            }
            // step2: test with custom exponent
            if let TestResult::Failed = test_rsa_keypair(2048, 65539) {
                return TestResult::Failed;
            }
            // step3: test with custom exponent 65537
            if let TestResult::Failed = test_rsa_keypair(2048, 65537) {
                return TestResult::Failed;
            }
        }
    }

    fn test_syscall_cryp_generate_secret_key(key_type: u32, key_size: usize) -> TestResult {
        tee_debug!(
            "test_syscall_cryp_generate_secret_key: key_type: {:?}, key_size: {:?}",
            key_type,
            key_size
        );
        // alloc sm4 key
        let mut obj_id: c_uint = 0;
        let result = syscall_cryp_obj_alloc(key_type as _, key_size as _, &mut obj_id);
        assert!(result.is_ok());
        // assert!(obj_id != 0);
        // secret key no need usr_params
        let result =
            syscall_obj_generate_key(obj_id as c_ulong, key_size as _, core::ptr::null(), 0);
        assert!(result.is_ok());
        // get attr from obj
        let obj_arc = tee_obj_get(obj_id as tee_obj_id_type);
        assert!(obj_arc.is_ok());
        let obj_arc = obj_arc.unwrap();
        let obj = obj_arc.lock();
        assert_eq!(obj.info.objectType, key_type);
        assert_eq!(obj.info.maxObjectSize, key_size as u32);
        assert_eq!(obj.info.objectUsage, TEE_USAGE_DEFAULT);
        assert_eq!(obj.attr.len(), 1);
        assert!(matches!(obj.attr[0], TeeCryptObj::obj_secret(_)));
        // get secret key from obj
        let secret_key = match &obj.attr[0] {
            TeeCryptObj::obj_secret(obj_secret) => obj_secret,
            _ => panic!("secret key not found"),
        };
        assert_eq!(secret_key.secret().key_size, (key_size / 8) as u32);
        tee_debug!("secret key: {:#?}", &obj.attr[0]);

        TestResult::Ok
    }

    test_fn! {
        using TestResult;

        fn test_syscall_cryp_generate_key_sm4() {
            if let TestResult::Failed = test_syscall_cryp_generate_secret_key(TEE_TYPE_SM4 as _, 128) {
                return TestResult::Failed;
            }
        }
    }

    test_fn! {
        using TestResult;

        fn test_syscall_cryp_generate_key_hmac_sm3() {
            if let TestResult::Failed = test_syscall_cryp_generate_secret_key(TEE_TYPE_HMAC_SM3 as _, 128) {
                return TestResult::Failed;
            }
        }
    }

    test_fn! {
        using TestResult;

        fn test_copy_in_attrs() {
            let tee_attr_value = TEE_Attribute {
                attributeID: 0,
                content: unsafe {
                    content {
                        value: Value {
                            a: 0 as u32,
                            b: 0 as u32,
                        },
                    }
                },
            };

            let mut attrs: [TEE_Attribute; 2] = [tee_attr_value; 2];
            let mut usr_attrs: [utee_attribute; 2] = [utee_attribute::default(); 2];
            // index 0 is value attribute
            usr_attrs[0].attribute_id = TEE_ATTR_FLAG_VALUE;
            usr_attrs[0].a = 0x11223344 as u64;
            usr_attrs[0].b = 0x55667788 as u64;
            // index 1 is memref attribute
            // allocate memory for memref
            let mut mem: [u8; 16] = [0xAA; 16];
            let mem_ptr = mem.as_ptr() as *mut c_void;
            usr_attrs[1].attribute_id &= !TEE_ATTR_FLAG_VALUE;
            usr_attrs[1].a = mem_ptr as u64;
            usr_attrs[1].b = mem.len() as u64;
            // copy in attrs
            let result = copy_in_attrs(&mut user_ta_ctx::default(), &usr_attrs, &mut attrs);
            assert!(result.is_ok());
            assert_eq!(attrs[0].attributeID, TEE_ATTR_FLAG_VALUE);
            assert_eq!(unsafe { attrs[0].content.value.a }, 0x11223344 as u32);
            assert_eq!(unsafe { attrs[0].content.value.b }, 0x55667788 as u32);
            assert_eq!(attrs[1].attributeID, 0);
            assert_eq!(unsafe { attrs[1].content.memref.buffer }, mem_ptr);
            assert_eq!(unsafe { attrs[1].content.memref.size }, mem.len());
        }
    }

    test_fn! {
        using TestResult;

        fn test_mpi_write_binary() {
            let mut m = Mpi::new(256).unwrap();
            let mut e: u32 = 0;
            unsafe {
                mpi_write_binary((&m).into(), &mut e as *mut u32 as *mut u8, size_of::<u32>());
            }
            let e = tee_u32_from_big_endian(e);
            assert_eq!(e, 256);
        }
    }
    tests_name! {
        TEST_TEE_SVC_CRYP;
        //------------------------
        test_tee_svc_cryp_utils,
        test_tee_svc_find_type_props,
        test_op_attr_secret_value_from_user,
        test_op_attr_secret_value_to_user,
        test_op_attr_secret_value_to_binary,
        test_op_u32_from_binary_helper,
        test_op_attr_value_to_user,
        test_op_attr_value_from_binary,
        test_tee_obj_set_type,
        test_cryptoattrref_u32,
        test_cryptoattrref_bignum,
        test_secret_value,
        test_syscall_cryp_obj_alloc,
        test_syscall_cryp_obj_get_attr,
        test_copy_in_attrs,
        test_syscall_cryp_generate_key_ecc_keypair,
        test_syscall_cryp_generate_key_rsa,
        test_syscall_cryp_generate_key_sm4,
        test_syscall_cryp_generate_key_hmac_sm3,
        test_mpi_write_binary,
    }
}
