// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use crate::{
    assert, assert_eq, assert_ne, run_tests,
    tee::{TestDescriptor, TestRunner},
    test_fn, tests, tests_name,
};

pub fn tee_test_unit() {
    use super::{
        libmbedtls::bignum::tests_tee_bignum::TEST_TEE_BIGNUM,
        tee_obj::tests_tee_obj::TEST_TEE_OBJ, tee_pobj::tests_tee_pobj::TEST_TEE_POBJ,
        tee_session::tests_tee_session::TEST_TEE_SESSION,
        tee_svc_cryp::tests_tee_svc_cryp::TEST_TEE_SVC_CRYP,
        user_access::tests_user_access::TEST_USER_ACCESS,
    };

    let mut runner = TestRunner::new();
    run_tests!(
        runner,
        [
            TEST_TEE_POBJ,
            TEST_TEE_OBJ,
            TEST_TEE_SVC_CRYP,
            TEST_USER_ACCESS,
            TEST_TEE_BIGNUM,
            TEST_TEE_SESSION
        ]
    );
}
