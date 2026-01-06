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

// the number of non-signed 32-bit integers left

// # parameter
// The 32 bit unsigned integer of the shift operation
//  * shift' -left shift number

//  # return value
// The 32 bit unsigned integer after
// shift

//  note
// This is an inline function for optimizing performance
#[inline]
pub(crate) fn shift_u32(v: u32, shift: u32) -> u32 {
    v << shift
}

// the number of non-signed 64-bit integers left

// # parameter
// The 64 bit unsigned integer of the shift operation
//  * shift' -left shift number

//  # return value
// The 64 bit unsigned integer after
// shift

//  note
// This is an inline function for optimizing performance
#[inline]
pub(crate) fn shift_u64(v: u64, shift: u32) -> u64 {
    v << shift
}

#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $type:ty, $member:ident) => {{
        let ptr = $ptr as *const _;
        (ptr as usize - core::mem::offset_of!($type, $member)) as *mut $type
    }};
}

#[macro_export]
macro_rules! member_size {
    ($type:ty, $member:ident) => {
        core::mem::offset_of!($type, $member)
    };
}

pub fn roundup_u<
    T: Copy
        + core::ops::Add<Output = T>
        + core::ops::Sub<Output = T>
        + core::ops::BitAnd<Output = T>
        + core::ops::Not<Output = T>
        + From<u8>,
>(
    v: T,
    size: T,
) -> T {
    (v + (size - T::from(1))) & !(size - T::from(1))
}

#[cfg(feature = "tee_test")]
pub mod tests_utils {
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{TestDescriptor, TestResult, bitstring::bit_ffc},
        test_fn, tests, tests_name,
    };

    test_fn! {
        using TestResult;

        fn test_bit_ffc() {
            let mut val: isize;

            // case 1: 全 0 => 第 0 位清除
            let bits = [0x00];
            val = -2;
            bit_ffc(&bits, 8, &mut val);
            assert_eq!(val, 0);

            // case 2: 00000001b => 第 1 位清除
            let bits = [0x01];
            val = -2;
            bit_ffc(&bits, 8, &mut val);
            assert_eq!(val, 1);

            // case 3: 11111111b => 全部 1 => 没有清除位
            let bits = [0xff];
            val = -2;
            bit_ffc(&bits, 8, &mut val);
            assert_eq!(val, -1);

            // case 4: 跨字节查找
            // byte0 = 0xff (全 1), byte1 = 0b11110111 (bit[11]=0)
            let bits = [0xff, 0b11110111];
            val = -2;
            bit_ffc(&bits, 16, &mut val);
            assert_eq!(val, 11);

            // case 5: 越界限制 — nbits < 实际位数
            let bits = [0x7f]; // 01111111 => bit[7]=0
            val = -2;
            bit_ffc(&bits, 7, &mut val); // 只检查前 7 位（0..6）
            assert_eq!(val, -1);
        }
    }

    tests_name! {
        TEST_TEE_UTILS;
        //------------------------
    }
}
