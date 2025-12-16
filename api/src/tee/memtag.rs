use super::types_ext::*;
use core::ffi::c_void;

#[inline]
pub fn memtag_strip_tag_vaddr(addr: *const c_void) -> vaddr_t {
   addr as vaddr_t
}
