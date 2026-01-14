// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been modified by KylinSoft on 2025.

// RSA signatures with MD5 hash
// Values prefixed with vendor ID bit31 with by TEE bitfields IDs
pub const TEE_ALG_RSASSA_PKCS1_PSS_MGF1_MD5: u32 = 0xF0111930;
pub const TEE_ALG_RSAES_PKCS1_OAEP_MGF1_MD5: u32 = 0xF0110230;

// API extended result codes as per TEE_Result IDs defined in GPD TEE
// Internal Core API specification v1.1:
//
// 0x70000000 - 0x7FFFFFFF: Reserved for implementation-specific return
// 			    code providing non-error information
// 0x80000000 - 0x8FFFFFFF: Reserved for implementation-specific errors
//
// TEE_ERROR_DEFER_DRIVER_INIT - Device driver failed to initialize because
// the driver depends on a device not yet initialized.
pub const TEE_ERROR_DEFER_DRIVER_INIT: u32 = 0x80000000;

// TEE_ERROR_NODE_DISABLED - Device driver failed to initialize because it is
// not allocated for TEE environment.
pub const TEE_ERROR_NODE_DISABLED: u32 = 0x80000001;

// HMAC-based Extract-and-Expand Key Derivation Function (HKDF)

pub const TEE_ALG_HKDF_MD5_DERIVE_KEY: u32 = 0x800010C0;
pub const TEE_ALG_HKDF_SHA1_DERIVE_KEY: u32 = 0x800020C0;
pub const TEE_ALG_HKDF_SHA224_DERIVE_KEY: u32 = 0x800030C0;
pub const TEE_ALG_HKDF_SHA256_DERIVE_KEY: u32 = 0x800040C0;
pub const TEE_ALG_HKDF_SHA384_DERIVE_KEY: u32 = 0x800050C0;
pub const TEE_ALG_HKDF_SHA512_DERIVE_KEY: u32 = 0x800060C0;

pub const TEE_TYPE_HKDF_IKM: u32 = 0xA10000C0;

pub const TEE_ATTR_HKDF_IKM: u32 = 0xC00001C0;
// There is a name clash with the  official attributes TEE_ATTR_HKDF_SALT
// and TEE_ATTR_HKDF_INFO so define these alternative ID.
pub const __OPTEE_TEE_ATTR_HKDF_SALT: u32 = 0xD00002C0;
pub const __OPTEE_TEE_ATTR_HKDF_INFO: u32 = 0xD00003C0;
pub const TEE_ATTR_HKDF_OKM_LENGTH: u32 = 0xF00004C0;

// Concatenation Key Derivation Function (Concat KDF)
// NIST SP 800-56A section 5.8.1

pub const TEE_ALG_CONCAT_KDF_SHA1_DERIVE_KEY: u32 = 0x800020C1;
pub const TEE_ALG_CONCAT_KDF_SHA224_DERIVE_KEY: u32 = 0x800030C1;
pub const TEE_ALG_CONCAT_KDF_SHA256_DERIVE_KEY: u32 = 0x800040C1;
pub const TEE_ALG_CONCAT_KDF_SHA384_DERIVE_KEY: u32 = 0x800050C1;
pub const TEE_ALG_CONCAT_KDF_SHA512_DERIVE_KEY: u32 = 0x800060C1;

pub const TEE_TYPE_CONCAT_KDF_Z: u32 = 0xA10000C1;

pub const TEE_ATTR_CONCAT_KDF_Z: u32 = 0xC00001C1;
pub const TEE_ATTR_CONCAT_KDF_OTHER_INFO: u32 = 0xD00002C1;
pub const TEE_ATTR_CONCAT_KDF_DKM_LENGTH: u32 = 0xF00003C1;

// PKCS #5 v2.0 Key Derivation Function 2 (PBKDF2)
// RFC 2898 section 5.2
// https://www.ietf.org/rfc/rfc2898.txt

