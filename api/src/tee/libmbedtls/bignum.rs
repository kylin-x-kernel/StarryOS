// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.
use alloc::vec;
use core::{
    cmp::Ordering,
    default, mem,
    ops::{Deref, DerefMut},
};
use mbedtls::bignum::Mpi;
use mbedtls_sys_auto::*;
use tee_raw_sys::*;

use crate::tee::TeeResult;
use crate::tee::config::CFG_CORE_BIGNUM_MAX_BITS;

/// BigNum 是新类型结构体，包装了 Mpi
#[derive(Debug, Clone)]
pub struct BigNum(Mpi);

impl BigNum {
    /// 创建一个新的 BigNum，值为 0
    pub fn new(value: u32) -> TeeResult<Self> {
        Mpi::new(value as _)
            .map(BigNum)
            .map_err(|_| TEE_ERROR_GENERIC)
    }

    /// 从 Mpi 创建 BigNum
    pub fn from_mpi(mpi: Mpi) -> Self {
        BigNum(mpi)
    }

    /// 获取内部的 Mpi
    pub fn into_mpi(self) -> Mpi {
        self.0
    }

    /// 获取内部 Mpi 的引用
    pub fn as_mpi(&self) -> &Mpi {
        &self.0
    }

    /// 获取内部 Mpi 的可变引用
    pub fn as_mpi_mut(&mut self) -> &mut Mpi {
        &mut self.0
    }
}

