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

pub fn roundup_u<T: Copy
    + core::ops::Add<Output = T>
    + core::ops::Sub<Output = T>
    + core::ops::BitAnd<Output = T>
    + core::ops::Not<Output = T>
    + From<u8>>(v: T, size: T) -> T 
{
    (v + (size - T::from(1))) & !(size - T::from(1))
}