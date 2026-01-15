#![allow(dead_code)]

use super::{test_framework::*, test_framework_basic::*};
use crate::{assert, assert_eq, assert_ne, run_tests, test_fn, tests};

// Example test function 1: Simple addition test using assert_eq!
test_fn! {
    using TestResult;

    fn test_add_two_numbers() {
        let a = 5;
        let b = 3;
        assert_eq!(a + b, 8);
    }
}

// Example test function 2: String comparison test using assert_ne!
test_fn! {
    using TestResult;

    fn test_string_compare() {
        let s1 = "hello";
        let s2 = "world";
        assert_ne!(s1, s2);
    }
}

// Example test function 3: Boolean condition test using assert!
test_fn! {
    using TestResult;

    fn test_boolean_condition() {
        let x = 10;
        assert!(x > 5);
    }
}

// Example test function 4: A test case that should fail
test_fn! {
    using TestResult;

    fn test_should_fail() {
        let a = 1;
        let b = 2;
        assert_ne!(a, b, "Assertion failed because 1 is not equal to 2"); // This assertion will fail
    }
}

// Example test function 5: Another test case that should fail
test_fn! {
    using TestResult;

    fn test_another_failure() {
        let condition = true;
        assert!(condition, "Condition is false, test failed");
    }
}

// Manually register all test cases using the tests! macro
tests! {
    test_add_two_numbers,
    test_string_compare,
    test_boolean_condition,
    test_should_fail,
    test_another_failure,
}

// This is a simulated main function to demonstrate how to run tests
pub fn tee_test_example() {
    let mut runner = TestRunner::new();
    run_tests!(runner, TEST_SUITE);
    let stats = runner.get_stats();

    // Print final statistics
    info!(
        "Final Test Stats: total={}, passed={}, failed={}, ignored={}",
        stats.total, stats.passed, stats.failed, stats.ignored
    );
}
