// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{
    boxed::Box,
    collections::VecDeque,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{default, ptr};

use lazy_static::lazy_static;
use spin::{Mutex, RwLock};
use tee_raw_sys::*;

use super::{TeeResult, tee_fs::tee_file_handle};

static POBJS_USAGE_MUTEX: Mutex<()> = Mutex::new(());
// static POBJS: LazyInit<Arc<Mutex<VecDeque<tee_pobj>>>> = LazyInit::new();
lazy_static! {
    static ref POBJS: tee_pobjs = tee_pobjs::new();
}

#[derive(Debug)]
pub struct TeeFileOperations {
    pub open: fn(
        po: &tee_pobj,
        size: &mut usize,
    ) -> TeeResult<Arc<tee_file_handle>>,

    pub create: fn(
        po: &tee_pobj,
        overwrite: bool,
        head: &[u8],
        attr: &[u8],
        data_core: &[u8],
        data_user: &[u8],
        data_size: usize,
    ) -> TeeResult<Arc<tee_file_handle>>,

    pub close: fn(
        fh: &mut Arc<tee_file_handle>,
    ) -> TeeResult,

    pub read: fn(
        fh: &Arc<tee_file_handle>,
        pos: usize,
        buf_core: &mut [u8],
        buf_user: &mut [u8],
        len: &mut usize,
    ) -> TeeResult,

    pub write: fn(
        fh: &Arc<tee_file_handle>,
        pos: usize,
        buf_core: &[u8],
        buf_user: &[u8],
        len: usize,
    ) -> TeeResult,

    pub rename: fn(
        old_po: &mut tee_pobj,
        new_po: &mut tee_pobj,
        overwrite: bool,
    ) -> TeeResult,

    pub remove: fn(
        po: &mut tee_pobj,
    ) -> TeeResult,

    pub truncate: fn(
        fh: &mut Arc<tee_file_handle>,
        size: usize,
    ) -> TeeResult,

    //fn opendir(uuid: &TEE_UUID, d: &mut Arc<tee_fs_dir>) -> TeeResult;
    
    //fn readdir(d: &mut Arc<tee_fs_dir>, ent: &mut Arc<tee_fs_dirent>) -> TeeResult;

    //fn closedir(d: &mut Arc<tee_fs_dir>) -> TeeResult;

    #[cfg(feature = "tee_test")]
    pub echo: fn() -> String,
}


#[repr(C)]
#[derive(Debug)]
pub struct tee_pobj {
    pub refcnt: u32,
    pub uuid: TEE_UUID,
    pub obj_id: Box<[u8]>,
    pub obj_id_len: u32,
    pub flags: u32,
    pub obj_info_usage: u32,
    pub temporary: bool, // can be changed while creating == true
    pub creating: bool,  // can only be changed with mutex held
    pub fops: Option<&'static TeeFileOperations>, // Filesystem handling this object
}

impl default::Default for tee_pobj {
    fn default() -> Self {
        tee_pobj {
            refcnt: 0,
            uuid: TEE_UUID::default(),
            obj_id: Box::new([]),
            obj_id_len: 0,
            flags: 0,
            obj_info_usage: 0,
            temporary: false,
            creating: false,
            fops: None,
        }
    }
}

impl tee_pobj {
    /// Create a new tee_pobj
    ///
    /// # Arguments
    /// * `uuid` - The UUID of the object
    /// * `obj_id` - The object ID
    /// * `obj_id_len` - The actual length of the object ID
    /// * `flags` - The flags of the object
    /// * `fops` - The reference to the TeeFileOperations struct
    pub fn new(
        uuid: TEE_UUID,
        obj_id: &[u8],
        obj_id_len: u32,
        flags: u32,
        fops: &'static TeeFileOperations,
    ) -> Self {
        Self {
            refcnt: 1,
            uuid,
            obj_id: obj_id.to_vec().into_boxed_slice(),
            obj_id_len,
            flags,
            obj_info_usage: 0,
            temporary: false,
            creating: false,
            fops: Some(fops),
        }
    }

