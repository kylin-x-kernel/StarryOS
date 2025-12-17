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
use axerrno::{AxError, AxResult};
use axtask::current;
use core::{default, ffi::c_ulong};
use flatten_objects::FlattenObjects;
use slab::Slab;
use spin::RwLock;
use starry_core::task::{AsThread, TeeSessionCtxTrait};
use tee_raw_sys::libc_compat::size_t;
use tee_raw_sys::*;

use super::{
    TeeResult,
    libmbedtls::bignum::BigNum,
    tee_fs::tee_file_handle,
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
pub struct tee_obj {
    pub info: TEE_ObjectInfo,
    busy: bool,          /* true if used by an operation */
    pub have_attrs: u32, /* bitfield identifying set properties */
    //void *attr;
    pub attr: Vec<TeeCryptObj>,
    ds_pos: size_t,
    pub pobj: Arc<tee_pobj>,
    fh: Arc<tee_file_handle>,
}

impl default::Default for tee_obj {
    fn default() -> Self {
        tee_obj {
            info: TEE_ObjectInfo {
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
            pobj: Arc::new(tee_pobj::default()),
            fh: Arc::new(tee_file_handle {}),
        }
    }
}

pub fn tee_obj_add(obj: tee_obj) -> TeeResult<tee_obj_id_type> {
    with_tee_session_ctx_mut(|ctx| {
        let id = ctx.objects.insert(Arc::new(obj));
        Ok(id as tee_obj_id_type)
    })
}

pub fn tee_obj_get(obj_id: tee_obj_id_type) -> TeeResult<Arc<tee_obj>> {
    with_tee_session_ctx(|ctx| match ctx.objects.get(obj_id as _) {
        Some(obj) => Ok(Arc::clone(obj)),
        None => Err(TEE_ERROR_ITEM_NOT_FOUND),
    })
}

#[cfg(feature = "tee_test")]
pub mod tests_tee_obj {
    //-------- test framework import --------
    use crate::tee::TestDescriptor;
    use crate::tee::TestResult;
    use crate::test_fn;
    use crate::{assert, assert_eq, assert_ne, tests, tests_name};

    //-------- local tests import --------
    use super::*;

    test_fn! {
        using TestResult;

        fn test_tee_obj_add_get() {
            let mut obj = tee_obj::default();
            obj.busy = true;
            let obj_id = tee_obj_add(obj).expect("Failed to add tee_obj");
            info!("Added tee_obj with id {}", obj_id);
            let retrieved_obj = tee_obj_get(obj_id).expect("Failed to get tee_obj");
            assert_eq!(retrieved_obj.busy, true);
        }
    }

    tests_name! {
        TEST_TEE_OBJ;
        //------------------------
        test_tee_obj_add_get,
    }
}
