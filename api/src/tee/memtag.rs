// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use core::ffi::c_void;

use cfg_if::cfg_if;

use super::{
    types_ext::*,
    utils::{shift_u32, shift_u64},
};

cfg_if::cfg_if! {
    if #[cfg(all(feature = "tee_cfg_memtag", target_arch = "aarch64"))] {
        const MEMTAG_IS_ENABLED: bool = true;
        const MEMTAG_TAG_SHIFT: u32 = 56;
        const MEMTAG_TAG_WIDTH: u32 = 4;
        const MEMTAG_TAG_MASK: u64 = (1u64 << MEMTAG_TAG_WIDTH) - 1;
        const MEMTAG_GRANULE_SIZE: usize = 16;
    } else {
        const MEMTAG_IS_ENABLED: bool = false;
        const MEMTAG_GRANULE_SIZE: usize = 1;
    }
}

// granule mask
const MEMTAG_GRANULE_MASK: usize = MEMTAG_GRANULE_SIZE - 1;

#[inline]
pub fn memtag_strip_tag_vaddr(addr: *const c_void) -> vaddr_t {
    addr as vaddr_t
}

// Strip memory tag from constant pointer
// for tagged memory architectures
// arg: addr: vaddr_t
// return: vaddr_t
#[inline]
pub(crate) fn memtag_strip_tag(addr: vaddr_t) -> vaddr_t {
    // In real implementation, this would strip architecture-specific memory tags
    memtag_strip_tag_vaddr_1(addr)
}

// Strip memory tag from pointer
// (for tagged memory architectures)
// arg: addr: vaddr_t
// return: vaddr_t
#[inline]
fn memtag_strip_tag_vaddr_1(addr: vaddr_t) -> vaddr_t {
    #[cfg(all(feature = "tee_cfg_memtag", target_arch = "aarch64"))]
    {
        // clear tag
        addr & !shift_u64(MEMTAG_TAG_MASK as usize, MEMTAG_TAG_SHIFT)
    }

    #[cfg(not(all(feature = "tee_cfg_memtag", target_arch = "aarch64")))]
    {
        // For now, return the address as-is
        addr
    }
}

// Strip memory tag from constant pointer
// for tagged memory architectures
// arg: addr: vaddr_t
// return: vaddr_t
#[inline]
pub(crate) fn memtag_strip_tag_const(addr: vaddr_t) -> vaddr_t {
    // In real implementation, this would strip architecture-specific memory tags
    memtag_strip_tag_vaddr_1(addr)
}
