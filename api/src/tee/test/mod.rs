use crate::tee::{
    TeeResult,
    test::{test_examples::tee_test_example, test_unit_test::tee_test_unit},
};

mod test_examples;
mod test_framework;
mod test_framework_basic;
mod test_unit_test;

// This is not a standard tee syscall, just a way to run tests in TEE mode.
pub fn sys_tee_scn_test() -> TeeResult {
    tee_test_example();
    tee_test_unit();
    Ok(())
}
