#![allow(dead_code)]
//! X Core Unit Test Framework
//!
//! 这个模块实现了一个自定义的单元测试框架，用于nvhe Rust代码的测试。
//! 框架支持手动注册测试用例，并提供基础的断言功能。
extern crate alloc;

use super::test_framework_basic::TestResult;
use alloc::format;
use core::fmt::Write;
// 测试结果枚举

impl TestResult {
    pub fn is_ok(&self) -> bool {
        matches!(self, TestResult::Ok)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, TestResult::Failed)
    }
}

// 测试统计信息
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

// 测试用例trait
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

// 测试描述符结构
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

        // 执行测试函数
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

// 简单的字符串写入器，用于格式化输出
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

// 测试运行器
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

        // 打印测试开始信息
        write!(self.output, "  Running test: {}", test.name()).ok();
        self.print_message(self.output.as_str());

        // 运行测试
        let result = test.run();

        // 打印测试结果
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

        // 更新统计信息
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

        // 打印最终统计信息
        self.print_final_stats();
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
            self.print_message("  >>> Some tests FAILED!");
        } else {
            self.print_message("  >>> All tests PASSED!");
        }
    }

    fn print_message(&self, msg: &str) {
        info!("{}", msg);
    }

    pub fn get_stats(&self) -> TestStats {
        self.stats
    }
}

// 基础断言宏
#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr) => {
        if $left != $right {
            // 输出调用时的表达式文本和实际值
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

// 用于定义测试函数的宏，如果测试通过则返回 TestResult::Ok
// #[macro_export]
// macro_rules! test_fn {
//     (pub fn $name:ident() $body:block) => {
//         pub fn $name() -> $crate::TestResult {
//             $body
//             $crate::TestResult::Ok
//         }
//     };
//     (fn $name:ident() $body:block) => {
//         fn $name() -> $crate::TestResult {
//             $body
//             $crate::TestResult::Ok
//         }
//     };
// }

// 用于手动注册测试用例的宏
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
    // 多个测试集
    ($runner:expr, [$($tests:expr),+ $(,)?]) => {
        $(
            $runner.run_tests_descriptors(stringify!($tests), $tests);
        )+
    };
    // 单个测试集
    ($runner:expr, $test:expr) => {
        $runner.run_tests_descriptors(stringify!($test), $test);
    };
}
