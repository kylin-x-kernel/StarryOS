#![allow(dead_code)]

//! Tee Unit Test Framework
//!
//! This module implements a custom unit test framework for Rust code.
//! The framework supports manual test case registration and provides basic assertion functionality.

use alloc::format;
use core::{
    fmt::Write,
    sync::atomic::{AtomicBool, Ordering},
};

use super::test_framework_basic::TestResult;

impl TestResult {
    pub fn is_ok(&self) -> bool {
        matches!(self, TestResult::Ok)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, TestResult::Failed)
    }
}

// Test statistics
#[derive(Debug, Clone, Copy)]
pub struct TestStats {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
}

impl TestStats {
    pub const fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            ignored: 0,
        }
    }

    pub fn add_result(&mut self, result: TestResult) {
        self.total += 1;
        match result {
            TestResult::Ok => self.passed += 1,
            TestResult::Failed => self.failed += 1,
            TestResult::Ignored => self.ignored += 1,
        }
    }
}

pub static TEST_FAILED_FLAG: AtomicBool = AtomicBool::new(false);

// Testable trait
pub trait Testable {
    fn run(&self) -> TestResult;
    fn name(&self) -> &'static str;
    fn should_panic(&self) -> bool {
        false
    }
    fn ignore(&self) -> bool {
        false
    }
}

// Test descriptor structure
#[derive(Clone, Copy)]
#[repr(C)]
pub struct TestDescriptor {
    pub name: &'static str,
    pub test_fn: fn() -> TestResult,
    pub should_panic: bool,
    pub ignore: bool,
}

impl TestDescriptor {
    pub const fn new(
        name: &'static str,
        test_fn: fn() -> TestResult,
        should_panic: bool,
        ignore: bool,
    ) -> Self {
        Self {
            name,
            test_fn,
            should_panic,
            ignore,
        }
    }
}

impl Testable for TestDescriptor {
    fn run(&self) -> TestResult {
        if self.ignore {
            return TestResult::Ignored;
        }

        // Execute the test function
        (self.test_fn)()
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn should_panic(&self) -> bool {
        self.should_panic
    }

    fn ignore(&self) -> bool {
        self.ignore
    }
}

// Simple string writer for formatted output
pub struct StringWriter {
    buffer: [u8; 256],
    pos: usize,
}

impl StringWriter {
    pub const fn new() -> Self {
        Self {
            buffer: [0; 256],
            pos: 0,
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buffer[..self.pos]).unwrap_or("")
    }

    pub fn clear(&mut self) {
        self.pos = 0;
    }
}

impl Write for StringWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buffer.len() - self.pos;
        let to_copy = core::cmp::min(bytes.len(), remaining);

        if to_copy > 0 {
            self.buffer[self.pos..self.pos + to_copy].copy_from_slice(&bytes[..to_copy]);
            self.pos += to_copy;
        }

        Ok(())
    }
}

// Test runner
pub struct TestRunner {
    stats: TestStats,
    output: StringWriter,
}

impl TestRunner {
    pub const fn new() -> Self {
        Self {
            stats: TestStats::new(),
            output: StringWriter::new(),
        }
    }

    pub fn run_test(&mut self, test: &TestDescriptor) -> TestResult {
        self.output.clear();

        // Print test start information
        write!(self.output, "  Running test: {}", test.name()).ok();
        self.print_message(self.output.as_str());

        // Run the test
        let result = test.run();

        // Print test result
        self.output.clear();
        match result {
            TestResult::Ok => {
                write!(self.output, "    Test {} ... OK", test.name()).ok();
            }
            TestResult::Failed => {
                write!(self.output, "    Test {} ... FAILED", test.name()).ok();
            }
            TestResult::Ignored => {
                write!(self.output, "    Test {} ... IGNORED", test.name()).ok();
            }
        }
        self.print_message(self.output.as_str());

        // Update statistics
        self.stats.add_result(result);

        result
    }

