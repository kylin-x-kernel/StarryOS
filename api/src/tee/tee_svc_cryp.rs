// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use crate::mm::vm_load_string;
use crate::tee;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::{alloc::alloc, alloc::dealloc, vec::Vec};
use axerrno::{AxError, AxResult};
use core::{
    alloc::Layout, any::Any, ffi::c_char, ffi::c_uint, ffi::c_ulong, mem::size_of, ptr::NonNull,
    slice, time::Duration,
};
use tee_raw_sys::libc_compat::size_t;
use tee_raw_sys::*;

use super::{
    TeeResult,
    tee_obj::{tee_obj, tee_obj_add},
    user_access::{copy_from_user, copy_to_user},
    libutee::utee_defines::{tee_u32_to_big_endian},
};
#[repr(C)]
struct tee_cryp_obj_type_attrs {
    attr_id: u32,
    flags: u16,
    ops_index: u16,
    raw_offs: u16,
    raw_size: u16,
}

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
#[repr(C)]
struct tee_cryp_obj_secret {
    key_size: u32,
    alloc_size: u32,
    /*
     * Pseudo code visualize layout of structure
     * Next follows data, such as:
     *	uint8_t data[alloc_size]
     * key_size must never exceed alloc_size
     */
}

pub struct tee_cryp_obj_secret_wrapper {
    ptr: NonNull<tee_cryp_obj_secret>,
    layout: Layout,
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
}

impl Drop for tee_cryp_obj_secret_wrapper {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr.as_ptr() as *mut u8, self.layout);
        }
    }
}

/* Handle storing of generic secret keys of varying lengths */
pub const ATTR_OPS_INDEX_SECRET: u32 = 0;
/* Convert to/from big-endian byte array and provider-specific bignum */
pub const ATTR_OPS_INDEX_BIGNUM: u32 = 1;
/* Convert to/from value attribute depending on direction */
/* Convert to/from big-endian byte array and provider-specific bignum */
pub const ATTR_OPS_INDEX_VALUE: u32 = 2;
/* Convert to/from curve25519 attribute depending on direction */
/* Convert to/from big-endian byte array and provider-specific bignum */
pub const ATTR_OPS_INDEX_25519: u32 = 3;
/* Convert to/from big-endian byte array and provider-specific bignum */
pub const ATTR_OPS_INDEX_448: u32 = 4;

