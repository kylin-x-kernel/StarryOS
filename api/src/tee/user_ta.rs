use core::default::Default;

#[repr(C)]
pub struct user_ta_ctx {}

impl Default for user_ta_ctx {
    fn default() -> Self {
        Self {
        }
    }
}