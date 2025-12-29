// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use core::ffi::{c_uint, c_ulong, c_void};
use alloc::sync::Arc;
use tee_raw_sys::*;

use super::{
    tee_ree_fs::tee_svc_storage_file_ops,
    tee_pobj::{tee_pobj_get, tee_pobj_usage, with_pobj_usage_lock},
	tee_obj::{tee_obj, tee_obj_add, tee_obj_close},
    tee_session::with_tee_ta_ctx,
    user_access::{bb_memdup_user_private, copy_to_user_struct},
    uuid::Uuid,
};
use crate::tee::TeeResult;


fn remove_corrupt_obj(o: &mut tee_obj) -> TeeResult {
    //remove the corrupt object from the session
    let pobj = Arc::get_mut(&mut o.pobj).ok_or(TEE_ERROR_BAD_STATE)?;

    let fops = pobj.read().fops.ok_or(TEE_ERROR_BAD_STATE)?;
    (fops.remove)(&mut pobj.write());
    // pobj.write().remove(pobj);

    Ok(())
}

/// Open a storage object
///
/// # Arguments
/// * `storage_id` - The storage ID
/// * `object_id` - The object ID
/// * `object_id_len` - The actual length of the object ID
/// * `flags` - The flags of the object
/// * `obj` - The object handle
///
/// # Returns
/// * The tee_obj_id
///
/// TODO: need add remove_corrupt_obj() while TEE_ERROR_CORRUPT_OBJECT
pub fn syscall_storage_obj_open(
    storage_id: c_ulong,
    object_id: *mut c_void,
    object_id_len: usize,
    flags: c_ulong,
    obj: *mut c_uint,
) -> TeeResult {
    let valid_flags: c_ulong = (TEE_DATA_FLAG_ACCESS_READ
        | TEE_DATA_FLAG_ACCESS_WRITE
        | TEE_DATA_FLAG_ACCESS_WRITE_META
        | TEE_DATA_FLAG_SHARE_READ
        | TEE_DATA_FLAG_SHARE_WRITE) as c_ulong;
    let fops =
        tee_svc_storage_file_ops(storage_id as c_uint).map_err(|_| TEE_ERROR_ITEM_NOT_FOUND)?;

    if flags & !valid_flags != 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    if object_id_len > TEE_OBJECT_ID_MAX_LEN as usize {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    // dump object_id to kernel memory from user space
    let object_id_slice =
        unsafe { core::slice::from_raw_parts(object_id as *const u8, object_id_len as usize) };
    let oid_bbuf = bb_memdup_user_private(object_id_slice)?;

    let uuid = with_tee_ta_ctx(|ctx| Ok(ctx.uuid.clone()))?;
    let uuid = Uuid::parse_str(&uuid)?;
    let po = tee_pobj_get(
        uuid.as_raw_ref(),
        &oid_bbuf,
        object_id_len as u32,
        flags as u32,
        tee_pobj_usage::TEE_POBJ_USAGE_OPEN,
        fops,
    )?;

	let mut o = tee_obj::default();

	// set handleFlags
    o.info.handleFlags = TEE_HANDLE_FLAG_PERSISTENT | TEE_HANDLE_FLAG_INITIALIZED | flags as u32;
    o.pobj = po.clone();
    let mut obj_arc = Arc::new(o);
    let tee_obj_id : u32 = tee_obj_add(obj_arc.clone())? as u32;

    let obj_open = (|| -> TeeResult {
        with_pobj_usage_lock(po.read().flags, || {
            // TODO: implement call tee_svc_storage_read_head();
            // check if need call tee_obj_close()
        });
    
        // copy obj_id to user space
        copy_to_user_struct(unsafe { &mut *obj }, &tee_obj_id)?;

        Ok(())
    })();
    match obj_open {
        Err(err) => {
            if err != TEE_ERROR_CORRUPT_OBJECT {
                tee_obj_close(Arc::get_mut(&mut obj_arc).ok_or(TEE_ERROR_BAD_STATE)?);
            }
        }
        _ => {}
    }
    
    Ok(())
}
