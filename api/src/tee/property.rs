// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, ffi::CString, vec, vec::Vec};
use core::{
    ffi::{CStr, c_uint, c_ulong, c_void},
    ptr::addr_of,
    slice,
};

use tee_raw_sys::{
    TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_ITEM_NOT_FOUND, TEE_ERROR_NOT_IMPLEMENTED,
    TEE_ERROR_NOT_SUPPORTED, TEE_ERROR_SHORT_BUFFER, TEE_Identity, TEE_PROPSET_CURRENT_CLIENT,
    TEE_PROPSET_CURRENT_TA, TEE_PROPSET_TEE_IMPLEMENTATION, TEE_PropSetHandle,
};

use crate::tee::{
    TeeResult,
    tee_session::{tee_session_ctx, with_tee_session_ctx},
    user_access::{copy_from_user, copy_to_user},
};

/// Trait representing a TA property.
trait TEEProps {
    fn name(&self) -> CString;
    fn prop_type(&self) -> PropType;
    fn get(&self, buf: *mut c_void, blen: &mut u32) -> TeeResult;
}

/// Represents a TEE property set according to the TEE Internal API.
/// The property set is a collection of properties that can be
/// queried from the TEE. The property set is identified by a
/// handle, which is a pointer to a TEE_PropSetHandle structure.
enum PropertySet {
    CurrentClient,
    CurrentTa,
    TeeImplementation,
}

impl PropertySet {
    fn from_raw(raw: c_ulong) -> TeeResult<Self> {
        let handle = raw as usize as TEE_PropSetHandle;
        match handle {
            TEE_PROPSET_CURRENT_CLIENT => Ok(PropertySet::CurrentClient),
            TEE_PROPSET_CURRENT_TA => Ok(PropertySet::CurrentTa),
            TEE_PROPSET_TEE_IMPLEMENTATION => Ok(PropertySet::TeeImplementation),
            _ => Err(TEE_ERROR_BAD_PARAMETERS),
        }
    }
}

enum PropType {
    BOOL,        // bool
    U32,         // uint32_t
    UUID,        // TEE_UUID
    IDENTITY,    // TEE_Identity
    STRING,      // zero terminated string of char
    BINARYBLOCK, // zero terminated base64 coded string
    U64,         // uint64_t
    INVALID,     // invalid value
}

impl PropType {
    fn as_raw(&self) -> c_uint {
        match self {
            PropType::BOOL => 0,
            PropType::U32 => 1,
            PropType::UUID => 2,
            PropType::IDENTITY => 3,
            PropType::STRING => 4,
            PropType::BINARYBLOCK => 5,
            PropType::U64 => 6,
            PropType::INVALID => 7,
        }
    }
}

struct ClientIdentity;

impl TEEProps for ClientIdentity {
    fn name(&self) -> CString {
        CString::new("gpd.client.identity").unwrap()
    }

    fn prop_type(&self) -> PropType {
        PropType::IDENTITY
    }

    fn get(&self, buf: *mut c_void, blen: &mut u32) -> TeeResult {
        let prop_size = size_of::<TEE_Identity>() as u32;
        if *blen < prop_size {
            *blen = prop_size;
            return Err(TEE_ERROR_SHORT_BUFFER);
        }
        *blen = prop_size;
        let clnt_id = with_tee_session_ctx(|ctx| Ok(ctx.clnt_id))?;
        copy_to_user(
            unsafe { slice::from_raw_parts_mut(buf as _, *blen as usize) },
            unsafe { slice::from_raw_parts(addr_of!(clnt_id) as _, size_of::<TEE_Identity>()) },
            *blen as usize,
        )
    }
}

fn get_prop_struct(prop_set: PropertySet, index: c_ulong) -> TeeResult<Box<dyn TEEProps>> {
    match prop_set {
        PropertySet::CurrentClient => match index {
            0 => Ok(Box::new(ClientIdentity)),
            _ => Err(TEE_ERROR_ITEM_NOT_FOUND),
        },
        _ => Err(TEE_ERROR_ITEM_NOT_FOUND),
    }
}