pub const TEE_ALG_PBKDF2_HMAC_SHA1_DERIVE_KEY: u32 = 0x800020C2;

pub const TEE_TYPE_PBKDF2_PASSWORD: u32 = 0xA10000C2;

pub const TEE_ATTR_PBKDF2_PASSWORD: u32 = 0xC00001C2;
pub const TEE_ATTR_PBKDF2_SALT: u32 = 0xD00002C2;
pub const TEE_ATTR_PBKDF2_ITERATION_COUNT: u32 = 0xF00003C2;
pub const TEE_ATTR_PBKDF2_DKM_LENGTH: u32 = 0xF00004C2;

// PKCS#1 v1.5 RSASSA pre-hashed sign/verify

pub const TEE_ALG_RSASSA_PKCS1_V1_5: u32 = 0xF0000830;

//  TDEA CMAC (NIST SP800-38B)
pub const TEE_ALG_DES3_CMAC: u32 = 0xF0000613;

//  SM4-XTS
pub const TEE_ALG_SM4_XTS: u32 = 0xF0000414;

// Implementation-specific object storage constants

// Storage is provided by the Rich Execution Environment (REE)
pub const TEE_STORAGE_PRIVATE_REE: u32 = 0x80000000;
// Storage is the Replay Protected Memory Block partition of an eMMC device
pub const TEE_STORAGE_PRIVATE_RPMB: u32 = 0x80000100;
// Was TEE_STORAGE_PRIVATE_SQL, which isn't supported any longer
pub const TEE_STORAGE_PRIVATE_SQL_RESERVED: u32 = 0x80000200;

// Extension of "Memory Access Rights Constants"
// #define TEE_MEMORY_ACCESS_READ             0x00000001
// #define TEE_MEMORY_ACCESS_WRITE            0x00000002
// #define TEE_MEMORY_ACCESS_ANY_OWNER        0x00000004
//
// TEE_MEMORY_ACCESS_NONSECURE : if set TEE_CheckMemoryAccessRights()
// successfully returns only if target vmem range is mapped non-secure.
//
// TEE_MEMORY_ACCESS_SECURE : if set TEE_CheckMemoryAccessRights()
// successfully returns only if target vmem range is mapped secure.
//
pub const TEE_MEMORY_ACCESS_NONSECURE: u32 = 0x10000000;
pub const TEE_MEMORY_ACCESS_SECURE: u32 = 0x20000000;

// Implementation-specific login types

// Private login method for REE kernel clients
pub const TEE_LOGIN_REE_KERNEL: u32 = 0x80000000;


// SHA3 224
pub(crate) const TEE_ALG_SHA3_224: u32 = 0x50000008;

// SHA3 256
pub(crate) const TEE_ALG_SHA3_256: u32 = 0x50000009;

// SHA3 384
pub(crate) const TEE_ALG_SHA3_384: u32 = 0x5000000A;

// SHA3 512
pub(crate) const TEE_ALG_SHA3_512: u32 = 0x5000000B;

// SHAKE128
pub(crate) const TEE_ALG_SHAKE128: u32 = 0x50000101;

// SHAKE256
pub(crate) const TEE_ALG_SHAKE256: u32 = 0x50000102;

