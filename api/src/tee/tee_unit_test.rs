use crate::tee::{TestDescriptor, TestRunner};
use crate::test_fn;
use crate::{assert, assert_eq, assert_ne, run_tests, tests, tests_name};

pub fn tee_test_unit() {
    use super::tee_obj::tests_tee_obj::TEST_TEE_OBJ;
    use super::tee_pobj::tests_tee_pobj::TEST_TEE_POBJ;
    use super::tee_svc_cryp::tests_tee_svc_cryp::TEST_TEE_SVC_CRYP;
    use super::user_access::tests_user_access::TEST_USER_ACCESS;
    use super::libmbedtls::bignum::tests_tee_bignum::TEST_TEE_BIGNUM;

    let mut runner = TestRunner::new();
    run_tests!(
        runner,
        [
            TEST_TEE_POBJ,
            TEST_TEE_OBJ,
            TEST_TEE_SVC_CRYP,
            TEST_USER_ACCESS,
            TEST_TEE_BIGNUM
        ]
    );
}
