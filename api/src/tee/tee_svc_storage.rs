// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{boxed::Box, string::String, sync::Arc, vec, vec::Vec};
use core::{
    ffi::{c_uint, c_ulong, c_void},
    mem::{offset_of, size_of, size_of_val},
    ptr,
};

use bytemuck::{Pod, Zeroable, bytes_of, bytes_of_mut};
use spin::{Mutex, RwLock};
use tee_raw_sys::*;

use super::{
    fs_dirfile::{TeeFsDirfileFileh, tee_fs_dirfile_fileh_to_fname},
    tee_fs::tee_fs_dirent,
    tee_misc::{tee_b2hs, tee_b2hs_hsbuf_size},
    tee_obj::{tee_obj, tee_obj_add, tee_obj_close, tee_obj_get, tee_obj_id_type},
    tee_pobj::{
        tee_pobj, tee_pobj_create_final, tee_pobj_get, tee_pobj_release, tee_pobj_rename,
        tee_pobj_usage, with_pobj_usage_lock,
    },
    tee_ree_fs::{TeeFileOperations, TeeFsDir, tee_svc_storage_file_ops},
    tee_session::{with_tee_session_ctx_mut, with_tee_ta_ctx},
    tee_svc_cryp::{
        syscall_cryp_obj_close, syscall_cryp_obj_get_info, tee_obj_attr_copy_from,
        tee_obj_attr_from_binary, tee_obj_attr_to_binary, tee_obj_set_type,
    },
    user_access::{
        bb_memdup_user_private, copy_to_user, copy_to_user_private, copy_to_user_struct,
    },
    uuid::Uuid,
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

pub struct tee_storage_enum {
    pub id: c_ulong,
    pub dir: Option<Box<TeeFsDir>>,
    pub fops: Option<&'static TeeFileOperations>,
}

pub fn tee_svc_storage_add_enum(mut obj: tee_storage_enum) -> TeeResult<c_ulong> {
    with_tee_session_ctx_mut(|ctx| {
        // 获取一个可用的 ID
        let vacant = ctx.storage_enums.vacant_entry();
        let id = vacant.key();

        // 设置 objectId
        obj.id = id as c_ulong;

        // 创建 Arc 并插入
        let arc_obj = Arc::new(Mutex::new(obj));
        let inserted_id = vacant.insert(arc_obj);
        tee_debug!("tee_svc_storage_add_enum: id: {}", id);

        Ok(id as c_ulong)
    })
}

fn tee_svc_storage_delete_enum(enum_id: c_ulong) -> TeeResult<Arc<Mutex<tee_storage_enum>>> {
    // remove from session objects
    with_tee_session_ctx_mut(|ctx| -> TeeResult<Arc<Mutex<tee_storage_enum>>> {
        let obj = ctx
            .storage_enums
            .try_remove(enum_id as _)
            .ok_or(TEE_ERROR_ITEM_NOT_FOUND)?;
        Ok(obj)
    })
}

fn tee_svc_storage_get_enum(enum_id: c_ulong) -> TeeResult<Arc<Mutex<tee_storage_enum>>> {
    with_tee_session_ctx_mut(|ctx| {
        let e = ctx
            .storage_enums
            .get(enum_id as usize)
            .ok_or(TEE_ERROR_BAD_PARAMETERS)?;
        Ok(e.clone())
    })
}

fn tee_svc_close_enum(enum_id: c_ulong) -> TeeResult {
    let obj = tee_svc_storage_delete_enum(enum_id)?;

    // get the lock of the dir, and get the &mut TeeFsDir borrow
    let fops = {
        let obj_guard = obj.lock();
        obj_guard.fops
    };

    if let Some(fops) = fops {
        let mut obj_guard = obj.lock();
        // let mut d = obj_guard.dir.as_mut().ok_or(TEE_ERROR_BAD_STATE)?.lock();
        (fops.closedir)(obj_guard.dir.as_mut().ok_or(TEE_ERROR_BAD_STATE)?)?;
    }

    // obj auto released when the scope ends
    Ok(())
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
    let pobj = o.pobj.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;

    let fops = pobj.read().fops.ok_or(TEE_ERROR_BAD_STATE)?;
    (fops.remove)(&mut pobj.write());
    // pobj.write().remove(pobj);

    Ok(())
}

fn tee_svc_storage_read_head(o: &mut tee_obj) -> TeeResult {
    tee_debug!("tee_svc_storage_read_head: o: {:?}", o);

    // 先获取 fops，然后立即释放读锁，避免与后续的写锁冲突
    let fops = {
        let guard = o.pobj.as_ref().ok_or(TEE_ERROR_BAD_STATE)?.read();
        guard.fops.ok_or(TEE_ERROR_BAD_STATE)?
    }; // guard 在这里被释放，读锁被释放

    tee_debug!("tee_svc_storage_read_head: fops: {:?}", fops);
    let mut size: usize = 0;
    {
        tee_debug!("try to get write lock");
        // 现在可以安全地获取写锁，因为读锁已经释放
        let mut pobj_guard = o.pobj.as_mut().ok_or(TEE_ERROR_BAD_STATE)?.write();
        tee_debug!("get write lock");
        // open the file, store the file handle in tee_obj.fh
        o.fh = Some(
            (fops.open)(&mut *pobj_guard, Some(&mut size)).inspect_err(|e| {
                error!("open failed: {:X?}", e);
            })?,
        );
    }
    tee_debug!("tee_svc_storage_read_head: size: {}", size);
    // read the head
    let mut head = tee_svc_storage_head::zeroed();
    let mut head_slice = bytes_of_mut(&mut head);
    let mut bytes: usize = head_slice.len();
    let fh = o.fh.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;
    (fops.read)(fh, 0, head_slice, &mut [], &mut bytes).inspect_err(|e| {
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

    tee_debug!(
        "bytes: {}, size_of_val(&head): {}",
        bytes,
        size_of_val(&head)
    );
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
        (fops.read)(
            o.fh.as_mut().ok_or(TEE_ERROR_BAD_STATE)?,
            size_of_val(&head),
            &mut attr,
            &mut [],
            &mut bytes,
        )
        .inspect_err(|e| {
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
    o.info.objectSize = head.objectSize as u32;
    // 需要再次获取写锁来修改 obj_info_usage
    o.pobj
        .as_ref()
        .ok_or(TEE_ERROR_BAD_STATE)?
        .write()
        .obj_info_usage = head.objectUsage as u32;
    o.info.objectType = head.objectType as u32;
    o.have_attrs = head.have_attrs as u32;

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
    tee_debug!(
        "syscall_storage_obj_open: storage_id: {:X?}, object_id: {:?}, object_id_len: {:X?}, \
         flags: {:X?}, obj: {:X?}",
        storage_id,
        object_id,
        object_id_len,
        flags,
        obj
    );
    let valid_flags: c_ulong = (TEE_DATA_FLAG_ACCESS_READ
        | TEE_DATA_FLAG_ACCESS_WRITE
        | TEE_DATA_FLAG_ACCESS_WRITE_META
        | TEE_DATA_FLAG_SHARE_READ
        | TEE_DATA_FLAG_SHARE_WRITE) as c_ulong;
    let fops =
        tee_svc_storage_file_ops(storage_id as c_uint).map_err(|_| TEE_ERROR_ITEM_NOT_FOUND)?;

    tee_debug!(
        "syscall_storage_obj_open: flags: {:#010X?}, valid_flags: {:#010X?}",
        flags,
        valid_flags
    );
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
    // let uuid = Uuid::new_raw(0, 0, 0, [0; 8]);

    tee_debug!("syscall_storage_obj_open: step 1 : tee_pobj_get");
    let po = tee_pobj_get(
        uuid.as_raw_ref(),
        &oid_bbuf,
        object_id_len as u32,
        flags as u32,
        tee_pobj_usage::TEE_POBJ_USAGE_OPEN,
        fops,
    )?;

    let mut o = tee_obj::default();

    tee_debug!("syscall_storage_obj_open: step 2 : tee_obj_add");
    // set handleFlags
    o.info.handleFlags = TEE_HANDLE_FLAG_PERSISTENT | TEE_HANDLE_FLAG_INITIALIZED | flags as u32;
    o.pobj = Some(po.clone());
    let tee_obj_id: u32 = tee_obj_add(o)? as u32;

    let mut o_arc = tee_obj_get(tee_obj_id as tee_obj_id_type)?;
    tee_debug!("o_arc: {:?}", o_arc);
    // 提前读取 flags，确保 po.read() 的 guard 已经释放
    let pobj_flags = {
        let guard = po.read();
        guard.flags
    }; // guard 在这里被释放
    let obj_open = (|| -> TeeResult {
        tee_debug!("syscall_storage_obj_open: step 3 : tee_svc_storage_read_head");
        with_pobj_usage_lock(pobj_flags, || -> TeeResult {
            // TODO: implement call tee_svc_storage_read_head();
            tee_svc_storage_read_head(&mut o_arc.lock())
            // check if need call tee_obj_close()
            // Ok(())
        })?;

        // copy obj_id to user space
        copy_to_user_struct(unsafe { &mut *obj }, &tee_obj_id)?;

        Ok(())
    })();
    match obj_open {
        Err(err) => {
            if err != TEE_ERROR_CORRUPT_OBJECT {
                let _ = tee_obj_close(tee_obj_id).inspect_err(|e| {
                    error!("tee_obj_close failed: {:X?}", e);
                });
                return Err(err);
            }
        }
        _ => {}
    }

    Ok(())
}

fn tee_svc_storage_init_file(
    o: &mut tee_obj,
    overwrite: bool,
    attr_o: Option<&mut tee_obj>,
    src_is_dst: bool,
    data: &[u8],
) -> TeeResult {
    let fops = {
        let guard = o.pobj.as_ref().ok_or(TEE_ERROR_BAD_STATE)?.read();
        guard.fops.ok_or(TEE_ERROR_BAD_STATE)?
    };

    let mut attr_size = 0;
    let mut attr: Box<[u8]> = Vec::<u8>::new().into_boxed_slice();
    if let Some(attr_o) = attr_o {
        if !src_is_dst {
            tee_obj_set_type(o, attr_o.info.objectType, attr_o.info.maxObjectSize as _)?;

            tee_obj_attr_copy_from(o, attr_o)?;
            o.have_attrs = attr_o.have_attrs;
            o.pobj
                .as_ref()
                .ok_or(TEE_ERROR_BAD_STATE)?
                .write()
                .obj_info_usage = attr_o.info.objectUsage;
            o.info.objectSize = attr_o.info.objectSize;
        }
        tee_obj_attr_to_binary(o, &mut [], &mut attr_size)?;
        if attr_size > 0 {
            attr = vec![0u8; attr_size].into_boxed_slice();
            tee_obj_attr_to_binary(o, &mut attr, &mut attr_size)?;
        }
    } else {
        tee_obj_set_type(o, TEE_TYPE_DATA, 0)?;
    }

    o.ds_pos = size_of::<tee_svc_storage_head>() + attr_size;

    // write head
    let mut head = tee_svc_storage_head::default();
    head.attr_size = attr_size as u32;
    head.objectSize = o.info.objectSize;
    head.maxObjectSize = o.info.maxObjectSize;
    head.objectType = o.info.objectType;
    head.have_attrs = o.have_attrs;
    let mut pobj_guard = o.pobj.as_mut().ok_or(TEE_ERROR_BAD_STATE)?.write();
    head.objectUsage = pobj_guard.obj_info_usage;
    o.fh = Some(
        (fops.create)(
            &mut pobj_guard,
            overwrite,
            bytemuck::bytes_of(&head),
            &attr,
            &[],
            data,
            data.len(),
        )
        .inspect_err(|e| {
            o.ds_pos = 0;
            error!("create failed: {:X?}", e);
        })?,
    );
    o.info.dataSize = data.len();

    Ok(())
}

/// inner result for syscall_storage_obj_create_inner
enum CreateInnerResult {
    /// 成功：转换已有对象为持久化对象（第一分支）
    ConvertedExisting,
    /// 成功：创建了新的持久化对象，返回 object id
    CreatedNew(u32),
    /// 失败：在 tee_obj_add 之前失败，需要清理 o.fh 和 po
    ErrBeforeAdd {
        error: u32,
        po: Option<Arc<RwLock<tee_pobj>>>,
        o: Option<tee_obj>,
    },
    /// 失败：在 tee_obj_add 之后失败（oclose 路径），需要调用 tee_obj_close
    ErrAfterAdd { error: u32, o_id: u32 },
}

/// inner context for syscall_storage_obj_create_inner
struct CreateInnerCtx<'a> {
    fops: &'a TeeFileOperations,
    po: Option<Arc<RwLock<tee_pobj>>>,
}

/// inner function: execute the core logic, return the result or the resources to clean up
///
/// # Arguments
/// * `ctx` - the inner context
/// * `flags` - the flags
/// * `attr` - the attribute
/// * `data` - the data
/// * `obj_is_null` - whether the object is null
/// # Returns
/// * `CreateInnerResult` - the result of the operation
fn syscall_storage_obj_create_inner(
    ctx: &mut CreateInnerCtx,
    flags: c_ulong,
    attr: c_ulong,
    data: &[u8],
    obj_is_null: bool,
) -> CreateInnerResult {
    // === 获取 attr_o ===
    let attr_o = if attr != TEE_HANDLE_NULL as _ {
        match tee_obj_get(attr as _) {
            Ok(o) => {
                let guard = o.lock();
                if guard.info.handleFlags & TEE_HANDLE_FLAG_INITIALIZED == 0 {
                    return CreateInnerResult::ErrBeforeAdd {
                        error: TEE_ERROR_BAD_PARAMETERS,
                        po: ctx.po.take(),
                        o: None,
                    };
                }
                drop(guard);
                Some(o)
            }
            Err(e) => {
                return CreateInnerResult::ErrBeforeAdd {
                    error: e,
                    po: ctx.po.take(),
                    o: None,
                };
            }
        }
    } else {
        None
    };

    // === C: if (!obj && attr_o && !PERSISTENT) - 转换已有对象 ===
    if obj_is_null
        && attr_o.is_some()
        && (attr_o.as_ref().unwrap().lock().info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT == 0)
    {
        // convert temporary object to persistent object
        // 1. obj == null means caller does not need to return a new handle(cause handle exists)
        // 2. attr_o != null means attributes object is provided(attr != TEE_HANDLE_NULL)
        // 3. TEE_HANDLE_FLAG_PERSISTENT == 0 means attributes object is not a persistent object(is a temporary object)

        let attr_o = attr_o.unwrap();
        let mut a = attr_o.lock();

        let saved_flags = a.info.handleFlags;
        a.info.handleFlags =
            TEE_HANDLE_FLAG_PERSISTENT | TEE_HANDLE_FLAG_INITIALIZED | flags as u32;

        // 转移 po 所有权给 attr_o
        let po_for_attr = ctx.po.take().unwrap();

        po_for_attr.write().obj_info_usage = a.info.objectUsage;
        a.pobj = Some(po_for_attr);

        if let Err(e) = tee_svc_storage_init_file(
            &mut a,
            (flags & TEE_DATA_FLAG_OVERWRITE as u64) != 0,
            Some(&mut tee_obj::default()),
            true,
            data,
        ) {
            // 恢复状态
            let po_back = a.pobj.take();
            a.info.handleFlags = saved_flags;
            return CreateInnerResult::ErrBeforeAdd {
                error: e,
                po: po_back,
                o: None,
            };
        }

        a.info.objectUsage = 0;
        return CreateInnerResult::ConvertedExisting;
    }

    // === 创建新 persistent object ===
    let mut o = tee_obj::default();
    o.info.handleFlags = TEE_HANDLE_FLAG_PERSISTENT | TEE_HANDLE_FLAG_INITIALIZED | flags as u32;

    // 转移 po 所有权给 o
    let po_for_o = ctx.po.take().unwrap();
    o.pobj = Some(po_for_o.clone());

    let init_result = if let Some(attr_o) = attr_o {
        let mut a = attr_o.lock();
        tee_svc_storage_init_file(
            &mut o,
            (flags & TEE_DATA_FLAG_OVERWRITE as u64) != 0,
            Some(&mut a),
            false,
            data,
        )
    } else {
        tee_svc_storage_init_file(
            &mut o,
            (flags & TEE_DATA_FLAG_OVERWRITE as u64) != 0,
            None,
            false,
            data,
        )
    };

    if let Err(e) = init_result {
        // 失败时，po 所有权在 o.pobj 中，需要取出来返回
        let po_back = o.pobj.take();
        return CreateInnerResult::ErrBeforeAdd {
            error: e,
            po: po_back,
            o: Some(o),
        };
    }

    o.info.objectUsage = 0;

    let o_id = match tee_obj_add(o) {
        Ok(id) => id as u32,
        Err(e) => {
            // tee_obj_add 失败比较特殊，o 的所有权已经被 move 进去了
            // 这种情况下 o 不会被添加到表中，但我们也无法拿回来
            // 实际上 tee_obj_add 不太可能失败（只是 slab insert）
            return CreateInnerResult::ErrBeforeAdd {
                error: e,
                po: Some(po_for_o),
                o: None, // o 已经被 move 了
            };
        }
    };

    // 成功添加到全局表，返回 o_id
    CreateInnerResult::CreatedNew(o_id)
}

/// create a new persistent object
///
/// # Arguments
/// * `storage_id` - the storage id
/// * `object_id` - the object id
/// * `object_id_len` - the length of the object id
/// * `flags` - the flags
/// * `attr` - the attribute
/// * `data` - the data
/// * `len` - the length of the data
/// * `obj` - the object
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_obj_create(
    storage_id: c_ulong,
    object_id: *mut c_void,
    object_id_len: usize,
    flags: c_ulong,
    attr: c_ulong,
    data: *mut c_void,
    len: usize,
    obj: *mut c_uint,
) -> TeeResult {
    tee_debug!(
        "syscall_storage_obj_create: storage_id: {:X?}, object_id: {:?}, object_id_len: {:X?}, \
         flags: {:X?}, attr: {:X?}, data: {:?}, len: {:X?}, obj: {:X?}",
        storage_id,
        object_id,
        object_id_len,
        flags,
        attr,
        data,
        len,
        obj
    );
    const VALID_FLAGS: c_ulong = (TEE_DATA_FLAG_ACCESS_READ
        | TEE_DATA_FLAG_ACCESS_WRITE
        | TEE_DATA_FLAG_ACCESS_WRITE_META
        | TEE_DATA_FLAG_SHARE_READ
        | TEE_DATA_FLAG_SHARE_WRITE
        | TEE_DATA_FLAG_OVERWRITE) as _;

    // === 参数校验（这些错误不需要资源清理）===
    if flags & !VALID_FLAGS != 0 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    let fops = tee_svc_storage_file_ops(storage_id as _).map_err(|_| TEE_ERROR_ITEM_NOT_FOUND)?;

    if object_id_len > TEE_OBJECT_ID_MAX_LEN as usize {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    // Check presence of optional buffer
    if len != 0 && data.is_null() {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    tee_debug!(
        "syscall_storage_obj_create object_id_len: {:X?}",
        object_id_len
    );
    let object_id_slice =
        unsafe { core::slice::from_raw_parts(object_id as *const u8, object_id_len) };
    let oid_bbuf = bb_memdup_user_private(object_id_slice)?;

    let uuid = with_tee_ta_ctx(|ctx| Ok(ctx.uuid.clone()))?;
    let uuid = Uuid::parse_str(&uuid)?;

    // === tee_pobj_get - need resource cleanup from here ===
    let po = tee_pobj_get(
        uuid.as_raw_ref(),
        &oid_bbuf,
        object_id_len as u32,
        flags as u32,
        tee_pobj_usage::TEE_POBJ_USAGE_CREATE,
        fops,
    )?;
    tee_debug!("syscall_storage_obj_create: tee_pobj_get po: {:?}", po);

    let data_slice = unsafe { core::slice::from_raw_parts(data as *const u8, len) };

    // === call inner function ===
    let mut inner_ctx = CreateInnerCtx {
        fops,
        po: Some(po.clone()),
    };

    let result =
        syscall_storage_obj_create_inner(&mut inner_ctx, flags, attr, data_slice, obj.is_null());

    // === 根据结果处理 ===
    match result {
        CreateInnerResult::ConvertedExisting => {
            // 第一分支成功，po 所有权已转移给 attr_o
            Ok(())
        }

        CreateInnerResult::CreatedNew(o_id) => {
            // 第二分支成功，继续处理
            if !obj.is_null() {
                if let Err(e) = unsafe { copy_to_user_struct(&mut *obj, &o_id) } {
                    // oclose 路径：C 逻辑中 oclose 不进行错误码转换
                    let _ = tee_obj_close(o_id);
                    return Err(e);
                }
            }

            tee_pobj_create_final(&mut po.write());

            if obj.is_null() {
                tee_obj_close(o_id)?;
            }

            Ok(())
        }

        CreateInnerResult::ErrBeforeAdd { error, po, o } => {
            // err: 路径
            let error = convert_error(error);

            if let Some(mut o) = o {
                (fops.close)(&mut o.fh);
                // o 会在这里 drop
            }

            if error == TEE_ERROR_CORRUPT_OBJECT {
                if let Some(ref po_ref) = po {
                    tee_debug!("CreateInnerResult::ErrBeforeAdd: fops.remove");
                    (fops.remove)(&mut po_ref.write());
                }
            }

            if let Some(po) = po {
                let _ = tee_pobj_release(po);
            }

            Err(error)
        }

        CreateInnerResult::ErrAfterAdd { error, o_id } => {
            // oclose: 路径
            let _ = tee_obj_close(o_id);
            Err(convert_error(error))
        }
    }
}

fn convert_error(mut e: u32) -> u32 {
    if e == TEE_ERROR_NO_DATA || e == TEE_ERROR_BAD_FORMAT {
        e = TEE_ERROR_CORRUPT_OBJECT;
    }
    e
}

/// delete a persistent object
///
/// # Arguments
/// * `obj_id` - the object id
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_obj_del(obj_id: c_ulong) -> TeeResult {
    tee_debug!("syscall_storage_obj_del: obj_id: {:X?}", obj_id);
    let o = tee_obj_get(obj_id)?;

    // check permission and get necessary information, then release o_guard immediately
    let (pobj_arc, fops) = {
        let o_guard = o.lock();

        if (o_guard.info.handleFlags & TEE_DATA_FLAG_ACCESS_WRITE_META == 0) {
            return Err(TEE_ERROR_ACCESS_CONFLICT);
        }

        // check if pobj exists and obj_id is not empty
        let pobj_arc = o_guard.pobj.as_ref().ok_or(TEE_ERROR_BAD_STATE)?.clone();
        {
            let pobj_guard = pobj_arc.read();
            if pobj_guard.obj_id.is_empty() {
                return Err(TEE_ERROR_BAD_STATE);
            }
        }

        let fops = {
            let pobj_guard = pobj_arc.read();
            pobj_guard.fops.ok_or(TEE_ERROR_BAD_STATE)?
        };

        // clone pobj_arc and get fops before releasing o_guard
        (pobj_arc, fops)
    };

    // now it is safe to get the write lock of pobj, because o_guard is released
    let mut pobj_guard = pobj_arc.write();
    let res = (fops.remove)(&mut pobj_guard);

    // release pobj_guard
    drop(pobj_guard);

    // now it is safe to call tee_obj_close, because all locks are released
    let _ = tee_obj_close(obj_id as u32);

    res
}

/// rename a persistent object
///
/// # Arguments
/// * `obj` - the object id
/// * `object_id` - the new object id
/// * `object_id_len` - the length of the new object id
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_obj_rename(
    obj: c_ulong,
    object_id: *mut c_void,
    object_id_len: usize,
) -> TeeResult {
    let object_id_slice =
        unsafe { core::slice::from_raw_parts(object_id as *const u8, object_id_len) };

    let oid_bbuf = bb_memdup_user_private(object_id_slice)?;

    tee_debug!(
        "syscall_storage_obj_rename: obj: {:X?}, object_id: {:?}, object_id_len: {:04X?}",
        obj,
        String::from_utf8_lossy(object_id_slice),
        object_id_len
    );

    if object_id_len > TEE_OBJECT_ID_MAX_LEN as usize {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }
    let (o, fops) = {
        let mut o = tee_obj_get(obj)?;
        let o_guard = o.lock();
        if o_guard.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT == 0 {
            return Err(TEE_ERROR_BAD_STATE);
        }
        if o_guard.info.handleFlags & TEE_DATA_FLAG_ACCESS_WRITE_META == 0 {
            return Err(TEE_ERROR_ACCESS_CONFLICT);
        }

        if o_guard.pobj.is_none() || o_guard.pobj.as_ref().unwrap().read().obj_id.is_empty() {
            return Err(TEE_ERROR_BAD_STATE);
        }

        // reserve dest name
        let fops = o_guard
            .pobj
            .as_ref()
            .ok_or(TEE_ERROR_ITEM_NOT_FOUND)?
            .read()
            .fops
            .ok_or(TEE_ERROR_BAD_STATE)?;
        drop(o_guard);
        (o, fops)
    };

    let uuid = with_tee_ta_ctx(|ctx| Ok(ctx.uuid.clone()))?;
    let uuid = Uuid::parse_str(&uuid)?;

    let po = tee_pobj_get(
        uuid.as_raw_ref(),
        &oid_bbuf,
        object_id_len as u32,
        TEE_DATA_FLAG_ACCESS_WRITE_META,
        tee_pobj_usage::TEE_POBJ_USAGE_RENAME,
        fops,
    )
    .inspect_err(|e| {
        error!("syscall_storage_obj_rename: tee_pobj_get error: {:#X?}", e);
    })?;

    // move
    let res = {
        let mut o_guard = o.lock();
        let mut pobj = o_guard.pobj.as_ref().unwrap().write();
        (fops.rename)(&mut pobj, &po.read(), false /* no overwrite */).inspect_err(|e| {
            error!("syscall_storage_obj_rename: fops.rename error: {:#X?}", e);
        })?;

        let po_guard = po.read();
        let obj_id = po_guard.obj_id.as_ref();
        let obj_id_len = po_guard.obj_id_len;
        tee_pobj_rename(&mut pobj, obj_id, obj_id_len)
    };

    // Always release the new po, regardless of success or failure
    // This matches the C implementation which calls tee_pobj_release(po) in the exit label
    let _ = tee_pobj_release(po);

    res
}

/// read data from the object
///
/// # Arguments
/// * `obj` - the object id
/// * `data` - the data to read
/// * `len` - the length of the data
/// * `count` - the count of the data
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_obj_read(
    obj: c_ulong,
    data: *mut c_void,
    len: usize,
    count: *mut u64,
) -> TeeResult {
    tee_debug!(
        "syscall_storage_obj_read: obj: {:X?}, data_len: {:X?}, count: {:X?}",
        obj,
        len,
        count
    );
    let o = tee_obj_get(obj)?;

    let (fops, pos_tmp) = {
        let o_guard = o.lock();

        if (o_guard.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT == 0) {
            return Err(TEE_ERROR_BAD_STATE);
        }

        if (o_guard.info.handleFlags & TEE_DATA_FLAG_ACCESS_READ == 0) {
            return Err(TEE_ERROR_ACCESS_CONFLICT);
        }

        let pos_tmp = o_guard
            .info
            .dataPosition
            .checked_add(len)
            .ok_or(TEE_ERROR_OVERFLOW)?;

        // data = memtag_strip_tag(data);
        tee_debug!("syscall_storage_obj_read: ds_pos: {:X?}", o_guard.ds_pos);

        let pos_tmp = o_guard
            .ds_pos
            .checked_add(o_guard.info.dataPosition)
            .ok_or(TEE_ERROR_OVERFLOW)?;

        (
            o_guard
                .pobj
                .as_ref()
                .ok_or(TEE_ERROR_BAD_STATE)?
                .read()
                .fops
                .ok_or(TEE_ERROR_BAD_STATE)?,
            pos_tmp,
        )
    };

    let mut bytes = len;
    let mut o_guard = o.lock();
    let data_slice = unsafe { core::slice::from_raw_parts_mut(data as *mut u8, len) };
    tee_debug!(
        "syscall_storage_obj_read: bytes: {:X?} dataPosition: 0x{:X?}",
        bytes,
        o_guard.info.dataPosition
    );
    let fh = o_guard.fh.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;
    (fops.read)(fh, pos_tmp, &mut [], data_slice, &mut bytes).inspect_err(|e| {
        if *e == TEE_ERROR_CORRUPT_OBJECT {
            error!("Object corrupt");
            remove_corrupt_obj(&mut o_guard);
        }
    })?;
    o_guard.info.dataPosition += bytes;

    let u_count = bytes as u64;
    copy_to_user_struct(unsafe { &mut *count }, &u_count);

    Ok(())
}

/// write data to the object
///
/// # Arguments
/// * `obj` - the object id
/// * `data` - the data to write
/// * `len` - the length of the data
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_obj_write(obj: c_ulong, data: *mut c_void, len: usize) -> TeeResult {
    tee_debug!(
        "syscall_storage_obj_write: obj: {:X?}, data_len: {:X?}",
        obj,
        len
    );
    let o = tee_obj_get(obj)?;
    let (fops, pos_tmp) = {
        let o_guard = o.lock();
        if (o_guard.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT == 0) {
            return Err(TEE_ERROR_BAD_STATE);
        }
        if (o_guard.info.handleFlags & TEE_DATA_FLAG_ACCESS_WRITE == 0) {
            return Err(TEE_ERROR_ACCESS_CONFLICT);
        }

        let pos_tmp = o_guard
            .info
            .dataPosition
            .checked_add(len)
            .ok_or(TEE_ERROR_OVERFLOW)?;

        let pos_tmp = o_guard
            .ds_pos
            .checked_add(o_guard.info.dataPosition)
            .ok_or(TEE_ERROR_OVERFLOW)?;

        (
            o_guard
                .pobj
                .as_ref()
                .ok_or(TEE_ERROR_BAD_STATE)?
                .read()
                .fops
                .ok_or(TEE_ERROR_BAD_STATE)?,
            pos_tmp,
        )
    };

    let mut o_guard = o.lock();

    tee_debug!(
        "syscall_storage_obj_write: dataPosition: {:X?}",
        o_guard.info.dataPosition
    );
    let data_slice = unsafe { core::slice::from_raw_parts(data as *const u8, len) };
    let fh = o_guard.fh.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;
    (fops.write)(fh, pos_tmp, &[], data_slice, len).inspect_err(|e| {
        error!("syscall_storage_obj_write: write failed: {:X?}", e);
    })?;
    o_guard.info.dataPosition += len;
    if o_guard.info.dataPosition > o_guard.info.dataSize {
        o_guard.info.dataSize = o_guard.info.dataPosition;
    }
    Ok(())
}

pub fn tee_svc_storage_write_usage(o: &mut tee_obj, usage: u32) -> TeeResult {
    let pos = offset_of!(tee_svc_storage_head, objectUsage);

    let fops = {
        let guard = o.pobj.as_ref().ok_or(TEE_ERROR_BAD_STATE)?.read();
        guard.fops.ok_or(TEE_ERROR_BAD_STATE)?
    };

    let usage_slice = unsafe {
        core::slice::from_raw_parts(
            &usage as *const u32 as *const u8,
            core::mem::size_of::<u32>(),
        )
    };

    let fh = o.fh.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;
    (fops.write)(fh, pos, usage_slice, &[], usage_slice.len()).inspect_err(|e| {
        error!("tee_svc_storage_write_usage: write failed: {:X?}", e);
    })
}

/// truncate the object to the length
///
/// # Arguments
/// * `obj` - the object id
/// * `len` - the length to truncate
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_obj_trunc(obj: c_ulong, len: usize) -> TeeResult {
    let o = tee_obj_get(obj)?;

    // check flags and get fops and attr size
    let (fops, attr_size) = {
        let mut o_guard = o.lock();
        if o_guard.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT == 0 {
            return Err(TEE_ERROR_BAD_STATE);
        }
        if o_guard.info.handleFlags & TEE_DATA_FLAG_ACCESS_WRITE == 0 {
            return Err(TEE_ERROR_ACCESS_CONFLICT);
        }

        let mut attr_size: usize = 0;
        tee_obj_attr_to_binary(&mut o_guard, &mut [], &mut attr_size)?;
        (
            o_guard
                .pobj
                .as_ref()
                .ok_or(TEE_ERROR_BAD_STATE)?
                .read()
                .fops
                .ok_or(TEE_ERROR_BAD_STATE)?,
            attr_size,
        )
    };

    // calculate offset
    let mut offs = size_of::<tee_svc_storage_head>()
        .checked_add(attr_size)
        .ok_or(TEE_ERROR_OVERFLOW)?;
    offs = offs.checked_add(len).ok_or(TEE_ERROR_OVERFLOW)?;

    // call truncate
    let res = {
        let mut o_guard = o.lock();
        let fh = o_guard.fh.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;
        (fops.truncate)(fh, offs)
    };

    match res {
        Ok(()) => {
            let mut o_guard = o.lock();
            tee_debug!(
                "truncate success, dataSize from {:X?} to {:X?}",
                o_guard.info.dataSize,
                len
            );
            o_guard.info.dataSize = len;
            Ok(())
        }
        Err(e) if e == TEE_ERROR_CORRUPT_OBJECT => {
            error!("Object corruption");
            let _ = remove_corrupt_obj(&mut o.lock()); // Not using the return value
            Err(TEE_ERROR_CORRUPT_OBJECT)
        }
        Err(_) => Err(TEE_ERROR_GENERIC),
    }
}

/// seek to the offset in the object
///
/// # Arguments
/// * `obj` - the object id
/// * `offset` - the offset to seek
/// * `whence` - the whence to seek
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_obj_seek(obj: c_ulong, offset: i32, whence: c_ulong) -> TeeResult {
    let o = tee_obj_get(obj)?;
    let o_guard = o.lock();
    if o_guard.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT == 0 {
        return Err(TEE_ERROR_BAD_STATE);
    }
    let mut new_pos: i64 = match whence as u32 {
        TEE_DATA_SEEK_SET => offset as i64,
        TEE_DATA_SEEK_CUR => (o_guard.info.dataPosition as i64)
            .checked_add(offset as i64)
            .ok_or(TEE_ERROR_OVERFLOW)?,
        TEE_DATA_SEEK_END => (o_guard.info.dataSize as i64)
            .checked_add(offset as i64)
            .ok_or(TEE_ERROR_OVERFLOW)?,
        _ => return Err(TEE_ERROR_BAD_PARAMETERS),
    };
    drop(o_guard);

    tee_debug!("syscall_storage_obj_seek: new_pos: 0x{:02X}", new_pos);
    if new_pos < 0 {
        new_pos = 0;
    }

    if new_pos > TEE_DATA_MAX_POSITION as i64 {
        return Err(TEE_ERROR_OVERFLOW);
    }

    let mut o_guard = o.lock();
    o_guard.info.dataPosition = new_pos as usize;
    tee_debug!(
        "syscall_storage_obj_seek: new_pos: 0x{:02X}",
        o_guard.info.dataPosition
    );

    Ok(())
}

/// allocate the enumeration of the object
///
/// # Arguments
/// * `obj_enum` - the object enumeration id
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_alloc_enum(obj_enum: *mut c_uint) -> TeeResult {
    let obj = tee_storage_enum {
        id: 0,
        dir: None,
        fops: None,
    };
    let id = tee_svc_storage_add_enum(obj)? as u32;
    copy_to_user_struct(unsafe { &mut *obj_enum }, &id)?;
    Ok(())
}

/// free the enumeration of the object
///
/// # Arguments
/// * `obj_enum` - the object enumeration id
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_free_enum(obj_enum: c_ulong) -> TeeResult {
    tee_svc_close_enum(obj_enum)
}

/// reset the enumeration of the object
///
/// # Arguments
/// * `obj_enum` - the object enumeration id
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_reset_enum(obj_enum: c_ulong) -> TeeResult {
    let obj = tee_svc_storage_get_enum(obj_enum)?;
    // get the lock of the dir, and get the &mut TeeFsDir borrow
    let fops = {
        let obj_guard = obj.lock();
        obj_guard.fops
    };

    if let Some(fops) = fops {
        let mut obj_guard = obj.lock();
        {
            (fops.closedir)(obj_guard.dir.as_mut().ok_or(TEE_ERROR_BAD_STATE)?)?;
        }
        obj_guard.fops = None;
        obj_guard.dir = None;
    }

    let obj_guard = obj.lock();
    debug_assert!(obj_guard.dir.is_none());
    Ok(())
}

/// start the enumeration of the object
///
/// # Arguments
/// * `obj_enum` - the object enumeration id
/// * `storage_id` - the storage id
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_start_enum(obj_enum: c_ulong, storage_id: c_ulong) -> TeeResult {
    let fops = tee_svc_storage_file_ops(storage_id as u32).map_err(|e| TEE_ERROR_ITEM_NOT_FOUND)?;

    let e = tee_svc_storage_get_enum(obj_enum)?;

    let e_fops = {
        let obj_guard = e.lock();
        obj_guard.fops
    };

    let mut obj_guard = e.lock();
    if obj_guard.dir.is_some() {
        let e_fops = e_fops.ok_or(TEE_ERROR_BAD_STATE)?;
        (e_fops.closedir)(obj_guard.dir.as_mut().ok_or(TEE_ERROR_BAD_STATE)?)?;
        obj_guard.dir = None;
    }

    obj_guard.fops = Some(fops);

    let uuid = with_tee_ta_ctx(|ctx| Ok(ctx.uuid.clone()))?;
    let uuid = Uuid::parse_str(&uuid)?;

    let dir = (fops.opendir)(uuid.as_raw_ref())?;
    obj_guard.dir = Some(dir);
    Ok(())
}

/// get the next object in the enumeration
///
/// # Arguments
/// * `obj_enum` - the object enumeration id
/// * `info` - the information of the object
/// * `obj_id` - the object id
/// * `len` - the length of the object id
/// # Returns
/// * `TeeResult` - the result of the operation
pub fn syscall_storage_next_enum(
    obj_enum: c_ulong,
    info: *mut utee_object_info,
    obj_id: *mut c_void,
    len: *mut u64,
) -> TeeResult {
    let mut o: Option<Box<tee_obj>> = None;

    let res = (|| -> TeeResult {
        let e = tee_svc_storage_get_enum(obj_enum)?;

        let fops = {
            let obj_guard = e.lock();
            obj_guard.fops.ok_or(TEE_ERROR_ITEM_NOT_FOUND)?
        };

        let mut obj_guard = e.lock();
        let mut dir = obj_guard.dir.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;
        let mut d = tee_fs_dirent::default();
        (fops.readdir)(dir, &mut d)?;
        drop(obj_guard); // 释放 e 的锁，避免在 tee_pobj_get 中持有多个锁

        o = Some(Box::new(tee_obj::default()));
        let o = o.as_mut().ok_or(TEE_ERROR_BAD_STATE)?;

        let uuid = with_tee_ta_ctx(|ctx| Ok(ctx.uuid.clone()))?;
        let uuid = Uuid::parse_str(&uuid)?;

        let pobj = tee_pobj_get(
            uuid.as_raw_ref(),
            d.oid.as_ref(),
            d.oid_len as u32,
            0,
            tee_pobj_usage::TEE_POBJ_USAGE_ENUM,
            fops,
        )?;

        o.pobj = Some(pobj.clone());
        o.info.handleFlags =
            pobj.read().flags | TEE_HANDLE_FLAG_PERSISTENT | TEE_HANDLE_FLAG_INITIALIZED;

        let pobj_flags = {
            let pobj_guard = pobj.read();
            pobj_guard.flags
        };

        let mut bbuf: utee_object_info = utee_object_info::default();
        with_pobj_usage_lock(pobj_flags, || -> TeeResult {
            tee_svc_storage_read_head(o)?;

            let pobj_guard = pobj.read();
            bbuf.obj_type = o.info.objectType;
            bbuf.obj_size = o.info.objectSize;
            bbuf.max_obj_size = o.info.maxObjectSize;
            bbuf.obj_usage = pobj_guard.obj_info_usage;
            bbuf.data_size = o.info.dataSize as _;
            bbuf.data_pos = o.info.dataPosition as _;
            bbuf.handle_flags = o.info.handleFlags as _;
            Ok(())
        })?;

        let info_ref = unsafe { &mut *info };
        copy_to_user_struct(info_ref, &bbuf)?;

        let pobj_guard = pobj.read();
        let obj_id_len = pobj_guard.obj_id_len as usize;

        let obj_id_slice =
            unsafe { core::slice::from_raw_parts_mut(obj_id as *mut u8, obj_id_len) };
        copy_to_user(obj_id_slice, pobj_guard.obj_id.as_ref(), obj_id_len)?;

        let l = pobj_guard.obj_id_len as u64;
        drop(pobj_guard);

        let l_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(l as *const u64 as *const u8, core::mem::size_of::<u64>())
        };
        let len_bytes: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(len as *mut u64 as *mut u8, core::mem::size_of::<u64>())
        };
        copy_to_user_private(len_bytes, l_bytes, core::mem::size_of::<u64>())?;

        Ok(())
    })();

    if let Some(mut o) = o {
        if let Some(pobj) = o.pobj.take() {
            let fops = pobj.read().fops.ok_or(TEE_ERROR_BAD_STATE)?;

            (fops.close)(&mut o.fh);
            let _ = tee_pobj_release(pobj);
        }
    }

    res
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
    const TEE_DIRNAME_BUFFER_REQUIRED_LEN: usize = tee_b2hs_hsbuf_size(TEE_UUID_HEX_LEN) + 1;

    test_fn! {
        using TestResult;

        fn test_size_of_val() {
            assert_eq!(size_of_val(&tee_svc_storage_head::default()), size_of::<tee_svc_storage_head>());
        }
    }

    // Helper to create a TeeUuid from its raw byte representation for predictable testing
    // This assumes little-endian for u16/u32 fields, adjust if your target is big-endian.
    fn create_uuid_from_bytes(bytes: [u8; 16]) -> TEE_UUID {
        TEE_UUID {
            timeLow: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            timeMid: u16::from_le_bytes([bytes[4], bytes[5]]),
            timeHiAndVersion: u16::from_le_bytes([bytes[6], bytes[7]]),
            clockSeqAndNode: [
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ],
        }
    }

    // --- Tests for tee_svc_storage_create_dirname ---

    test_fn! {
        using TestResult;
        fn test_create_dirname_standard_uuid() {
            let uuid_bytes: [u8; 16] = [
                0x78, 0x56, 0x34, 0x12, // time_low (reversed for LE)
                0xBC, 0x9A,             // time_mid (reversed for LE)
                0xF0, 0xDE,             // time_hi_and_version (reversed for LE)
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, // clock_seq_and_node
            ];
            let uuid = create_uuid_from_bytes(uuid_bytes);

            // Use the defined constant for buffer size
            let mut buf = [0u8; TEE_DIRNAME_BUFFER_REQUIRED_LEN];
            let result = tee_svc_storage_create_dirname(&mut buf, &uuid);

            assert!(result.is_ok());
            // Verify the string content, excluding the final null terminator for str::from_utf8
            assert_eq!(str::from_utf8(&buf[..TEE_DIRNAME_BUFFER_REQUIRED_LEN - 1]).unwrap(), "/78563412BC9AF0DE1122334455667788");
            // Verify the final null terminator
            assert_eq!(buf[TEE_DIRNAME_BUFFER_REQUIRED_LEN - 1], 0);
        }
    }

    test_fn! {
        using TestResult;
        fn test_create_dirname_all_zeros_uuid() {
            let uuid = TEE_UUID {
                timeLow: 0, timeMid: 0, timeHiAndVersion: 0, clockSeqAndNode: [0; 8],
            };
            let mut buf = [0u8; TEE_DIRNAME_BUFFER_REQUIRED_LEN];
            let result = tee_svc_storage_create_dirname(&mut buf, &uuid);

            assert!(result.is_ok());
            assert_eq!(str::from_utf8(&buf[..TEE_DIRNAME_BUFFER_REQUIRED_LEN - 1]).unwrap(), "/00000000000000000000000000000000");
            assert_eq!(buf[TEE_DIRNAME_BUFFER_REQUIRED_LEN - 1], 0);
        }
    }

    test_fn! {
        using TestResult;
        fn test_create_dirname_specific_uuid_values() {
            let uuid_bytes: [u8; 16] = [
                0x01, 0x02, 0x03, 0x04,
                0x05, 0x06,
                0x07, 0x08,
                0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
            ];
            let uuid = create_uuid_from_bytes(uuid_bytes);
            let mut buf = [0u8; TEE_DIRNAME_BUFFER_REQUIRED_LEN];
            let result = tee_svc_storage_create_dirname(&mut buf, &uuid);

            assert!(result.is_ok());
            assert_eq!(str::from_utf8(&buf[..TEE_DIRNAME_BUFFER_REQUIRED_LEN - 1]).unwrap(), "/0102030405060708090A0B0C0D0E0F10");
            assert_eq!(buf[TEE_DIRNAME_BUFFER_REQUIRED_LEN - 1], 0);
        }
    }

    test_fn! {
        using TestResult;
        fn test_create_dirname_short_buffer() {
            let uuid = TEE_UUID {
                timeLow: 0, timeMid: 0, timeHiAndVersion: 0, clockSeqAndNode: [0; 8],
            };
            // Provide a buffer one byte smaller than required
            let mut buf = [0u8; TEE_DIRNAME_BUFFER_REQUIRED_LEN - 1];
            let result = tee_svc_storage_create_dirname(&mut buf, &uuid);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), TEE_ERROR_SHORT_BUFFER);
        }
    }

    test_fn! {
        using TestResult;
        fn test_create_dirname_empty_buffer() {
            let uuid = TEE_UUID {
                timeLow: 0, timeMid: 0, timeHiAndVersion: 0, clockSeqAndNode: [0; 8],
            };
            let mut buf = [0u8; 0];
            let result = tee_svc_storage_create_dirname(&mut buf, &uuid);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), TEE_ERROR_SHORT_BUFFER);
        }
    }

    test_fn! {
        using TestResult;
        fn test_create_dirname_exact_buffer() {
            let uuid = TEE_UUID {
                timeLow: 0xAABBCCDD, timeMid: 0xEEFF, timeHiAndVersion: 0x1122, clockSeqAndNode: [0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA],
            };
            let mut buf = [0u8; TEE_DIRNAME_BUFFER_REQUIRED_LEN];
            let result = tee_svc_storage_create_dirname(&mut buf, &uuid);

            assert!(result.is_ok());
            // Expected hex string based on LE byte order:
            // AABBCCDD -> "DDCCBBAA"
            // EEFF     -> "FFEE"
            // 1122     -> "2211"
            // 33..AA   -> "33445566778899AA"
            assert_eq!(str::from_utf8(&buf[..TEE_DIRNAME_BUFFER_REQUIRED_LEN - 1]).unwrap(), "/DDCCBBAAFFEE221133445566778899AA");
            assert_eq!(buf[TEE_DIRNAME_BUFFER_REQUIRED_LEN - 1], 0);
        }
    }

    // --- Additional tests for tee_b2hs if needed ---

    test_fn! {
        using TestResult;
        fn test_tee_b2hs_uppercase_conversion() {
            let b = &[0xab, 0xcd, 0xef];
            let mut hs = [0u8; tee_b2hs_hsbuf_size(3)]; // 3 bytes * 2 hex chars + 1 null = 7
            let result = tee_b2hs(b, &mut hs);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 6); // Returns length without null
            assert_eq!(str::from_utf8(&hs[..6]).unwrap(), "ABCDEF");
            assert_eq!(hs[6], 0); // Verify null terminator
            warn!("Hello from test debug");
        }
    }

    test_fn! {
        using TestResult;
        fn test_tee_b2hs_null_termination() {
            let b = &[0x12];
            let mut hs = [0u8; tee_b2hs_hsbuf_size(1)]; // 1 byte * 2 hex chars + 1 null = 3
            let result = tee_b2hs(b, &mut hs);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 2);
            assert_eq!(str::from_utf8(&hs[..2]).unwrap(), "12");
            assert_eq!(hs[2], 0); // Verify null terminator
        }
    }

    test_fn! {
        using TestResult;
        fn test_tee_b2hs_short_output_buffer() {
            let b = &[0x12, 0x34]; // Needs 4 hex chars + 1 null = 5 bytes
            let mut hs = [0u8; tee_b2hs_hsbuf_size(2) - 1]; // Provide 1 byte less than required
            let result = tee_b2hs(b, &mut hs);
            assert!(result.is_err());
        }
    }

    test_fn! {
        using TestResult;
        fn test_syscall_storage_obj_create_type_data() {
            // step 1 : create a new object
            let storage_id = TEE_STORAGE_PRIVATE as c_ulong;
            let object_id = "test_object_create";
            let object_id_len = object_id.len();
            let flags = TEE_DATA_FLAG_ACCESS_READ
                | TEE_DATA_FLAG_ACCESS_WRITE
                | TEE_DATA_FLAG_ACCESS_WRITE_META
                | TEE_DATA_FLAG_OVERWRITE;
            // TEE_TYPE_DATA has no attributes
            let attr = TEE_HANDLE_NULL;
            let data_create = b"test_data";
            let len = data_create.len();
            let mut obj = 0 as c_uint;
            let result = syscall_storage_obj_create(storage_id,
                object_id.as_ptr() as *mut c_void, object_id_len,
                flags as c_ulong,
                attr as c_ulong,
                data_create.as_ptr() as *mut c_void,
                len,
                &mut obj as *mut c_uint);
            tee_debug!("result: {:X?}", result);
            assert!(result.is_ok());

            // step 2 : read the object
            let mut data_read = vec![0u8; data_create.len()];
            let mut count = 0 as u64;
            let mut result = syscall_storage_obj_read(obj as c_ulong, data_read.as_ptr() as *mut c_void, data_read.len(), &mut count);
            assert!(result.is_ok());
            tee_debug!("data: {:?}, count: 0x{:X?}", data_read, count);
            assert_eq!(data_read, b"test_data");
            assert_eq!(count, data_create.len() as u64);
            // assert_eq!(str::from_utf8(&data[..len]).unwrap(), "test_data");

            // step 3 : syscall_storage_obj_write
            let data_write = b"TEST_DATA";
            let len = data_write.len();
            let result = syscall_storage_obj_write(obj as c_ulong, data_write.as_ptr() as *mut c_void, len);
            assert!(result.is_ok());

            // step 4 : seek
            let result = syscall_storage_obj_seek(obj as c_ulong, -(data_write.len() as i32), TEE_DATA_SEEK_CUR as c_ulong);
            assert!(result.is_ok());

            // step 5 : read
            let mut data_read = vec![0u8; data_write.len()];
            let mut count = 0 as u64;
            let mut result = syscall_storage_obj_read(obj as c_ulong, data_read.as_ptr() as *mut c_void, data_read.len(), &mut count);
            assert!(result.is_ok());
            tee_debug!("data: {:?}, count: 0x{:X?}", data_read, count);
            assert_eq!(data_read, b"TEST_DATA");
            assert_eq!(count, data_write.len() as u64);

            // step 6 : truncate
            let len = data_create.len();
            let result = syscall_storage_obj_trunc(obj as c_ulong, len);
            assert!(result.is_ok());
            let result = syscall_storage_obj_seek(obj as c_ulong, 0, TEE_DATA_SEEK_SET as c_ulong);
            assert!(result.is_ok());
            // step 7 : read
            let mut data_read = vec![0u8; data_create.len()];
            let mut count = 0 as u64;
            let mut result = syscall_storage_obj_read(obj as c_ulong, data_read.as_ptr() as *mut c_void, data_read.len(), &mut count);
            assert!(result.is_ok());
            tee_debug!("data: {:?}, count: 0x{:X?}", data_read, count);
            assert_eq!(data_read, b"test_data");
            assert_eq!(count, data_create.len() as u64);
            // read failed because the object is truncated
            let mut data_read = vec![0u8; 1];
            let mut count = 0 as u64;
            let mut result = syscall_storage_obj_read(obj as c_ulong, data_read.as_ptr() as *mut c_void, data_read.len(), &mut count);
            // assert!(result.is_err());
            assert_eq!(count, 0);
            // seek to the overflow position , is ok
            let result = syscall_storage_obj_seek(obj as c_ulong, (data_create.len() + 1) as i32, TEE_DATA_SEEK_SET as c_ulong);
            assert!(result.is_ok());

            // step 7 : get info to check size
            let mut info = utee_object_info::default();
            let result = syscall_cryp_obj_get_info(obj as c_ulong, &mut info);

            tee_debug!("info: {:?}", info);
            assert!(result.is_ok());
            assert_eq!(info.data_size, data_create.len() as u32);
            assert!(info.handle_flags&(TEE_HANDLE_FLAG_PERSISTENT
                | TEE_HANDLE_FLAG_INITIALIZED
                | TEE_DATA_FLAG_ACCESS_READ
                | TEE_DATA_FLAG_ACCESS_WRITE
                | TEE_DATA_FLAG_ACCESS_WRITE_META) != 0);
            assert_eq!(info.obj_type, TEE_TYPE_DATA);

            // step 8 : rename object
            let object_id_new = "test_object_new";
            let object_id_new_len = object_id_new.len();
            let result = syscall_storage_obj_rename(obj as c_ulong, object_id_new.as_ptr() as *mut c_void, object_id_new_len);
            tee_debug!("result: {:X?}", result);
            assert!(result.is_ok());

            // // step 9 : get info to check size
            // let mut info = utee_object_info::default();
            // let result = syscall_cryp_obj_get_info(obj as c_ulong, &mut info);

            // step 8 : close the object
            let obj_id = obj as c_ulong;
            let result = syscall_storage_obj_del(obj_id);
            assert!(result.is_ok());
            // check if the object is deleted
            let result = tee_obj_get(obj_id as tee_obj_id_type);
            assert!(matches!(result, Err(TEE_ERROR_ITEM_NOT_FOUND)));
        }
    }

    test_fn! {
        using TestResult;
        fn test_syscall_storage_init() {
        }
    }

    test_fn! {
        using TestResult;
        fn test_syscall_storage_obj_open() {
            let storage_id = TEE_STORAGE_PRIVATE as c_ulong;
            let object_id = "test_object";
            let object_id_len = object_id.len();
            let flags = TEE_DATA_FLAG_ACCESS_READ | TEE_DATA_FLAG_ACCESS_WRITE;
            let mut obj = 0 as c_uint  ;
            let result = syscall_storage_obj_open(storage_id, object_id.as_ptr() as *mut c_void, object_id_len, flags as c_ulong, &mut obj as *mut c_uint);
            info!("result: Err(0x{:X})", result.unwrap_err());
            assert!(result.is_ok());
        }
    }

    tests_name! {
        TEST_TEE_SVC_STORAGE;
        //------------------------
        test_size_of_val,
        test_create_dirname_standard_uuid,
        test_create_dirname_all_zeros_uuid,
        test_create_dirname_specific_uuid_values,
        test_create_dirname_short_buffer,
        test_create_dirname_empty_buffer,
        test_create_dirname_exact_buffer,
        test_tee_b2hs_uppercase_conversion,
        test_tee_b2hs_null_termination,
        test_tee_b2hs_short_output_buffer,
        //------------------------
        test_syscall_storage_init,
        // test_syscall_storage_obj_open,
        test_syscall_storage_obj_create_type_data,
    }
}
