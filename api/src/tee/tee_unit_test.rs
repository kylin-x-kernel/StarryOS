// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use crate::{
    assert, assert_eq, assert_ne, run_tests,
    tee::{TestDescriptor, TestRunner, test::test_framework::tests_failed},
    test_fn, tests, tests_name,
};

pub fn tee_test_unit() {
    use super::{
        common::file_ops::tests_file_ops::TEST_FILE_OPS,
        crypto::crypto_impl::tests_tee_crypto_impl::TEST_TEE_CRYPTO_IMPL,
        crypto_temp::aes_cbc::tests_aes_cbc::TEST_TEE_AES_CBC,
        fs_dirfile::tests_tee_fs_dirfile::TEST_TEE_FS_DIRFILE,
        fs_htree::tests_fs_htree::TEST_FS_HTREE,
        fs_htree_tests::tests_fs_htree_tests::TEST_FS_HTREE_TESTS,
        libmbedtls::bignum::tests_tee_bignum::TEST_TEE_BIGNUM,
        rng_software::tests_rng_software::TEST_RNG_SOFTWARE,
        tee_misc::tests_tee_misc::TEST_TEE_MISC, tee_obj::tests_tee_obj::TEST_TEE_OBJ,
        tee_pobj::tests_tee_pobj::TEST_TEE_POBJ, tee_ree_fs::tests_tee_ree_fs::TEST_TEE_REE_FS,
        tee_session::tests_tee_session::TEST_TEE_SESSION,
        tee_svc_cryp::tests_tee_svc_cryp::TEST_TEE_SVC_CRYP,
        tee_svc_storage::tests_tee_svc_storage::TEST_TEE_SVC_STORAGE,
        user_access::tests_user_access::TEST_USER_ACCESS, utils::tests_utils::TEST_TEE_UTILS,
    };

    let mut runner = TestRunner::new();
    run_tests!(
        runner,
        [
            TEST_TEE_POBJ,
            TEST_TEE_OBJ,
            TEST_TEE_SVC_CRYP,
            TEST_TEE_SVC_STORAGE,
            TEST_USER_ACCESS,
            TEST_TEE_BIGNUM,
            TEST_TEE_SESSION,
            TEST_FILE_OPS,
            TEST_TEE_FS_DIRFILE,
            TEST_TEE_UTILS,
            TEST_TEE_MISC,
            TEST_TEE_REE_FS,
            TEST_FS_HTREE,
            TEST_FS_HTREE_TESTS,
            TEST_RNG_SOFTWARE,
            TEST_TEE_CRYPTO_IMPL,
            TEST_TEE_AES_CBC,
        ]
    );

    if tests_failed() {
        error!("!!! SOME TESTS FAILED, NEED TO BE FIXED !!!");
    } else {
        info!("!!! ALL TESTS PASSED !!!");
    }
}
