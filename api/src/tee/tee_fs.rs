// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use tee_raw_sys::TEE_OBJECT_ID_MAX_LEN;

#[repr(C)]
pub struct tee_fs_dirent {
    pub oid: [u8; TEE_OBJECT_ID_MAX_LEN as _],
    pub oid_len: u32,
}

// pub type TeeFileHandle = TeeFsFd;
