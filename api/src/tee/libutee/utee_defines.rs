use tee_raw_sys::*;

#[cfg(feature = "tee")]
use crate::tee::{
    TEE_ALG_DES3_CMAC, TEE_ALG_RSAES_PKCS1_OAEP_MGF1_MD5, TEE_ALG_RSASSA_PKCS1_PSS_MGF1_MD5,
    TEE_ALG_RSASSA_PKCS1_V1_5, TEE_ALG_SHAKE128, TEE_ALG_SHAKE256, TEE_ALG_SM4_XTS, TEE_ALG_X448,
};

pub const TEE_CHAIN_MODE_ECB_NOPAD: u32 = 0x0;
pub const TEE_CHAIN_MODE_CBC_NOPAD: u32 = 0x1;
pub const TEE_CHAIN_MODE_CTR: u32 = 0x2;
pub const TEE_CHAIN_MODE_CTS: u32 = 0x3;
pub const TEE_CHAIN_MODE_XTS: u32 = 0x4;
pub const TEE_CHAIN_MODE_CBC_MAC_PKCS5: u32 = 0x5;
pub const TEE_CHAIN_MODE_CMAC: u32 = 0x6;
pub const TEE_CHAIN_MODE_CCM: u32 = 0x7;
pub const TEE_CHAIN_MODE_GCM: u32 = 0x8;
pub const TEE_CHAIN_MODE_PKCS1_PSS_MGF1: u32 = 0x9; /* ??? */

pub(crate) fn tee_u32_to_big_endian(x: u32) -> u32 {
    x.to_be()
}

pub(crate) fn tee_u32_from_big_endian(x: u32) -> u32 {
    u32::from_be(x)
}

/// Gets the class of a given algorithm
pub(crate) fn tee_alg_get_class(algo: u32) -> u32 {
    if algo == TEE_ALG_SM2_PKE {
        return TEE_OPERATION_ASYMMETRIC_CIPHER;
    }
    if algo == TEE_ALG_SM2_KEP {
        return TEE_OPERATION_KEY_DERIVATION;
    }
    if algo == TEE_ALG_RSASSA_PKCS1_V1_5 {
        return TEE_OPERATION_ASYMMETRIC_SIGNATURE;
    }
    if algo == TEE_ALG_DES3_CMAC {
        return TEE_OPERATION_MAC;
    }
    if algo == TEE_ALG_SM4_XTS {
        return TEE_OPERATION_CIPHER;
    }
    if algo == TEE_ALG_RSASSA_PKCS1_PSS_MGF1_MD5 {
        return TEE_OPERATION_ASYMMETRIC_SIGNATURE;
    }
    if algo == TEE_ALG_RSAES_PKCS1_OAEP_MGF1_MD5 {
        return TEE_OPERATION_ASYMMETRIC_CIPHER;
    }

    // Extract bits [31:28]
    (algo >> 28) & 0xF
}

pub(crate) fn tee_alg_get_main_alg(algo: u32) -> u32 {
    match algo {
        TEE_ALG_SM2_PKE => TEE_MAIN_ALGO_SM2_PKE,
        TEE_ALG_SM2_KEP => TEE_MAIN_ALGO_SM2_KEP,
        TEE_ALG_X25519 => TEE_MAIN_ALGO_X25519,
        TEE_ALG_ED25519 => TEE_MAIN_ALGO_ED25519,
        TEE_ALG_ECDSA_SHA1 | TEE_ALG_ECDSA_SHA224 | TEE_ALG_ECDSA_SHA256 | TEE_ALG_ECDSA_SHA384
        | TEE_ALG_ECDSA_SHA512 => TEE_MAIN_ALGO_ECDSA,
        TEE_ALG_HKDF => TEE_MAIN_ALGO_HKDF,
        TEE_ALG_SHAKE128 => TEE_MAIN_ALGO_SHAKE128,
        TEE_ALG_SHAKE256 => TEE_MAIN_ALGO_SHAKE256,
        TEE_ALG_X448 => TEE_MAIN_ALGO_X448,
        _ => algo & 0xff,
    }
}

pub(crate) fn tee_alg_get_chain_mode(algo: u32) -> u32 {
    ((algo) >> 8) & 0xF
}
