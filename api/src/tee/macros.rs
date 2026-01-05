#[macro_export]
macro_rules! tee_debug {
    ($($arg:tt)*) => {
        debug!($($arg)*);
    };
}
