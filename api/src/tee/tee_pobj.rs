// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use core::default;
use spin::Mutex;
use tee_raw_sys::*;

static POBJS_USAGE_MUTEX: Mutex<()> = Mutex::new(());

#[repr(C)]
pub struct tee_pobj {
    obj_id_len: u32,
    pub flags: u32,
    pub obj_info_usage: u32,
}

impl default::Default for tee_pobj {
    fn default() -> Self {
        tee_pobj {
            obj_id_len: 0,
            flags: 0,
            obj_info_usage: 0,
        }
    }
}

fn pobj_need_usage_lock(obj: &tee_pobj) -> bool {
    obj.flags & (TEE_DATA_FLAG_SHARE_WRITE | TEE_DATA_FLAG_SHARE_READ) != 0
}

pub fn with_pobj_usage_lock<R, F>(obj: &tee_pobj, f: F) -> R
where
    F: FnOnce() -> R,
{
    if pobj_need_usage_lock(obj) {
        let _guard = POBJS_USAGE_MUTEX.lock();
        f()
    } else {
        f()
    }
}

#[cfg(feature = "tee_test")]
pub mod tests_tee_pobj {
    //-------- test framework import --------
    use crate::tee::TestDescriptor;
    use crate::tee::TestResult;
    use crate::test_fn;
    use crate::{assert, assert_eq, assert_ne, tests, tests_name};

    //-------- local tests import --------
    use super::*;

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
            let result: Result<(), ()> = with_pobj_usage_lock(&pobj, || {
                Ok(())
            });
            assert_eq!(result, Ok::<(), ()>(()));
            // set flag
            pobj.flags = TEE_DATA_FLAG_SHARE_WRITE;
            let result: Result<(), ()> = with_pobj_usage_lock(&pobj, || {
                Ok(())
            });
            assert_eq!(result, Ok::<(), ()>(()));
            // set flag
            pobj.flags = TEE_DATA_FLAG_SHARE_READ;
            let result: Result<(), ()> = with_pobj_usage_lock(&pobj, || {
                Ok(())
            });
            assert_eq!(result, Ok::<(), ()>(()));
        }
    }

    tests_name! {
        TEST_TEE_POBJ;
        //------------------------
        test_tee_pobj_default,
        test_with_pobj_usage_lock,
    }
}
