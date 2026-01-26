// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2023 The arm-gic Authors.
// Copyright (C) 2025 Weikang Guo <guoweikang.kernel@gmail.com>
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.
// 
// This file has been modified by KylinSoft on 2025.

/// Reads and returns the value of the given aarch64 system register.
macro_rules! read_sysreg {
    ($name:ident) => {
        {
            let mut value: u64;
            ::core::arch::asm!(
                concat!("mrs {value:x}, ", ::core::stringify!($name)),
                value = out(reg) value,
                options(nomem, nostack),
            );
            value
        }
    }
}
pub(crate) use read_sysreg;

/// Writes the given value to the given aarch64 system register.
macro_rules! write_sysreg {
    ($name:ident, $value:expr) => {
        {
            let v: u64 = $value;
            ::core::arch::asm!(
                concat!("msr ", ::core::stringify!($name), ", {value:x}"),
                value = in(reg) v,
                options(nomem, nostack),
            )
        }
    }
}
pub(crate) use write_sysreg;