    pub fn run_tests_descriptors(&mut self, name: &str, tests: &[TestDescriptor]) {
        self.stats = TestStats::new();

        self.print_message("--------------------------------");
        self.print_message(format!("Starting unit tests [{}]...", name).as_str());

        for test in tests {
            self.run_test(test);
        }

        // Print final statistics
        self.print_final_stats();

        // Set global flag if any test failed
        if self.stats.failed > 0 {
            TEST_FAILED_FLAG.store(true, Ordering::Relaxed);
        }
    }

    pub fn print_final_stats(&mut self) {
        self.output.clear();
        write!(
            self.output,
            "  >>> Test results: {} passed, {} failed, {} ignored, {} total",
            self.stats.passed, self.stats.failed, self.stats.ignored, self.stats.total
        )
        .ok();
        self.print_message(self.output.as_str());

        if self.stats.failed > 0 {
            self.print_error("  >>> This tests FAILED!");
        } else {
            self.print_message("  >>> This tests PASSED!");
        }
    }

    fn print_message(&self, msg: &str) {
        info!("{}", msg);
    }

    fn print_error(&self, msg: &str) {
        error!("{}", msg);
    }

    pub fn get_stats(&self) -> TestStats {
        self.stats
    }
}

// Basic assertion macros
#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr) => {
        if $left != $right {
            // Output the expression text and actual values at call time
            error!(
                "assert_eq! failed: {} ({:x?}) == {} ({:x?})",
                stringify!($left),
                $left,
                stringify!($right),
                $right
            );
            return TestResult::Failed;
        }
    };
    ($left:expr, $right:expr, $($arg:tt)*) => {
        if $left != $right {
            error!(
                "assert_eq! failed: {} ({:x?}) == {} ({:x?})",
                stringify!($left),
                $left,
                stringify!($right),
                $right
            );
            return TestResult::Failed;
        }
    };
}

#[macro_export]
macro_rules! assert_ne {
    ($left:expr, $right:expr) => {
        if $left == $right {
            error!(
                "assert_ne! failed: {} ({:x?}) == {} ({:x?})",
                stringify!($left),
                $left,
                stringify!($right),
                $right
            );
            return TestResult::Failed;
        }
    };
    ($left:expr, $right:expr, $($arg:tt)*) => {
        if $left == $right {
            error!(
                "assert_ne! failed: {} ({:x?}) == {} ({:x?})",
                stringify!($left),
                $left,
                stringify!($right),
                $right
            );
            return TestResult::Failed;
        }
    };
}

#[macro_export]
macro_rules! assert {
    ($cond:expr) => {
        if !$cond {
            error!("assert! failed: {}", stringify!($cond));
            return TestResult::Failed;
        }
    };
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            error!("assert! failed: {}", stringify!($cond));
            return TestResult::Failed;
        }
    };
}

// Macros for manually registering test cases
#[macro_export]
macro_rules! tests {
    ($($test_name:ident,)*) => {
        pub static TEST_SUITE: &[TestDescriptor] = &[
            $(
                TestDescriptor::new(
                    stringify!($test_name),
                    $test_name,
                    false, // should_panic
                    false, // ignore
                ),
            )*
        ];
    };
}

#[macro_export]
macro_rules! tests_name {
    ($suite_name:ident; $($test_name:ident),* $(,)?) => {
        pub static $suite_name: &[TestDescriptor] = &[
            $(
                TestDescriptor::new(
                    stringify!($test_name),
                    $test_name,
                    false, // should_panic
                    false, // ignore
                ),
            )*
        ];
    };
}

#[macro_export]
macro_rules! run_tests {
    // Multiple test suites
    ($runner:expr, [$($tests:expr),+ $(,)?]) => {
        $(
            $runner.run_tests_descriptors(stringify!($tests), $tests);
        )+
    };
    // Single test suite
    ($runner:expr, $test:expr) => {
        $runner.run_tests_descriptors(stringify!($test), $test);
    };
}

pub fn tests_failed() -> bool {
    TEST_FAILED_FLAG.load(Ordering::Relaxed)
}
