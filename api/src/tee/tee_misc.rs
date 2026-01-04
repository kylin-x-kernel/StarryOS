use hex::{encode_to_slice, decode_to_slice};

#[allow(unused)]
pub const fn tee_b2hs_hsbuf_size(x: usize) -> usize {
    x.saturating_mul(2).saturating_add(1)
}

#[allow(unused)]
pub const fn tee_hs2b_bbuf_size(x: usize) -> usize {
    // saturating_add 避免 x = usize::MAX 时溢出
    x.saturating_add(1) >> 1
}

/// 将二进制数据 `b` 编码为十六进制字符串（写入 `hs`）
///
/// 返回写入的长度（不包含末尾 0）
pub fn tee_b2hs(b: &[u8], hs: &mut [u8]) -> Result<usize, ()> {
    let expected_len = b.len() * 2;

    if hs.len() < expected_len + 1 {
        return Err(()); // 模拟 TEE_ERROR_SHORT_BUFFER
    }

    encode_to_slice(b, &mut hs[..expected_len]).map_err(|_| ())?;

    hs.iter_mut()
        .take(expected_len)
        .for_each(|b| {
            if b'a' <= *b && *b <= b'z' {
                *b = *b - b'a' + b'A';
            }
        });

    hs[expected_len] = 0; // 结尾补 0，用于 C 兼容
    Ok(expected_len)
}

/// 将十六进制字符串 `hs` 解码为二进制（写入 `b`）
///
/// 返回写入的字节数
pub fn tee_hs2b(hs: &[u8], b: &mut [u8]) -> Result<usize, ()> {
    let hslen = hs.len();
    if hslen % 2 != 0 {
        return Err(()); // 长度必须是偶数
    }

    let expected_len = hslen / 2;
    if b.len() < expected_len {
        return Err(());
    }

    decode_to_slice(hs, &mut b[..expected_len]).map_err(|_| ())?;
    Ok(expected_len)
}