/// Algorithm identifier constants for TEE (Trusted Execution Environment) API
/// These values follow the GlobalPlatform TEE Internal Core API specification
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub(crate) enum TEEAlgorithm {
    // OP-TEE specific algorithm identifiers due to specification inconsistency
    /// ECDSA P192 algorithm - OP-TEE specific value to conform to Table 6-12
    OpteeEcdsaP192 = 0x70001041,
    /// ECDSA P224 algorithm - OP-TEE specific value to conform to Table 6-12
    OpteeEcdsaP224 = 0x70002041,
    /// ECDSA P256 algorithm - OP-TEE specific value to conform to Table 6-12
    OpteeEcdsaP256 = 0x70003041,
    /// ECDSA P384 algorithm - OP-TEE specific value to conform to Table 6-12
    OpteeEcdsaP384 = 0x70004041,
    /// ECDSA P521 algorithm - OP-TEE specific value to conform to Table 6-12
    OpteeEcdsaP521 = 0x70005041,
    /// ECDH P192 algorithm - OP-TEE specific value to conform to Table 6-12
    OpteeEcdhP192 = 0x80001042,
    /// ECDH P224 algorithm - OP-TEE specific value to conform to Table 6-12
    OpteeEcdhP224 = 0x80002042,
    /// ECDH P256 algorithm - OP-TEE specific value to conform to Table 6-12
    OpteeEcdhP256 = 0x80003042,
    /// ECDH P384 algorithm - OP-TEE specific value to conform to Table 6-12
    OpteeEcdhP384 = 0x80004042,
    /// ECDH P521 algorithm - OP-TEE specific value to conform to Table 6-12
    OpteeEcdhP521 = 0x80005042,

    // Deprecated ECDSA and ECDH identifiers - replaced by SHA-specific variants
    /// ECDSA with SHA-1 hash algorithm
    EcdsaSha1 = 0x70001042,
    /// ECDSA with SHA-224 hash algorithm
    EcdsaSha224 = 0x70002042,
    /// ECDSA with SHA-256 hash algorithm
    EcdsaSha256 = 0x70003042,
    /// ECDSA with SHA-384 hash algorithm
    EcdsaSha384 = 0x70004042,
    /// ECDSA with SHA-512 hash algorithm
    EcdsaSha512 = 0x70005042,
    /// ECDSA with SHA3-224 hash algorithm
    EcdsaSha3_224 = 0x70006042,
    /// ECDSA with SHA3-256 hash algorithm
    EcdsaSha3_256 = 0x70007042,
    /// ECDSA with SHA3-384 hash algorithm
    EcdsaSha3_384 = 0x70008042,
    /// ECDSA with SHA3-512 hash algorithm
    EcdsaSha3_512 = 0x70009042,

    // Elliptic curve algorithms
    /// ECDH key derivation algorithm
    EcdhDeriveSharedSecret = 0x80000042,

    // EdDSA algorithms
    /// Ed25519 signature algorithm
    Ed25519 = 0x70006043,
    /// Ed448 signature algorithm
    Ed448 = 0x70006044,

    // SM algorithms
    /// SM2 public key encryption algorithm
    Sm2Pke = 0x80000046,
    /// HKDF key derivation function
    Hkdf = 0x80000047,
    /// SM3 hash algorithm
    Sm3 = 0x50000007,
    /// X25519 key exchange algorithm
    X25519 = 0x80000044,
    /// X448 key exchange algorithm
    X448 = 0x80000045,

    // SM4 algorithms with PKCS5 padding
    /// SM4 in ECB mode with PKCS5 padding
    Sm4EcbPkcs5 = 0x10000015,
    /// SM4 in CBC mode with PKCS5 padding
    Sm4CbcPkcs5 = 0x10000115,

    // Hash algorithms
    /// SHA3-224 hash algorithm
    Sha3_224 = 0x50000008,
    /// SHA3-256 hash algorithm
    Sha3_256 = 0x50000009,
    /// SHA3-384 hash algorithm
    Sha3_384 = 0x5000000A,
    /// SHA3-512 hash algorithm
    Sha3_512 = 0x5000000B,
    /// SHAKE128 extendable output function
    Shake128 = 0x50000101,
    /// SHAKE256 extendable output function
    Shake256 = 0x50000102,

    // Illegal value marker
    /// Marker for illegal/invalid algorithm values
    IllegalValue = 0xEFFFFFFF,
}

