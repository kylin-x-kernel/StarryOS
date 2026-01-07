#[macro_export]
macro_rules! tee_debug {
    ($($arg:tt)*) => {
        info!($($arg)*);
    };
}
