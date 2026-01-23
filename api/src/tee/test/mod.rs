#[cfg(feature = "tee_test")]
use crate::tee::TeeResult;

pub mod test_examples;
pub mod test_framework;
pub mod test_framework_basic;

#[cfg(feature = "tee_test")]
pub(crate) fn sys_tee_scn_test() -> TeeResult {
    use crate::tee::{tee_unit_test::tee_test_unit, test::test_examples::tee_test_example};

    tee_test_example();
    tee_test_unit();

    Ok(())
}
