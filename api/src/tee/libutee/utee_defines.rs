use tee_raw_sys::*;

#[cfg(feature = "tee")]
use crate::tee::{
    TEE_ALG_DES3_CMAC, TEE_ALG_RSAES_PKCS1_OAEP_MGF1_MD5, TEE_ALG_RSASSA_PKCS1_PSS_MGF1_MD5,
    TEE_ALG_RSASSA_PKCS1_V1_5, TEE_ALG_SM4_XTS, TEE_ALG_SHAKE128, TEE_ALG_SHAKE256, TEE_ALG_X448
};

pub(crate) fn tee_u32_to_big_endian(x: u32) -> u32 {
    x.to_be()
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
