// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{
    boxed::Box,
    sync::Arc,
    vec::{self, Vec},
};
use bincode::de;
use core::{default, ffi::c_ulong};

use axerrno::{AxError, AxResult};
use axtask::current;
use flatten_objects::FlattenObjects;
use slab::Slab;
use spin::{Mutex, RwLock};
use starry_core::task::{AsThread, TeeSessionCtxTrait};
use tee_raw_sys::{libc_compat::size_t, *};

use super::{
    TeeResult,
    libmbedtls::bignum::BigNum,
    tee_ree_fs::tee_file_handle,
    tee_pobj::tee_pobj,
    tee_session::{tee_session_ctx, with_tee_session_ctx, with_tee_session_ctx_mut},
    tee_svc_cryp::{TeeCryptObj, TeeCryptObjAttr},
};

pub type tee_obj_id_type = c_ulong; //usize;
/// The maximum number of open files
pub const AX_TEE_OBJ_LIMIT: usize = 1024;

// scope_local::scope_local! {
//     /// The open objects for TA.
//     pub static TEE_OBJ_TABLE: Arc<RwLock<Slab<Arc<tee_obj>>>> = Arc::default();
// }

#[repr(C)]
#[derive(Debug)]
pub struct tee_obj {
    pub info: TEE_ObjectInfo,
    busy: bool,          // true if used by an operation
    pub have_attrs: u32, // bitfield identifying set properties
    // void *attr;
    pub attr: Vec<TeeCryptObj>,
    pub ds_pos: size_t,
    pub pobj: Arc<RwLock<tee_pobj>>,
    /// file handle for the pobject
    pub fh: Box<tee_file_handle>,
}

impl default::Default for tee_obj {
    fn default() -> Self {
        tee_obj {
            info: TEE_ObjectInfo {
                objectId: 0,
                objectType: 0,
                objectSize: 0,
                maxObjectSize: 0,
                objectUsage: 0,
                dataSize: 0,
                dataPosition: 0,
                handleFlags: 0,
            },
            busy: false,
            have_attrs: 0,
            attr: Vec::new(),
            ds_pos: 0,
            pobj: Arc::new(RwLock::new(tee_pobj::default())),
            fh: Box::new(tee_file_handle::default()),
        }
    }
}

pub fn tee_obj_add(mut obj: tee_obj) -> TeeResult<tee_obj_id_type> {
    with_tee_session_ctx_mut(|ctx| {
        // 获取一个可用的 ID
        let vacant = ctx.objects.vacant_entry();
        let id = vacant.key();
        
        // 设置 objectId
        obj.info.objectId = id as u32;
        
        // 创建 Arc 并插入
        let arc_obj = Arc::new(Mutex::new(obj));
        let inserted_id = vacant.insert(arc_obj);
        info!("tee_obj_add: id: {}", id);
        
        Ok(id as tee_obj_id_type)
    })
}

pub fn tee_obj_get(obj_id: tee_obj_id_type) -> TeeResult<Arc<Mutex<tee_obj>>> {
    with_tee_session_ctx(|ctx| match ctx.objects.get(obj_id as _) {
        Some(obj) => Ok(Arc::clone(&obj)),
        None => Err(TEE_ERROR_ITEM_NOT_FOUND),
    })
}

pub fn tee_obj_delete(obj: &mut tee_obj) {
    // remove from session objects
    with_tee_session_ctx_mut(|ctx| {
        ctx.objects.remove(obj.info.objectId as _);
        Ok(())
    });
}

pub fn tee_obj_close(obj: &mut tee_obj) {
    tee_obj_delete(obj);

    if obj.info.handleFlags & TEE_HANDLE_FLAG_PERSISTENT != 0 {
        // TODO: implement fops close
        //obj.pobj.fops.close(&obj.fh);
        // tee_pobj_release(obj.pobj);
    }

    // tee_obj_free(obj);
}

#[cfg(feature = "tee_test")]
pub mod tests_tee_obj {
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

        fn test_tee_obj_add_get() {
            let mut obj = tee_obj::default();
            obj.busy = true;
            let obj_id = tee_obj_add(obj).expect("Failed to add tee_obj");
            info!("Added tee_obj with id {}", obj_id);
            let retrieved_obj = tee_obj_get(obj_id).expect("Failed to get tee_obj");
            assert_eq!(retrieved_obj.lock().busy, true);
        }
    }

    tests_name! {
        TEST_TEE_OBJ;
        //------------------------
        test_tee_obj_add_get,
    }
}
