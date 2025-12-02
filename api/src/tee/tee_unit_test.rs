use crate::tee::{TestDescriptor,TestRunner};
use crate::test_fn;
use crate::{assert, assert_eq, assert_ne, tests, tests_name, run_tests, };

pub fn tee_test_unit() {
    use super::tee_pobj::tests_tee_pobj::TEST_TEE_POBJ;
    let mut runner = TestRunner::new();
    run_tests!(runner, [TEST_TEE_POBJ,]);
}
