// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use super::types_ext::*;
use core::ffi::c_void;

#[inline]
pub fn memtag_strip_tag_vaddr(addr: *const c_void) -> vaddr_t {
    addr as vaddr_t
}