impl From<u32> for TEEAlgorithm {
    /// Convert a u32 value to TEEAlgorithm
    /// Note: This conversion is partial and may not cover all possible u32 values
    fn from(value: u32) -> Self {
        match value {
            0x70001041 => TEEAlgorithm::OpteeEcdsaP192,
            0x70002041 => TEEAlgorithm::OpteeEcdsaP224,
            0x70003041 => TEEAlgorithm::OpteeEcdsaP256,
            0x70004041 => TEEAlgorithm::OpteeEcdsaP384,
            0x70005041 => TEEAlgorithm::OpteeEcdsaP521,
            0x80001042 => TEEAlgorithm::OpteeEcdhP192,
            0x80002042 => TEEAlgorithm::OpteeEcdhP224,
            0x80003042 => TEEAlgorithm::OpteeEcdhP256,
            0x80004042 => TEEAlgorithm::OpteeEcdhP384,
            0x80005042 => TEEAlgorithm::OpteeEcdhP521,
            0x70001042 => TEEAlgorithm::EcdsaSha1,
            0x70002042 => TEEAlgorithm::EcdsaSha224,
            0x70003042 => TEEAlgorithm::EcdsaSha256,
            0x70004042 => TEEAlgorithm::EcdsaSha384,
            0x70005042 => TEEAlgorithm::EcdsaSha512,
            0x70006042 => TEEAlgorithm::EcdsaSha3_224,
            0x70007042 => TEEAlgorithm::EcdsaSha3_256,
            0x70008042 => TEEAlgorithm::EcdsaSha3_384,
            0x70009042 => TEEAlgorithm::EcdsaSha3_512,
            0x80000042 => TEEAlgorithm::EcdhDeriveSharedSecret,
            0x70006043 => TEEAlgorithm::Ed25519,
            0x70006044 => TEEAlgorithm::Ed448,
            0x80000046 => TEEAlgorithm::Sm2Pke,
            0x80000047 => TEEAlgorithm::Hkdf,
            0x50000007 => TEEAlgorithm::Sm3,
            0x80000044 => TEEAlgorithm::X25519,
            0x80000045 => TEEAlgorithm::X448,
            0x10000015 => TEEAlgorithm::Sm4EcbPkcs5,
            0x10000115 => TEEAlgorithm::Sm4CbcPkcs5,
            0x50000008 => TEEAlgorithm::Sha3_224,
            0x50000009 => TEEAlgorithm::Sha3_256,
            0x5000000A => TEEAlgorithm::Sha3_384,
            0x5000000B => TEEAlgorithm::Sha3_512,
            0x50000101 => TEEAlgorithm::Shake128,
            0x50000102 => TEEAlgorithm::Shake256,
            _ => TEEAlgorithm::IllegalValue,
        }
    }
}

impl Into<u32> for TEEAlgorithm {
    /// Convert TEEAlgorithm to u32 value
    fn into(self) -> u32 {
        self as u32
    }
}

// Legacy constant definitions for backward compatibility
// These maintain the original C-style defines for compatibility with existing code
// ECDSA_P192
pub(crate) const TEE_ALG_ECDSA_P192: u32 = TEE_ALG_ECDSA_SHA1;
// ECDSA_P224
pub(crate) const TEE_ALG_ECDSA_P224: u32 = TEE_ALG_ECDSA_SHA224;
// ECDSA_P256
pub(crate) const TEE_ALG_ECDSA_P256: u32 = TEE_ALG_ECDSA_SHA256;
// ECDSA_P384
pub(crate) const TEE_ALG_ECDSA_P384: u32 = TEE_ALG_ECDSA_SHA384;
// ECDSA_P521
pub(crate) const TEE_ALG_ECDSA_P521: u32 = TEE_ALG_ECDSA_SHA512;

// ECDH variants all map to the same shared secret derivation algorithm
// ECDH_P192
pub(crate) const TEE_ALG_ECDH_P192: u32 = TEE_ALG_ECDH_DERIVE_SHARED_SECRET;
// ECDH_P224
pub(crate) const TEE_ALG_ECDH_P224: u32 = TEE_ALG_ECDH_DERIVE_SHARED_SECRET;
// ECDH_P256
pub(crate) const TEE_ALG_ECDH_P256: u32 = TEE_ALG_ECDH_DERIVE_SHARED_SECRET;
// ECDH_P384
pub(crate) const TEE_ALG_ECDH_P384: u32 = TEE_ALG_ECDH_DERIVE_SHARED_SECRET;
// ECDH_P521
pub(crate) const TEE_ALG_ECDH_P521: u32 = TEE_ALG_ECDH_DERIVE_SHARED_SECRET;

