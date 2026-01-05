use hex::{decode_to_slice, encode_to_slice};

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

    hs.iter_mut().take(expected_len).for_each(|b| {
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

#[cfg(feature = "tee_test")]
pub mod tests_tee_misc {
    //-------- test framework import --------
    //-------- local tests import --------
    use super::*;
    use crate::{
        assert, assert_eq, assert_ne,
        tee::{TestDescriptor, TestResult},
        test_fn, tests, tests_name,
    };

    test_fn! {
        using TestResult;

        fn test_b2hs_empty_input() {
            let b = &[];
            let mut hs = [0u8; 1]; // Need space for the null terminator
            let result = tee_b2hs(b, &mut hs);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
            assert_eq!(hs[0], 0); // Null terminator should be present
        }
    }

    test_fn! {
        using TestResult;

        fn test_b2hs_single_byte() {
            let b = &[0xAB];
            let mut hs = [0u8; 3]; // "AB" + null terminator
            let result = tee_b2hs(b, &mut hs);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 2);
            assert_eq!(str::from_utf8(&hs[..2]).unwrap(), "AB");
            assert_eq!(hs[2], 0);
        }
    }

    test_fn! {
        using TestResult;
        fn test_b2hs_multiple_bytes() {
            let b = &[0x12, 0x34, 0xCD, 0xEF];
            let mut hs = [0u8; 9]; // 4 bytes * 2 hex chars + null terminator
            let result = tee_b2hs(b, &mut hs);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 8);
            assert_eq!(str::from_utf8(&hs[..8]).unwrap(), "1234CDEF");
            assert_eq!(hs[8], 0);
        }
    }

    test_fn! {
        using TestResult;
        fn test_b2hs_short_buffer() {
            let b = &[0x12, 0x34]; // Needs 4 hex chars + 1 null = 5 bytes
            let mut hs = [0u8; 4]; // Too short
            let result = tee_b2hs(b, &mut hs);
            assert!(result.is_err());
        }
    }

    test_fn! {
        using TestResult;

        fn test_b2hs_exact_buffer_size() {
            let b = &[0xAA]; // Needs 2 hex chars + 1 null = 3 bytes
            let mut hs = [0u8; 3]; // Exact size
            let result = tee_b2hs(b, &mut hs);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 2);
            assert_eq!(str::from_utf8(&hs[..2]).unwrap(), "AA");
            assert_eq!(hs[2], 0);
        }
    }

    // --- Tests for tee_hs2b ---

    test_fn! {
        using TestResult;
        fn test_hs2b_empty_input() {
            let hs = &[];
            let mut b = [0u8; 0];
            let result = tee_hs2b(hs, &mut b);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        }
    }

    test_fn! {
        using TestResult;
        fn test_hs2b_single_byte_hex() {
            let hs = b"AB";
            let mut b = [0u8; 1];
            let result = tee_hs2b(hs, &mut b);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 1);
            assert_eq!(b[0], 0xAB);
        }
    }

    test_fn! {
        using TestResult;
        fn test_hs2b_multiple_bytes_hex() {
            let hs = b"1234cdef";
            let mut b = [0u8; 4];
            let result = tee_hs2b(hs, &mut b);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 4);
            assert_eq!(b, [0x12, 0x34, 0xCD, 0xEF]);
        }
    }

    test_fn! {
        using TestResult;
        fn test_hs2b_odd_length_hex() {
            let hs = b"123"; // Odd length
            let mut b = [0u8; 2];
            let result = tee_hs2b(hs, &mut b);
            assert!(result.is_err());
        }
    }

    test_fn! {
        using TestResult;
        fn test_hs2b_short_buffer() {
            let hs = b"1234"; // Needs 2 bytes output
            let mut b = [0u8; 1]; // Too short
            let result = tee_hs2b(hs, &mut b);
            assert!(result.is_err());
        }
    }

    test_fn! {
        using TestResult;
        fn test_hs2b_invalid_hex_chars() {
            let hs = b"12gx"; // 'g' is an invalid hex character
            let mut b = [0u8; 2];
            let result = tee_hs2b(hs, &mut b);
            assert!(result.is_err()); // `decode_to_slice` will return an error
        }
    }

    test_fn! {
        using TestResult;
        fn test_hs2b_uppercase_hex() {
            let hs = b"ABCDEF";
            let mut b = [0u8; 3];
            let result = tee_hs2b(hs, &mut b);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 3);
            assert_eq!(b, [0xAB, 0xCD, 0xEF]);
        }
    }

    test_fn! {
        using TestResult;
        fn test_hs2b_mixed_case_hex() {
            let hs = b"aBcDeF";
            let mut b = [0u8; 3];
            let result = tee_hs2b(hs, &mut b);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 3);
            assert_eq!(b, [0xAB, 0xCD, 0xEF]);
        }
    }

    tests_name! {
        TEST_TEE_MISC;
        //------------------------
        test_b2hs_empty_input,
        test_b2hs_single_byte,
        test_b2hs_multiple_bytes,
        test_b2hs_short_buffer,
        test_b2hs_exact_buffer_size,
        test_hs2b_empty_input,
        test_hs2b_single_byte_hex,
        test_hs2b_multiple_bytes_hex,
        test_hs2b_odd_length_hex,
        test_hs2b_short_buffer,
        test_hs2b_invalid_hex_chars,
        test_hs2b_uppercase_hex,
        test_hs2b_mixed_case_hex,
    }
}
