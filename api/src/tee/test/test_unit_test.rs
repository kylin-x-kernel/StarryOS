use crate::{
    run_tests,
    tee::{
        tee_session::tests_tee_session::TEST_TEE_SESSION,
        test::test_framework::{TestRunner, tests_failed},
    },
};

pub fn tee_test_unit() {
    let mut runner = TestRunner::new();
    // Here you would register and run your unit tests
    run_tests!(runner, [TEST_TEE_SESSION,]);

    if tests_failed() {
        error!("!!! SOME TESTS FAILED, NEED TO BE FIXED !!!");
    } else {
        info!("!!! ALL TESTS PASSED !!!");
    }
}
