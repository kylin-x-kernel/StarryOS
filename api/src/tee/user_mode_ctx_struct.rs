use core::default::Default;

#[repr(C)]
pub struct user_mode_ctx {}

impl Default for user_mode_ctx {
    fn default() -> Self {
        Self {
        }
    }
}