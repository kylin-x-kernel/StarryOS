// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use super::{crypto_temp::{CryptoHashOps}};
use crate::tee::{TeeResult, utee_defines::TEE_MAX_HASH_SIZE, common::array};
use tee_raw_sys::{TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_BAD_STATE};
//--------------------from rust-mbedtls bindings.rs --------------------
#[allow(non_camel_case_types)]
pub struct md_info_t {}
#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub struct md_context_t<'a> {
    md_info: &'a md_info_t,
}
#[allow(dead_code)]
pub fn md_starts(_ctx: *mut md_context_t) -> i32 {
    0
}
#[allow(dead_code)]
pub fn md_update(
    _ctx: *mut md_context_t,
    _input: *const u8,
    _ilen: usize,
) -> u32 {
    0
}
#[allow(dead_code)]
pub fn md_get_size(_md_info: *const md_info_t) -> u8 {
    32 // 假设返回 SHA-256 的大小
}
#[allow(dead_code)]
pub fn md_finish(_ctx: *mut md_context_t, _output: *mut  u8) ->  i32 {
    0
}
#[allow(dead_code)]
pub fn md_free(_ctx: *mut md_context_t) {}
#[allow(dead_code)]
pub fn md_clone(_dst: *mut md_context_t, _src: *const md_context_t) ->  i32 {
    0
}

//-------------------- end rust-mbedtls bindings.rs --------------------
#[allow(dead_code)]
pub struct MbedHashCtx<'a> {
    md_context: md_context_t<'a>,
}

impl CryptoHashOps for MbedHashCtx<'_> {

    fn init(&mut self) -> TeeResult {
       if md_starts(&mut self.md_context) != 0 {
            Err(TEE_ERROR_BAD_STATE)
        } else {
            Ok(())
        }
    }

    fn update(&mut self, buf: Option<&[u8]>) -> TeeResult {
        let (buf_ptr, buf_len) = array::get_const_ptr_and_len(buf);

        if md_update(&mut self.md_context, buf_ptr, buf_len) != 0 {
            Err(TEE_ERROR_BAD_STATE)
        } else {
            Ok(())
        }
    }

    fn finalize(&mut self, digest: Option<&mut [u8]>) -> TeeResult {
        let hash_size = md_get_size(self.md_context.md_info) as usize;
        let mut block_digest = [0u8; TEE_MAX_HASH_SIZE]; // 内部的临时哈希缓冲区

        if hash_size > block_digest.len() {
            return Err(TEE_ERROR_BAD_STATE);
        }

        match digest {
            Some( user_buf) => {
                let target_ptr = if user_buf.len() >= hash_size {
                    user_buf.as_mut_ptr()
                } else {
                    block_digest.as_mut_ptr()
                };

                if md_finish(&mut self.md_context, target_ptr) != 0 {
                    return Err(TEE_ERROR_BAD_STATE);
                }

                if user_buf.len() < hash_size {
                    let copy_len = user_buf.len();
                    user_buf[..copy_len].copy_from_slice(&block_digest[..copy_len]);
                }
            }
            None => {
                return Err(TEE_ERROR_BAD_PARAMETERS);
            }
        }

        Ok(())
    }

    fn free_ctx(&mut self) {
        md_free(&mut self.md_context);
    }

    fn copy_state(&self, dst_ctx: &mut Self) {
        if md_clone(
            &mut dst_ctx.md_context,
            &self.md_context,
        ) != 0 {
            // TODO panic
        }
    }
}