impl Deref for BigNum {
    type Target = Mpi;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BigNum {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl default::Default for BigNum {
    fn default() -> Self {
        // 注意：这不是 const 函数，因为 Mpi::new 不是 const
        BigNum::new(0).unwrap()
    }
}

impl From<Mpi> for BigNum {
    fn from(mpi: Mpi) -> Self {
        BigNum(mpi)
    }
}

impl From<BigNum> for Mpi {
    fn from(bn: BigNum) -> Self {
        bn.0
    }
}

impl PartialEq for BigNum {
    fn eq(&self, other: &Self) -> bool {
        // 使用 Mpi 的 cmp 方法进行比较
        self.cmp(other) == core::cmp::Ordering::Equal
    }
}

impl Eq for BigNum {}

const cil: usize = mem::size_of::<mpi_uint>();
const bil: usize = cil << 3;

fn bits_to_limbs(i: usize) -> usize {
    i / bil + if i % bil != 0 { 1 } else { 0 }
}

/// Get number of bytes required to store the big number
///     
/// a: BigNum
/// Returns: number of bytes
pub fn crypto_bignum_num_bytes(a: &BigNum) -> TeeResult<usize> {
    a.byte_length().map_err(|_| TEE_ERROR_GENERIC)
}

/// Get number of bits required to store the big number
///
/// a: BigNum
/// Returns: number of bits
pub fn crypto_bignum_num_bits(a: &BigNum) -> TeeResult<usize> {
    a.bit_length().map_err(|_| TEE_ERROR_GENERIC)
}

/// Compare two big numbers
///
/// a: BigNum
/// b: BigNum
/// Returns: Ordering
pub fn crypto_bignum_compare(a: &BigNum, b: &BigNum) -> Ordering {
    a.cmp(b)
}

/// Convert big number to binary representation
///
/// from: BigNum
/// to: mutable byte slice
/// Returns: TeeResult
pub fn crypto_bignum_bn2bin(from: &BigNum, to: &mut [u8]) -> TeeResult<()> {
    let a = from.to_binary().map_err(|_| TEE_ERROR_GENERIC)?;
    to[..a.len()].copy_from_slice(&a);
    Ok(())
}

/// Convert binary representation to big number
///
/// from: byte slice
/// to: mutable BigNum
/// Returns: TeeResult
pub fn crypto_bignum_bin2bn(from: &[u8], to: &mut BigNum) -> TeeResult<()> {
    *to = BigNum::from_mpi(Mpi::from_binary(from).map_err(|_| TEE_ERROR_BAD_PARAMETERS)?);
    Ok(())
}

/// Copy big number from `from` to `to`
///
/// FIXME: Add try_clone for Mpi in mbedtls-rs
pub fn crypto_bignum_copy(to: &mut BigNum, from: &BigNum) {
    to.clone_from(from);
}

/// Allocate a big number with specified size in bits
///
/// size_bits: usize
/// Returns: TeeResult<BigNum>
pub fn crypto_bignum_allocate(size_bits: usize) -> TeeResult<BigNum> {
    let mut size_bits = size_bits;
    if size_bits > CFG_CORE_BIGNUM_MAX_BITS {
        size_bits = CFG_CORE_BIGNUM_MAX_BITS;
    }
    let mut bn = BigNum::new(0).map_err(|_| TEE_ERROR_OUT_OF_MEMORY)?;

    // info!("bignum bits is {:?}", bits_to_limbs(size_bits));
    // Enlarge to the specified number of limbs
    bn.mpi_grow(bits_to_limbs(size_bits))
        .map_err(|_| TEE_ERROR_OUT_OF_MEMORY)?;

    Ok(bn)
}

/// Free a big number
///
/// bn: BigNum
pub fn crypto_bignum_free(bn: BigNum) {
    drop(bn);
}

/// Clear a big number (set to zero)
///
/// bn: mutable BigNum
pub fn crypto_bignum_clear(bn: &mut BigNum) {
    bn.clear();
}

#[cfg(feature = "tee_test")]
pub mod tests_tee_bignum {
    use zerocopy::IntoBytes;

    //-------- test framework import --------
    use crate::tee::TestDescriptor;
    use crate::tee::TestResult;
    use crate::test_fn;
    use crate::{assert, assert_eq, assert_ne, tests, tests_name};

    //-------- local tests import --------
    use super::*;

    test_fn! {
        using TestResult;

        fn test_tee_bignum() {
            // Test bits_to_limbs
            assert_eq!(bits_to_limbs(0), 0);
            assert_eq!(bits_to_limbs(1), 1);
            assert_eq!(bits_to_limbs(bil - 1), 1);
            assert_eq!(bits_to_limbs(bil), 1);
            assert_eq!(bits_to_limbs(bil + 1), 2);
            assert_eq!(bits_to_limbs(bil * 2), 2);
            // Test allocation
            let bn = crypto_bignum_allocate(1024).expect("Failed to allocate BigNum");
            // Test copy
            let mut bn_copy = crypto_bignum_allocate(1024).expect("Failed to allocate BigNum");
            crypto_bignum_copy(&mut bn_copy, &bn);
            assert_eq!(crypto_bignum_compare(&bn, &bn_copy), Ordering::Equal);
            // Test bin2bn with zero data
            let zero_data = vec![0u8; 1];
            let mut bn_zero = crypto_bignum_allocate(128).expect("Failed to allocate BigNum");
            crypto_bignum_bin2bn(&zero_data, &mut bn_zero).expect("Failed to convert bin to bn");
            // let num_bytes = crypto_bignum_num_bytes(&bn_zero).expect("Failed to get num bytes");
            assert_eq!(bn_zero.as_u32().unwrap(), 0);
            // let mut bin_out = vec![1u8; 1];
            // crypto_bignum_bn2bin(&bn_zero, &mut bin_out).expect("Failed to convert bn to bin");
            // assert_eq!(bin_out, zero_data);

            // Test compare
            // Test bin2bn and bn2bin
            let mut bn_from_bin = crypto_bignum_allocate(1024).expect("Failed to allocate BigNum");
            let bin_data = vec![0xF2, 0x34, 0x56, 0x78];
            let ret = crypto_bignum_bin2bn(&bin_data, &mut bn_from_bin).expect("Failed to convert bin to bn");
            assert_eq!(ret, ());
            let mut bin_out = vec![0u8; 4];
            crypto_bignum_bn2bin(&bn_from_bin, &mut bin_out).expect("Failed to convert bn to bin");
            assert_eq!(bin_data, bin_out);
            // Test num_bytes and num_bits
            let num_bytes = crypto_bignum_num_bytes(&bn_from_bin).expect("Failed to get num bytes");
            let num_bits = crypto_bignum_num_bits(&bn_from_bin).expect("Failed to get num bits");
            assert_eq!(num_bytes, 4);
            assert_eq!(num_bits, 32);
            // Test clear
            crypto_bignum_clear(&mut bn_from_bin);
            // Test bin with prefix zeros
            let mut bn_from_bin = crypto_bignum_allocate(1024).expect("Failed to allocate BigNum");
            let bin_data = vec![0x00, 0x00, 0x56, 0x78];
            let ret = crypto_bignum_bin2bn(&bin_data, &mut bn_from_bin).expect("Failed to convert bin to bn");
            assert_eq!(ret, ());
            let mut bin_out = vec![5u8; 4];
            crypto_bignum_bn2bin(&bn_from_bin, &mut bin_out).expect("Failed to convert bn to bin");
            assert_eq!(&[0x56, 0x78, 5, 5], bin_out.as_bytes());
            // Test free
            crypto_bignum_free(bn);
            crypto_bignum_free(bn_copy);
            crypto_bignum_free(bn_zero);
            crypto_bignum_free(bn_from_bin);
        }
    }

    tests_name! {
        TEST_TEE_BIGNUM;
        //------------------------
        test_tee_bignum,
    }
}
