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

/// Get the memory tag value from an address
///
/// This function extracts the memory tag from a virtual address. Memory tags are a hardware-assisted security feature
/// used to detect memory access errors. The tag value is stored in the high bits of the address.
///
/// # Parameters
/// * `_addr` - The virtual address from which to retrieve the tag
///
/// # Return value
/// Returns the tag value (0-15) from the address. If memory tagging is not enabled or the architecture is not AArch64, it always returns 0.
///
/// # Conditional compilation
/// This function only extracts the actual tag value when both of the following conditions are met:
/// 1. The `tee_cfg_memtag` feature is enabled
/// 2. The target architecture is `aarch64`
///
/// # Safety
/// This function is a pure computation operation and does not access actual memory, making it a safe operation.
/// Even passing an invalid address will not cause a memory access exception.
#[cfg(not(feature = "tee_slab_crypt_state"))]
#[inline]
fn memtag_get_tag(
    _addr: vaddr_t
) -> u8
{
    #[cfg(all(feature = "tee_cfg_memtag", target_arch = "aarch64"))]
    {
        let va: usize = _addr;

        return ((va >> MEMTAG_TAG_SHIFT) & MEMTAG_TAG_MASK) as u8;
    }

    #[cfg(not(all(feature = "tee_cfg_memtag", target_arch = "aarch64")))]
    {
        return 0u8;
    }
}


#[cfg(feature = "tee_slab_crypt_state")]
#[inline]
fn memtag_get_tag(
    _addr: vaddr_t
) -> u8
{
    0
}

#[inline]
pub(crate) fn memtag_insert_tag_vaddr(
    addr: vaddr_t,
    _tag: u8
) -> vaddr_t {
    let va = memtag_strip_tag_vaddr_1(addr);
    #[cfg(all(feature = "tee_cfg_memtag", target_arch = "aarch64"))]
    {
        va |= shift_u64(_tag, MEMTAG_TAG_SHIFT);
    }

    return va;
}

#[cfg(not(feature = "tee_slab_crypt_state"))]
#[inline]
pub(crate) fn uref_to_vaddr (
    uref: u32
) -> vaddr_t
{
    unsafe extern "C" {
        // unsafe static __text_start: *const u8;
        unsafe static _stext: *const u8;
    }

    #[cfg(all(feature = "tee_cfg_memtag", target_arch = "aarch64"))]
    {
        let u = uref & (u32::MAX >> MEMTAG_TAG_WIDTH);
        let uref_tag_shift: u32 = 32 - MEMTAG_TAG_WIDTH;
        let tag: u8 = uref >> uref_tag_shift;
        return memtag_insert_tag_vaddr(VCORE_START_VA + u, tag);
    }

    let VCORE_START_VA: vaddr_t = unsafe {_stext as vaddr_t};

    return VCORE_START_VA + uref as vaddr_t;
}

// Convert kernel address to user reference
// new slab
#[cfg(feature = "tee_slab_crypt_state")]
#[inline]
pub(crate) fn kaddr_to_uref(
    kaddr: vaddr_t
) -> u32 {
    kaddr as _
}

#[cfg(feature = "tee_slab_crypt_state")]
#[inline]
pub(crate) fn uref_to_vaddr (
    uref: u32
) -> vaddr_t
{
    uref as _
}

#[cfg(not(feature = "tee_slab_crypt_state"))]
#[inline]
pub(crate) fn kaddr_to_uref(
    kaddr: vaddr_t
) -> u32 {
    unsafe extern "C" {
        static _stext: *const u8;
    }

    // Safety: Converting pointer to usize for arithmetic operations
    let addr = kaddr as usize;

    let VCORE_START_VA: vaddr_t = unsafe {_stext as usize as vaddr_t};

    #[cfg(all(feature = "tee_cfg_memtag", target_arch = "aarch64"))]
    {
        // Memory tagging is enabled: handle tag extraction and storage
        // Tag shift position: upper bits of 32-bit uref
        let uref_tag_shift = 32u32 - MEMTAG_TAG_WIDTH;

        // Strip memory tag to get base address
        let mut uref = memtag_strip_tag_vaddr_1(addr);

        // Convert to offset from core start address
        uref -= VCORE_START_VA as usize;

        // Assert that the offset fits in 32 bits minus tag bits
        assert!(uref < (u32::MAX >> MEMTAG_TAG_WIDTH) as usize,
                "Kernel address offset too large for uref");

        // Extract tag from original address and store in upper bits
        let tag = memtag_get_tag(addr);
        uref |= (tag as usize) << uref_tag_shift;

        // Convert to u32 (uref is always 32 bits)
        uref as u32
    }

    #[cfg(not(all(feature = "tee_cfg_memtag", target_arch = "aarch64")))]
    {
        // Memory tagging is disabled: simple offset conversion
        // Just subtract the virtual core base address
        let uref = addr - VCORE_START_VA as usize;

        // Assert the result fits in 32 bits
        assert!(uref < u32::MAX as usize,
                "Kernel address offset too large for uref");

        uref as u32
    }
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
