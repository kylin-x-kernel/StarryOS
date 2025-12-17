// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

// for source:
// 	- core/include/crypto/crypto.h

use crate::tee::{
    TeeResult,
    crypto::crypto::{ecc_keypair, ecc_public_key},
};

pub trait crypto_ecc_keypair_ops {
    fn generate(&mut self, key_size_bits: usize) -> TeeResult<()>;
    fn sign(&mut self, algo: u32, msg: &[u8], sig: &mut [u8], sig_len: &mut usize)
    -> TeeResult<()>;
    fn shared_secret(
        &mut self,
        public_key: &mut ecc_public_key,
        secret: &mut [u8],
        secret_len: &mut usize,
    ) -> TeeResult<()>;
    fn decrypt(&mut self, src: &[u8], dst: &mut [u8], dst_len: &mut usize) -> TeeResult<()>;
}
