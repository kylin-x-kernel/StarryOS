// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use core::ffi::c_void;

use super:: {
    types_ext::*,
    utils::{shift_u64,shift_u32},
};


// Memory tagging configuration constants
#[cfg(all(feature = "cfg_memtag", target_arch = "aarch64"))]
const MEMTAG_IS_ENABLED: bool = true;

// Memory tagging configuration constants
#[cfg(not(all(feature = "cfg_memtag", target_arch = "aarch64")))]
const MEMTAG_IS_ENABLED: bool = false;

// tag shift
#[cfg(all(feature = "cfg_memtag", target_arch = "aarch64"))]
const MEMTAG_TAG_SHIFT: u32 = 56;

// tag width
#[cfg(all(feature = "cfg_memtag", target_arch = "aarch64"))]
const MEMTAG_TAG_WIDTH: u32 = 4;

// tag mask
#[cfg(all(feature = "cfg_memtag", target_arch = "aarch64"))]
const MEMTAG_TAG_MASK: u64 = (1u64 << MEMTAG_TAG_WIDTH) - 1;

// granule size
#[cfg(all(feature = "cfg_memtag", target_arch = "aarch64"))]
const MEMTAG_GRANULE_SIZE: usize = 16;

// granule size
#[cfg(not(all(feature = "cfg_memtag", target_arch = "aarch64")))]
const MEMTAG_GRANULE_SIZE: usize = 1;

// granule mask
const MEMTAG_GRANULE_MASK: usize = MEMTAG_GRANULE_SIZE - 1;

#[inline]
pub fn memtag_strip_tag_vaddr(addr: *const c_void) -> vaddr_t {
    addr as vaddr_t
}

// Strip memory tag from pointer
// (for tagged memory architectures)
// arg: addr: vaddr_t
// return: vaddr_t
#[inline]
fn memtag_strip_tag_vaddr_1(addr: vaddr_t) -> vaddr_t {
    #[cfg(all(feature = "cfg_memtag", target_arch = "aarch64"))]
    {
        // clear tag
        addr & !shift_u64(MEMTAG_TAG_MASK as usize, MEMTAG_TAG_SHIFT)
    }

    #[cfg(not(all(feature = "cfg_memtag", target_arch = "aarch64")))]
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

