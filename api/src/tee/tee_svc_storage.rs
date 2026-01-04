// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{sync::Arc, vec::Vec, vec};
use core::{
    ffi::{c_uint, c_ulong, c_void},
    mem::{size_of, size_of_val},
};

use bytemuck::{Pod, Zeroable, bytes_of, bytes_of_mut};
use tee_raw_sys::*;

use super::{
    fs_dirfile::{TeeFsDirfileFileh, tee_fs_dirfile_fileh_to_fname},
    tee_misc::{tee_b2hs, tee_b2hs_hsbuf_size},
    tee_obj::{tee_obj, tee_obj_add, tee_obj_close},
    tee_pobj::{tee_pobj_get, tee_pobj_usage, with_pobj_usage_lock},
    tee_ree_fs::tee_svc_storage_file_ops,
    tee_session::with_tee_ta_ctx,
    user_access::{bb_memdup_user_private, copy_to_user_struct},
    uuid::Uuid,
    tee_svc_cryp::{tee_obj_attr_from_binary, tee_obj_set_type},
};
use crate::tee::TeeResult;

pub const TEE_UUID_HEX_LEN: usize = size_of::<TEE_UUID>();

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
struct tee_svc_storage_head {
    pub attr_size: u32,
    pub objectSize: u32,
    pub maxObjectSize: u32,
    pub objectUsage: u32,
    pub objectType: u32,
    pub have_attrs: u32,
}