fn get_prop_index(name: &str) -> TeeResult<u32> {
    match name {
        "gpd.client.identity" => Ok(0),
        _ => Err(TEE_ERROR_ITEM_NOT_FOUND),
    }
}

pub(crate) fn sys_tee_scn_get_property(
    prop_set: c_ulong,
    index: c_ulong,
    name: *mut c_void,
    name_len: *mut c_uint,
    buf: *mut c_void,
    blen: *mut c_uint,
    prop_type: *mut c_uint,
) -> TeeResult {
    let prop = get_prop_struct(PropertySet::from_raw(prop_set)?, index)?;

    // Get the property type
    if !prop_type.is_null() {
        let raw_type = prop.prop_type().as_raw();
        copy_to_user(
            unsafe { slice::from_raw_parts_mut(prop_type as _, size_of::<u32>()) },
            &raw_type.to_ne_bytes(),
            size_of::<u32>(),
        )?;
    }

    // Get the property
    if !buf.is_null() && !blen.is_null() {
        let mut klen_buf = [0u8; 4];
        copy_from_user(
            &mut klen_buf,
            unsafe { slice::from_raw_parts(blen as _, size_of::<u32>()) },
            size_of::<u32>(),
        )?;
        let mut klen = u32::from_ne_bytes(klen_buf);

        prop.get(buf, &mut klen)?;
        copy_to_user(
            unsafe { slice::from_raw_parts_mut(blen as _, size_of::<u32>()) },
            &klen.to_ne_bytes(),
            size_of::<u32>(),
        )?;
    }

    // Get the property name
    if !name.is_null() && !name_len.is_null() {
        let prop_name = prop.name();
        let prop_name_bytes = prop_name.to_bytes_with_nul();
        let prop_name_len = prop_name_bytes.len() as u32;

        let mut klen_buf = [0u8; 4];
        copy_from_user(
            &mut klen_buf,
            unsafe { slice::from_raw_parts(name_len as _, size_of::<u32>()) },
            size_of::<u32>(),
        )?;
        let mut klen = u32::from_ne_bytes(klen_buf);

        if klen < prop_name_len {
            klen = prop_name_len;
            copy_to_user(
                unsafe { slice::from_raw_parts_mut(name_len as _, size_of::<u32>()) },
                &klen.to_ne_bytes(),
                size_of::<u32>(),
            )?;
            return Err(TEE_ERROR_SHORT_BUFFER);
        }

        copy_to_user(
            unsafe { slice::from_raw_parts_mut(name as _, klen as usize) },
            prop_name_bytes,
            prop_name_len as usize,
        )?;

        klen = prop_name_len;
        copy_to_user(
            unsafe { slice::from_raw_parts_mut(name_len as _, size_of::<u32>()) },
            &klen.to_ne_bytes(),
            size_of::<u32>(),
        )?;
    }

    Ok(())
}

pub(crate) fn sys_tee_scn_get_property_name_to_index(
    prop_set: c_ulong,
    name: *mut c_void,
    name_len: c_ulong,
    index: *mut c_uint,
) -> TeeResult {
    if name.is_null() || name_len <= 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    let mut kname_buf = vec![0u8; name_len as usize];
    copy_from_user(
        &mut kname_buf,
        unsafe { slice::from_raw_parts(name as *const u8, name_len as usize) },
        name_len as usize,
    )?;
    let kname = match core::str::from_utf8(&kname_buf[..(name_len as usize - 1)]) {
        Ok(kname) => kname,
        Err(_) => return Err(TEE_ERROR_BAD_PARAMETERS),
    };

    let prop_index = get_prop_index(kname)?;
    copy_to_user(
        unsafe { slice::from_raw_parts_mut(index as _, size_of::<u32>()) },
        &prop_index.to_ne_bytes(),
        size_of::<u32>(),
    )?;

    Ok(())
}
