// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

#[inline]
pub const fn bit32(nr: u32) -> u32 {
    1u32 << nr
}

#[inline]
pub const fn bit64(nr: u32) -> u64 {
    1u64 << nr
}

#[inline]
pub const fn bit(nr: u32) -> u32 {
    bit32(nr)
}

#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $type:ty, $member:ident) => {
        {
            let ptr = $ptr as *const _;
            (ptr as usize - core::mem::offset_of!($type, $member)) as *mut $type
        }
    }
}

#[macro_export]
macro_rules! member_size {
    ($type:ty, $member:ident) => {
        core::mem::offset_of!($type, $member)
    }
}