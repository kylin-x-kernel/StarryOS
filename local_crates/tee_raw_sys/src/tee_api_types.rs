// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been modified by KylinSoft on 2025.

use core::{ffi::*, fmt::Debug, fmt};

use crate::libc_compat::size_t;

#[allow(non_camel_case_types)]
pub type TEE_Result = u32;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct TEE_UUID {
    pub timeLow: u32,
    pub timeMid: u16,
    pub timeHiAndVersion: u16,
    pub clockSeqAndNode: [u8; 8],
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TEE_Identity {
    pub login: u32,
    pub uuid: TEE_UUID,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct Memref {
    pub buffer: *mut c_void,
    pub size: usize,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Value {
    pub a: u32,
    pub b: u32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union TEE_Param {
    pub memref: Memref,
    pub value: Value,
}

#[repr(C)]
pub struct __TEE_TASessionHandle {
    _unused: [u8; 0],
}
#[allow(non_camel_case_types)]
pub type TEE_TASessionHandle = *mut __TEE_TASessionHandle;

#[repr(C)]
pub struct __TEE_PropSetHandle {
    _unused: [u8; 0],
}
#[allow(non_camel_case_types)]
pub type TEE_PropSetHandle = *mut __TEE_PropSetHandle;

#[repr(C)]
pub struct __TEE_ObjectHandle {
    _unused: [u8; 0],
}
#[allow(non_camel_case_types)]
pub type TEE_ObjectHandle = *mut __TEE_ObjectHandle;

#[repr(C)]
pub struct __TEE_ObjectEnumHandle {
    _unused: [u8; 0],
}
#[allow(non_camel_case_types)]
pub type TEE_ObjectEnumHandle = *mut __TEE_ObjectEnumHandle;

#[repr(C)]
pub struct __TEE_OperationHandle {
    _unused: [u8; 0],
}
#[allow(non_camel_case_types)]
pub type TEE_OperationHandle = *mut __TEE_OperationHandle;

// Storage Definitions

#[allow(non_camel_case_types)]
pub type TEE_ObjectType = u32;

#[repr(C)]
pub struct TEE_ObjectInfo {
    pub objectId: u32,
    pub objectType: u32,
    pub objectSize: u32,
    pub maxObjectSize: u32,
    pub objectUsage: u32,
    pub dataSize: usize,
    pub dataPosition: usize,
    pub handleFlags: u32,
}

impl Debug for TEE_ObjectInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TEE_ObjectInfo{{objectId: {:#010X?}, objectType: {:#010X?}, objectSize: {:#010X?}, maxObjectSize: \
             {:#010X?}, objectUsage: {:#010X?}, dataSize: {:#010X?}, dataPosition: {:#010X?}, handleFlags: {:#010X?}}}",
            self.objectId,
            self.objectType,
            self.objectSize,
            self.maxObjectSize,
            self.objectUsage,
            self.dataSize,
            self.dataPosition,
            self.handleFlags
        )
    }
}
// Reserve the GP 1.1.1 type
#[repr(C)]
pub enum TEE_Whence {
    TEE_DATA_SEEK_SET,
    TEE_DATA_SEEK_CUR,
    TEE_DATA_SEEK_END,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union content {
    pub memref: Memref,
    pub value: Value,
}

impl Default for content {
    fn default() -> Self {
        content {
            memref: Memref::default(),
        }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TEE_Attribute {
    pub attributeID: u32,
    pub content: content,
}

impl Default for TEE_Attribute {
    fn default() -> Self {
        TEE_Attribute {
            attributeID: 0,
            content: content::default(),
        }
    }
}

// Cryptographic Operations API

// Reserve the GP 1.1.1 type
#[repr(C)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum TEE_OperationMode {
    TEE_MODE_ENCRYPT,
    TEE_MODE_DECRYPT,
    TEE_MODE_SIGN,
    TEE_MODE_VERIFY,
    TEE_MODE_MAC,
    TEE_MODE_DIGEST,
    TEE_MODE_DERIVE,
}

#[repr(C)]
pub struct TEE_OperationInfo {
    pub algorithm: u32,
    pub operationClass: u32,
    pub mode: u32,
    pub digestLength: u32,
    pub maxKeySize: u32,
    pub keySize: u32,
    pub requiredKeyUsage: u32,
    pub handleState: u32,
}

#[repr(C)]
pub struct TEE_OperationInfoKey {
    pub keySize: u32,
    pub requiredKeyUsage: u32,
}

#[repr(C)]
pub struct TEE_OperationInfoMultiple {
    pub algorithm: u32,
    pub operationClass: u32,
    pub mode: u32,
    pub digestLength: u32,
    pub maxKeySize: u32,
    pub handleState: u32,
    pub operationState: u32,
    pub numberOfKeys: u32,
    pub keyInformation: *mut TEE_OperationInfoKey,
}

// Time & Date API

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TeeTime {
    pub seconds: u32,
    pub millis: u32,
}

// TEE Arithmetical APIs

#[allow(non_camel_case_types)]
pub type TEE_BigInt = u32;
#[allow(non_camel_case_types)]
pub type TEE_BigIntFMM = u32;
#[allow(non_camel_case_types)]
pub type TEE_BigIntFMMContext = u32;

// Tee Secure Element APIs

#[repr(C)]
pub struct __TEE_SEServiceHandle {
    _unused: [u8; 0],
}
#[allow(non_camel_case_types)]
pub type TEE_SEServiceHandle = *mut __TEE_SEServiceHandle;
#[repr(C)]
pub struct __TEE_SEReaderHandle {
    _unused: [u8; 0],
}
#[allow(non_camel_case_types)]
pub type TEE_SEReaderHandle = *mut __TEE_SEReaderHandle;
#[repr(C)]
pub struct __TEE_SESessionHandle {
    _unused: [u8; 0],
}
#[allow(non_camel_case_types)]
pub type TEE_SESessionHandle = *mut __TEE_SESessionHandle;
#[repr(C)]
pub struct __TEE_SEChannelHandle {
    _unused: [u8; 0],
}
#[allow(non_camel_case_types)]
pub type TEE_SEChannelHandle = *mut __TEE_SEChannelHandle;

#[repr(C)]
pub struct TEE_SEReaderProperties {
    pub sePresent: bool,
    pub teeOnly: bool,
    pub selectResponseEnable: bool,
}

#[repr(C)]
pub struct TEE_SEAID {
    pub buffer: *mut u8,
    pub bufferLen: size_t,
}

// Other definitions
#[allow(non_camel_case_types)]
pub type TEE_ErrorOrigin = u32;
#[allow(non_camel_case_types)]
pub type TEE_Session = *mut c_void;

pub const TEE_MEM_INPUT: u32 = 0x00000001;
pub const TEE_MEM_OUTPUT: u32 = 0x00000002;
pub const TEE_MEMREF_0_USED: u32 = 0x00000001;
pub const TEE_MEMREF_1_USED: u32 = 0x00000002;
pub const TEE_MEMREF_2_USED: u32 = 0x00000004;
pub const TEE_MEMREF_3_USED: u32 = 0x00000008;
pub const TEE_SE_READER_NAME_MAX: u32 = 20;