/// 创建一个基于 TEE_UUID 的目录名。
///
/// 目录名格式为：`/` + UUID 的大写十六进制表示 + `\0` (用于 C 兼容)。
/// C 函数中的 +1 是为了 null 终止符。
/// 因此，所需的缓冲区大小是 TEE_UUID 的十六进制长度 + 1 (null 终止符)。
/// pub const TEE_DIRNAME_BUFFER_REQUIRED_LEN: usize = TEE_UUID_HEX_LEN * 2 + 1;
///
/// # 参数
/// * `buf` - 用于写入目录名的可变字节切片。
/// * `uuid` - 用于生成目录名的 `TEE_UUID`。
///
/// # 返回
/// `Ok(())` - 目录名成功写入 `buf`。
/// `Err(TeeError::ShortBuffer)` - 提供的 `buf` 缓冲区太小。
/// `Err(TeeError::Generic)` - 其他转换错误。
pub fn tee_svc_storage_create_dirname(buf: &mut [u8], uuid: &TEE_UUID) -> TeeResult {
    let required_len = tee_b2hs_hsbuf_size(TEE_UUID_HEX_LEN) + 1; // '/' + UUID_HEX_CHARS + '\0'

    if buf.len() < required_len {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    buf[0] = b'/';

    let uuid_hex_start_idx = 1; // 从 buf 的第二个字节开始写入 UUID

    // convert TEE_UUID to byte slice
    // safety: TEE_UUID is #[repr(C)], memory layout is determined, size is fixed (16 bytes), can be safely converted
    let uuid_bytes = unsafe {
        core::slice::from_raw_parts(uuid as *const TEE_UUID as *const u8, size_of::<TEE_UUID>())
    };

    tee_b2hs(uuid_bytes, &mut buf[uuid_hex_start_idx..]).map_err(|_| TEE_ERROR_GENERIC)?;

    Ok(())
}

const CFG_TEE_FS_PARENT_PATH: &str = "/tmp/";

pub fn tee_svc_storage_create_filename_dfh(
    buf: &mut [u8],
    dfh: Option<&TeeFsDirfileFileh>,
) -> TeeResult<usize> {
    let prefix = CFG_TEE_FS_PARENT_PATH;

    if buf.len() < prefix.len() + 1 {
        return Err(TEE_ERROR_SHORT_BUFFER);
    }

    // 复制前缀
    buf[..prefix.len()].copy_from_slice(prefix.as_bytes());

    // 获取剩余部分用于文件名
    let remaining_buf = &mut buf[prefix.len()..];

    let filename_len = tee_fs_dirfile_fileh_to_fname(dfh, remaining_buf)?;

    Ok(prefix.len() + filename_len)
}

fn remove_corrupt_obj(o: &mut tee_obj) -> TeeResult {
    // remove the corrupt object from the session
    let pobj = Arc::get_mut(&mut o.pobj).ok_or(TEE_ERROR_BAD_STATE)?;

    let fops = pobj.read().fops.ok_or(TEE_ERROR_BAD_STATE)?;
    (fops.remove)(&mut pobj.write());
    // pobj.write().remove(pobj);

    Ok(())
}

fn tee_svc_storage_read_head(o: &mut tee_obj) -> TeeResult {
    let fops = o.pobj.read().fops.ok_or(TEE_ERROR_BAD_STATE)?;

    let mut size: usize = 0;
    let pobj = Arc::get_mut(&mut o.pobj).ok_or_else(|| {
        error!("get pobj failed");
        TEE_ERROR_BAD_STATE
    })?;

    // open the file, store the file handle in tee_obj.fh
    o.fh = (fops.open)(&mut pobj.write(), Some(&mut size)).inspect_err(|e| {
        error!("open failed: {:?}", e);
    })?;

    // read the head
    let mut head = tee_svc_storage_head::zeroed();
    let mut head_slice = bytes_of_mut(&mut head);
    let mut bytes: usize = head_slice.len();
    (fops.read)(&mut o.fh, 0, head_slice, &mut [], &mut bytes).inspect_err(|e| {
        if *e == TEE_ERROR_CORRUPT_OBJECT {
            error!("head corrupt");
        }
    })?;

    // check size overflow
    let mut tmp = (head.attr_size as usize)
        .checked_add(size_of_val(&head))
        .ok_or(TEE_ERROR_OVERFLOW)?;

    if tmp > size {
        return Err(TEE_ERROR_CORRUPT_OBJECT);
    }

    if bytes != size_of_val(&head) {
        return Err(TEE_ERROR_BAD_FORMAT);
    }

    tee_obj_set_type(o, head.objectType as _, head.maxObjectSize as _)?;
    o.ds_pos = tmp;

    // Read attr data if attr_size > 0, otherwise use empty slice
    let attr_data = if head.attr_size > 0 {
        let mut attr = vec![0u8; head.attr_size as usize];
        // read meta
        bytes = head.attr_size as usize;
        (fops.read)(&mut o.fh, size_of_val(&head), &mut attr, &mut [], &mut bytes).inspect_err(|e| {
            if *e == TEE_ERROR_CORRUPT_OBJECT {
                error!("attr corrupt");
            }
        })?;

        if bytes != head.attr_size as usize {
            return Err(TEE_ERROR_CORRUPT_OBJECT);
        }

        attr
    } else {
        vec![]
    };

    tee_obj_attr_from_binary(o, &attr_data)?;

    o.info.dataSize = size - size_of_val(&head) - head.attr_size as usize;

    // o.info.dataSize = size - size_of::<tee_svc_storage_head>() - head.attr_size;
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
    let tee_obj_id: u32 = tee_obj_add(obj_arc.clone())? as u32;

    let obj_open = (|| -> TeeResult {
        with_pobj_usage_lock(po.read().flags, || -> TeeResult {
            // TODO: implement call tee_svc_storage_read_head();
            tee_svc_storage_read_head(Arc::get_mut(&mut obj_arc).ok_or(TEE_ERROR_BAD_STATE)?)?;
            // check if need call tee_obj_close()
            Ok(())
        })?;

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


#[cfg(feature = "tee_test")]
pub mod tests_tee_svc_storage {
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{TestDescriptor, TestResult},
        test_fn, tests, tests_name,
    };

    test_fn! {
        using TestResult;

        fn test_size_of_val() {
            assert_eq!(size_of_val(&tee_svc_storage_head::default()), size_of::<tee_svc_storage_head>());
        }
    }

    tests_name! {
        TEST_TEE_SVC_STORAGE;
        //------------------------
        test_size_of_val,
    }
}

