use crate::{
    run_tests,
    tee::{
        tee_session::tests_tee_session::TEST_TEE_SESSION,
        test::test_framework::{TestRunner, tests_failed},
        user_access::tests_user_access::TEST_USER_ACCESS,
    },
};

pub fn tee_test_unit() {
    let mut runner = TestRunner::new();
    // Here you would register and run your unit tests
    run_tests!(runner, [TEST_TEE_SESSION, TEST_USER_ACCESS,]);

    if tests_failed() {
        error!("!!! SOME TESTS FAILED, NEED TO BE FIXED !!!");
    } else {
        info!("!!! ALL TESTS PASSED !!!");
    }
}
