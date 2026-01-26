#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestResult {
    Ok,
    Failed,
    Ignored,
}

#[macro_export]
macro_rules! test_fn {
    (
        using $result:path;
        $(#[$attr:meta])*
        pub fn $name:ident() $body:block
    ) => {
        pub fn $name() -> $result {
            $body
            <$result>::Ok
        }
    };

    (
        using $result:path;
        $(#[$attr:meta])*
        fn $name:ident() $body:block
    ) => {
        fn $name() -> $result {
            $body
            <$result>::Ok
        }
    };
}
