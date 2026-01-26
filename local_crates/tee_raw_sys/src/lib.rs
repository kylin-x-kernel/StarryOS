// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been modified by KylinSoft on 2025.
//

#![no_std]
#![allow(non_camel_case_types, non_snake_case)]

pub use tee_api_types::*;
pub use tee_api_defines::*;
pub use utee_types::*;

mod tee_api_types;
mod tee_api_defines;
mod utee_types;

/// Libc compatibility types
pub mod libc_compat {
    /// C size type
    pub type size_t = usize;
    /// Maximum-width integer type
    pub type intmax_t = i64;
}