    /// Check if the tee_pobj matches the given parameters
    ///
    /// # Arguments
    /// * `uuid` - The UUID of the object
    /// * `obj_id` - The object ID
    /// * `obj_id_len` - The actual length of the object ID
    /// * `fops` - The reference to the TeeFileOperations struct
    pub fn matches(
        &self,
        uuid: &TEE_UUID,
        obj_id: &[u8],
        obj_id_len: u32,
        fops: &Option<&'static TeeFileOperations>,
    ) -> bool {
        info!("matches begin");
        // check obj_id_len
        if self.obj_id_len != obj_id_len {
            return false;
        }
        // check obj_id
        if self.obj_id[..obj_id_len as usize] != obj_id[..obj_id_len as usize] {
            return false;
        }
        // check uuid
        if self.uuid != *uuid {
            return false;
        }
        info!("matches fops with {:?}, {:?}", self.fops, fops);
        // check fops, using ptr::eq
        match (&self.fops, fops) {
            (Some(a), Some(b)) => {
                info!("matches fops: {:p}, {:p}", *a as *const _, *b as *const _);
                let result = ptr::eq(*a, *b);
                info!("matches fops result: {}", result);
                result
            }
            (None, None) => true,
            _ => false,
        }
    }
}

/// A collection of tee_pobjs
///
/// must ensure process safe and thread safe
#[derive(Debug)]
pub struct tee_pobjs {
    inner: Arc<Mutex<VecDeque<Arc<RwLock<tee_pobj>>>>>,
}