// Main algorithm identifiers
// ECDSA SHA1
pub(crate) const TEE_ALG_ECDSA_SHA1: u32 = 0x70001042;
// ECDSA SHA224
pub(crate) const TEE_ALG_ECDSA_SHA224: u32 = 0x70002042;
// ECDSA SHA256
pub(crate) const TEE_ALG_ECDSA_SHA256: u32 = 0x70003042;
// ECDSA SHA384
pub(crate) const TEE_ALG_ECDSA_SHA384: u32 = 0x70004042;
// ECDSA SHA512
pub(crate) const TEE_ALG_ECDSA_SHA512: u32 = 0x70005042;
// ECDSA SHA3
pub(crate) const TEE_ALG_ECDSA_SHA3_224: u32 = 0x70006042;
// ECDSA SHA3
pub(crate) const TEE_ALG_ECDSA_SHA3_256: u32 = 0x70007042;
// ECDSA SHA3
pub(crate) const TEE_ALG_ECDSA_SHA3_384: u32 = 0x70008042;
// ECDSA SHA3
pub(crate) const TEE_ALG_ECDSA_SHA3_512: u32 = 0x70009042;

// ECDH
pub(crate) const TEE_ALG_ECDH_DERIVE_SHARED_SECRET: u32 = 0x80000042;
// EdDSA
pub(crate) const TEE_ALG_ED25519: u32 = 0x70006043;
// EdDSA
pub(crate) const TEE_ALG_ED448: u32 = 0x70006044;
// SM2
pub(crate) const TEE_ALG_SM2_PKE: u32 = 0x80000046;
// HKDF
pub(crate) const TEE_ALG_HKDF: u32 = 0x80000047;
// SM3
pub(crate) const TEE_ALG_SM3: u32 = 0x50000007;
// X25519
pub(crate) const TEE_ALG_X25519: u32 = 0x80000044;
// X448
pub(crate) const TEE_ALG_X448: u32 = 0x80000045;
// SM4 ECB
pub(crate) const TEE_ALG_SM4_ECB_PKCS5: u32 = 0x10000015;
// SM4 CBC
pub(crate) const TEE_ALG_SM4_CBC_PKCS5: u32 = 0x10000115;
// ILLEGAL VALUE
pub(crate) const TEE_ALG_ILLEGAL_VALUE: u32 = 0xEFFFFFFF;

// OP-TEE specific algorithm identifiers due to specification inconsistency
// ECDSA_P192
pub(crate) const __OPTEE_ALG_ECDSA_P192: u32 = 0x70001041;
// ECDSA_P224
pub(crate) const __OPTEE_ALG_ECDSA_P224: u32 = 0x70002041;
// ECDSA_P256
pub(crate) const __OPTEE_ALG_ECDSA_P256: u32 = 0x70003041;
// ECDSA_P384
pub(crate) const __OPTEE_ALG_ECDSA_P384: u32 = 0x70004041;
// ECDSA_P521
pub(crate) const __OPTEE_ALG_ECDSA_P521: u32 = 0x70005041;
// ECDH 192
pub(crate) const __OPTEE_ALG_ECDH_P192: u32 = 0x80001042;
// ECDH 224
pub(crate) const __OPTEE_ALG_ECDH_P224: u32 = 0x80002042;
// ECDH 256
pub(crate) const __OPTEE_ALG_ECDH_P256: u32 = 0x80003042;
// ECDH 384
pub(crate) const __OPTEE_ALG_ECDH_P384: u32 = 0x80004042;
// ECDH 521
pub(crate) const __OPTEE_ALG_ECDH_P521: u32 = 0x80005042;