pub static TEE_CRYP_OBJ_SECRET_VALUE_ATTRS: [tee_cryp_obj_type_attrs; 1] =
    [tee_cryp_obj_type_attrs {
        attr_id: 1,
        flags: 0,
        ops_index: 1,
        raw_offs: 0,
        raw_size: 0,
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

pub static TEE_CRYP_OBJ_PROPS: [tee_cryp_obj_type_props; 5] = [
    // AES
    prop(
        TEE_TYPE_AES,
        64,
        128,
        256,
        256 / 8 + size_of::<tee_cryp_obj_secret>() as u16,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
    // DES
    prop(
        TEE_TYPE_DES,
        64,
        64,
        64,
        64 / 8 + size_of::<tee_cryp_obj_secret>() as u16,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
    // DES3
    prop(
        TEE_TYPE_DES3,
        64,
        128,
        192,
        192 / 8 + size_of::<tee_cryp_obj_secret>() as u16,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
    // SM4
    prop(
        TEE_TYPE_SM4,
        128,
        128,
        128,
        128 / 8 + size_of::<tee_cryp_obj_secret>() as u16,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
    // HMAC-MD5
    prop(
        TEE_TYPE_HMAC_MD5,
        8,
        64,
        512,
        512 / 8 + size_of::<tee_cryp_obj_secret>() as u16,
        &TEE_CRYP_OBJ_SECRET_VALUE_ATTRS,
    ),
];

pub fn tee_obj_set_type(O: &mut tee_obj, obj_type: u32, max_key_size: size_t) -> AxResult<isize> {
    Ok(0)
}

pub(crate) fn syscall_cryp_obj_alloc(obj_type: c_ulong, max_key_size: c_ulong) -> AxResult<c_uint> {
    let mut obj = tee_obj::default();

    tee_obj_set_type(&mut obj, obj_type as _, max_key_size as _)?;
    tee_obj_add(obj).map(|id| id as c_uint);
    Ok(0)
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
    //data_slice[..size].copy_from_slice(user_buffer);
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
    // 把 &mut u64 转换为 &mut [u8]
    let s_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(&mut s as *mut u64 as *mut u8, core::mem::size_of::<u64>())
    };
    let size_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(
            size_ref as *const u64 as *const u8,
            core::mem::size_of::<u64>(),
        )
    };
    copy_from_user(s_bytes, size_bytes, core::mem::size_of::<u64>())?;

    let key_size = attr.secret().key_size as u64;

    // --- 2. 将 key_size 回写到用户的 size 指针 ---
    let size_ref_bytes: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(
            size_ref as *mut u64 as *mut u8,
            core::mem::size_of::<u64>(),
        )
    };
    let key_size_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(
            &key_size as *const u64 as *const u8,
            core::mem::size_of::<u64>(),
        )
    };

    copy_to_user(size_ref_bytes, key_size_bytes, core::mem::size_of::<u64>())?;

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

fn op_u32_to_binary_helper(v:u32, data: &mut [u8], offs: &mut size_t) -> TeeResult {
	let field: u32;
	let next_offs: size_t;

	next_offs = offs.checked_add(size_of::<u32>()).ok_or(
		TEE_ERROR_OVERFLOW
	)?;

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

fn op_u32_from_binary_helper(
    v: &mut u32,
    data: &[u8],
    offs: &mut size_t,
) -> TeeResult {
    let field: u32;

    if data.len() < *offs + size_of::<u32>() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    let field_bytes = &data[*offs..*offs + size_of::<u32>()];
    field = u32::from_be_bytes(field_bytes.try_into().map_err(|_| TEE_ERROR_BAD_PARAMETERS)?);
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

// static TEE_Result op_attr_secret_value_to_user(void *attr,
// 					       struct ts_session *sess __unused,
// 					       void *buffer, uint64_t *size)
// {
// 	TEE_Result res;
// 	struct tee_cryp_obj_secret *key = attr;
// 	uint64_t s;
// 	uint64_t key_size;

// 	res = copy_from_user(&s, size, sizeof(s));
// 	if (res != TEE_SUCCESS)
// 		return res;

// 	key_size = key->key_size;
// 	res = copy_to_user(size, &key_size, sizeof(key_size));
// 	if (res != TEE_SUCCESS)
// 		return res;

// 	if (s < key->key_size || !buffer)
// 		return TEE_ERROR_SHORT_BUFFER;

// 	return copy_to_user(buffer, key + 1, key->key_size);
// }

// fn op_attr_secret_value_to_user(
//     attr: &tee_cryp_obj_secret_wrapper,
//     buffer: Option<&mut [u8]>, // C: void *buffer
//     size_ref: &mut u64,        // C: uint64_t *size
// ) -> TeeResult {
//     // --- 1. copy_from_user(&s, size, sizeof(s)) ---
//     let mut s: u64 = 0;
//     // 把 &mut u64 转换为 &mut [u8]
//     let s_bytes: &mut [u8] = unsafe {
//         core::slice::from_raw_parts_mut(&mut s as *mut u64 as *mut u8, core::mem::size_of::<u64>())
//     };
//     let size_bytes: &[u8] = unsafe {
//         core::slice::from_raw_parts(
//             size_ref as *const u64 as *const u8,
//             core::mem::size_of::<u64>(),
//         )
//     };
//     copy_from_user(s_bytes, size_bytes, core::mem::size_of::<u64>())?;

//     let key_size = attr.secret().key_size as u64;

//     // --- 2. 将 key_size 回写到用户的 size 指针 ---
//     let size_ref_bytes: &mut [u8] = unsafe {
//         core::slice::from_raw_parts_mut(
//             size_ref as *mut u64 as *mut u8,
//             core::mem::size_of::<u64>(),
//         )
//     };
//     let key_size_bytes: &[u8] = unsafe {
//         core::slice::from_raw_parts(
//             &key_size as *const u64 as *const u8,
//             core::mem::size_of::<u64>(),
//         )
//     };

//     copy_to_user(size_ref_bytes, key_size_bytes, core::mem::size_of::<u64>())?;

//     // --- 3. 检查 buffer 是否足够大 ---
//     let data = attr.data(); // 尾随数组 &[u8]

//     if s < key_size || buffer.is_none() {
//         return Err(TEE_ERROR_SHORT_BUFFER);
//     }

//     let buffer = buffer.unwrap();
//     if buffer.len() < key_size as usize {
//         return Err(TEE_ERROR_SHORT_BUFFER);
//     }

//     // --- 4. 将尾随数据 copy_to_user(buffer, key + 1, key_size) ---
//     copy_to_user(buffer, data, key_size as usize)?;
//     Ok(())
// }


#[cfg(feature = "tee_test")]
pub mod tests_tee_svc_cryp {
    use zerocopy::IntoBytes;

    //-------- test framework import --------
    use crate::tee::TestDescriptor;
    use crate::tee::TestResult;
    use crate::test_fn;
    use crate::{assert, assert_eq, assert_ne, tests, tests_name};

    //-------- local tests import --------
    use super::*;

	test_fn! {
        using TestResult;

        fn test_tee_svc_cryp_utils() {
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
            assert_eq!(read_value, 0x99AABBCC);
            assert_eq!(offs_read, 4);
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
    }
}