impl tee_pobjs {
    /// Create a new tee_pobjs
    pub fn new() -> Self {
        tee_pobjs {
            inner: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Find a tee_pobj in the collection
    ///
    /// # Arguments
    /// * `uuid` - The UUID of the object
    /// * `obj_id` - The object ID
    /// * `obj_id_len` - The actual length of the object ID
    /// * `fops` - The reference to the TeeFileOperations struct
    pub fn find_pobj(
        &self,
        uuid: &TEE_UUID,
        obj_id: &[u8],
        obj_id_len: u32,
        fops: &Option<&'static TeeFileOperations>,
    ) -> Option<Arc<RwLock<tee_pobj>>> {
        let pobjs = self.inner.lock();
        pobjs
            .iter()
            .find(|pobj_arc| {
                let pobj_guard = pobj_arc.read();
                pobj_guard.matches(uuid, obj_id, obj_id_len, fops)
            })
            .map(|pobj_arc| Arc::clone(pobj_arc))
    }
}

// Helper functions for REE_FS_OPS
fn ree_fs_open(_po: &tee_pobj, _size: &mut usize) -> TeeResult<Arc<tee_file_handle>> {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_create(
    _po: &tee_pobj,
    _overwrite: bool,
    _head: &[u8],
    _attr: &[u8],
    _data_core: &[u8],
    _data_user: &[u8],
    _data_size: usize,
) -> TeeResult<Arc<tee_file_handle>> {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_close(_fh: &mut Arc<tee_file_handle>) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_read(
    _fh: &Arc<tee_file_handle>,
    _pos: usize,
    _buf_core: &mut [u8],
    _buf_user: &mut [u8],
    _len: &mut usize,
) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_write(
    _fh: &Arc<tee_file_handle>,
    _pos: usize,
    _buf_core: &[u8],
    _buf_user: &[u8],
    _len: usize,
) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_truncate(_fh: &mut Arc<tee_file_handle>, _size: usize) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_rename(_old_po: &mut tee_pobj, _new_po: &mut tee_pobj, _overwrite: bool) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

fn ree_fs_remove(_po: &mut tee_pobj) -> TeeResult {
    Err(TEE_ERROR_NOT_SUPPORTED)
}

#[cfg(feature = "tee_test")]
fn ree_fs_echo() -> String {
    "TeeFileOperations->echo".to_string()
}

// global file_ops
pub static REE_FS_OPS: TeeFileOperations = TeeFileOperations {
    open: ree_fs_open,
    create: ree_fs_create,
    close: ree_fs_close,
    read: ree_fs_read,
    write: ree_fs_write,
    truncate: ree_fs_truncate,
    rename: ree_fs_rename,
    remove: ree_fs_remove,
    #[cfg(feature = "tee_test")]
    echo: ree_fs_echo,
};

/// Usage of the tee_pobj
#[derive(PartialEq, Debug)]
pub enum tee_pobj_usage {
    TEE_POBJ_USAGE_OPEN = 0,
    TEE_POBJ_USAGE_RENAME = 1,
    TEE_POBJ_USAGE_CREATE = 2,
    TEE_POBJ_USAGE_ENUM = 3,
}

/// Check if the tee_pobj needs usage lock
///
/// # Arguments
/// * `obj` - The tee_pobj
fn pobj_need_usage_lock(flags: u32) -> bool {
    flags & (TEE_DATA_FLAG_SHARE_WRITE | TEE_DATA_FLAG_SHARE_READ) != 0
}

/// With usage lock
///
/// # Arguments
/// * `obj` - The tee_pobj
/// * `f` - The function to execute
pub fn with_pobj_usage_lock<R, F>(flags: u32, f: F) -> R
where
    F: FnOnce() -> R,
{
    if pobj_need_usage_lock(flags) {
        let _guard = POBJS_USAGE_MUTEX.lock();
        f()
    } else {
        f()
    }
}

fn tee_pobj_check_access(oflags: u32, nflags: u32) -> TeeResult {
    Ok(())
}

/// Get a tee_pobj from the collection
///
/// # Arguments
/// * `uuid` - The UUID of the object
/// * `obj_id` - The object ID
/// * `obj_id_len` - The actual length of the object ID
/// * `flags` - The flags of the object
/// * `usage` - The usage of the tee_pobj
/// * `fops` - The reference to the TeeFileOperations struct
///
/// # Returns
/// * The tee_pobj, can safe shared reference
pub fn tee_pobj_get(
    uuid: &TEE_UUID,
    obj_id: &[u8],
    obj_id_len: u32,
    flags: u32,
    usage: tee_pobj_usage,
    fops: &'static TeeFileOperations,
) -> TeeResult<Arc<RwLock<tee_pobj>>> {
    info!(
        "tee_pobj_get: uuid: {:x?}, obj_id: {:x?}, obj_id_len: {}, flags: {}, usage: {:?}, fops: \
         {:p}",
        uuid, obj_id, obj_id_len, flags, usage, fops as *const _
    );
    // lock the pobjs
    if let Some(obj) = POBJS.find_pobj(
        uuid,
        obj_id,
        obj_id_len,
        &Some(fops),
    ) {
        let mut obj_guard = obj.write();

        if usage == tee_pobj_usage::TEE_POBJ_USAGE_ENUM {
            obj_guard.refcnt += 1;
            return Ok(Arc::clone(&obj));
        }

        if obj_guard.creating
            || (usage == tee_pobj_usage::TEE_POBJ_USAGE_CREATE
                && (flags & TEE_DATA_FLAG_OVERWRITE) == 0)
        {
            return Err(TEE_ERROR_ACCESS_CONFLICT);
        }

        tee_pobj_check_access(obj_guard.flags, flags);
        obj_guard.refcnt += 1;
        return Ok(Arc::clone(&obj));
    }

    // new file
    let mut obj = tee_pobj::default();
    obj.refcnt = 1;
    obj.uuid = *uuid;
    obj.flags = flags;
    obj.fops = Some(fops);

    if usage == tee_pobj_usage::TEE_POBJ_USAGE_CREATE {
        obj.temporary = true;
        obj.creating = true;
    }

    // copy obj_id
    obj.obj_id = obj_id[..obj_id_len as usize].to_vec().into_boxed_slice();
    obj.obj_id_len = obj_id_len;

    // add to pobjs
    let mut pobjs = POBJS.inner.lock();
    let pobj = Arc::new(RwLock::new(obj));
    pobjs.push_back(pobj.clone());
    return Ok(pobj);
}

#[cfg(feature = "tee_test")]
pub mod tests_tee_pobj {
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

        fn test_tee_pobj_default() {
            let pobj = tee_pobj::default();
            assert_eq!(pobj.obj_id_len, 0);
        }
    }

    test_fn! {
        using TestResult;

        fn test_with_pobj_usage_lock() {
            let mut pobj = tee_pobj::default();
            let result: Result<(), ()> = with_pobj_usage_lock(pobj.flags, || {
                Ok(())
            });
            assert_eq!(result, Ok::<(), ()>(()));
            // set flag
            pobj.flags = TEE_DATA_FLAG_SHARE_WRITE;
            let result: Result<(), ()> = with_pobj_usage_lock(pobj.flags, || {
                Ok(())
            });
            assert_eq!(result, Ok::<(), ()>(()));
            // set flag
            pobj.flags = TEE_DATA_FLAG_SHARE_READ;
            let result: Result<(), ()> = with_pobj_usage_lock(pobj.flags, || {
                Ok(())
            });
            assert_eq!(result, Ok::<(), ()>(()));
        }
    }

    test_fn! {
        using TestResult;

        fn test_tee_pobj_get() {
            // 1. create a new pobj
            let obj_id = [0x12, 0x34, 0x56, 0x78];
            {
                let result = tee_pobj_get(&TEE_UUID::default(), &obj_id, obj_id.len() as u32, 0, tee_pobj_usage::TEE_POBJ_USAGE_ENUM, &REE_FS_OPS);
                assert_eq!(result.is_ok(), true);
                // check VecQueue size
                let mut pobjs = POBJS.inner.lock();
                assert_eq!(pobjs.len(), 1);
                // check pobj
                let pobj = result.unwrap();
                let pobj_guard = pobj.read();
                assert_eq!(pobj_guard.obj_id, obj_id.to_vec().into_boxed_slice());
                assert_eq!(pobj_guard.obj_id_len, obj_id.len() as u32);
                assert_eq!(pobj_guard.flags, 0);
                assert_eq!(pobj_guard.fops.unwrap() as *const TeeFileOperations, &REE_FS_OPS as *const TeeFileOperations);
                let echo = (pobj_guard.fops.unwrap().echo)();
                assert_eq!(echo, "TeeFileOperations->echo");
            }
            // 2. get the same pobj
            {
                let result = tee_pobj_get(&TEE_UUID::default(), &obj_id, obj_id.len() as u32, 0, tee_pobj_usage::TEE_POBJ_USAGE_ENUM, &REE_FS_OPS);
                assert_eq!(result.is_ok(), true);
                // check VecQueue size
                let mut pobjs = POBJS.inner.lock();
                assert_eq!(pobjs.len(), 1);
                // check pobj
                let pobj = result.unwrap();
                let pobj_guard = pobj.read();
                assert_eq!(pobj_guard.obj_id, obj_id.to_vec().into_boxed_slice());
                assert_eq!(pobj_guard.obj_id_len, obj_id.len() as u32);
                assert_eq!(pobj_guard.flags, 0);
                assert_eq!(pobj_guard.fops.unwrap() as *const TeeFileOperations, &REE_FS_OPS as *const TeeFileOperations);
                let echo = (pobj_guard.fops.unwrap().echo)();
                assert_eq!(echo, "TeeFileOperations->echo");
            }
        }
    }

    tests_name! {
        TEST_TEE_POBJ;
        //------------------------
        test_tee_pobj_default,
        test_with_pobj_usage_lock,
        test_tee_pobj_get,
    }
}
