// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been modified by KylinSoft on 2025.

/*
 * RSA signatures with MD5 hash
 * Values prefixed with vendor ID bit31 with by TEE bitfields IDs
 */
pub const TEE_ALG_RSASSA_PKCS1_PSS_MGF1_MD5: u32 = 0xF0111930;
pub const TEE_ALG_RSAES_PKCS1_OAEP_MGF1_MD5: u32 = 0xF0110230;

/*
 * API extended result codes as per TEE_Result IDs defined in GPD TEE
 * Internal Core API specification v1.1:
 *
 * 0x70000000 - 0x7FFFFFFF: Reserved for implementation-specific return
 *			    code providing non-error information
 * 0x80000000 - 0x8FFFFFFF: Reserved for implementation-specific errors
 *
 * TEE_ERROR_DEFER_DRIVER_INIT - Device driver failed to initialize because
 * the driver depends on a device not yet initialized.
 */
pub const TEE_ERROR_DEFER_DRIVER_INIT: u32 = 0x80000000;

/*
 * TEE_ERROR_NODE_DISABLED - Device driver failed to initialize because it is
 * not allocated for TEE environment.
 */
pub const TEE_ERROR_NODE_DISABLED: u32 = 0x80000001;

/*
 * HMAC-based Extract-and-Expand Key Derivation Function (HKDF)
 */

pub const TEE_ALG_HKDF_MD5_DERIVE_KEY: u32 = 0x800010C0;
pub const TEE_ALG_HKDF_SHA1_DERIVE_KEY: u32 = 0x800020C0;
pub const TEE_ALG_HKDF_SHA224_DERIVE_KEY: u32 = 0x800030C0;
pub const TEE_ALG_HKDF_SHA256_DERIVE_KEY: u32 = 0x800040C0;
pub const TEE_ALG_HKDF_SHA384_DERIVE_KEY: u32 = 0x800050C0;
pub const TEE_ALG_HKDF_SHA512_DERIVE_KEY: u32 = 0x800060C0;

pub const TEE_TYPE_HKDF_IKM: u32 = 0xA10000C0;

pub const TEE_ATTR_HKDF_IKM: u32 = 0xC00001C0;
/*
 * There is a name clash with the  official attributes TEE_ATTR_HKDF_SALT
 * and TEE_ATTR_HKDF_INFO so define these alternative ID.
 */
pub const __OPTEE_TEE_ATTR_HKDF_SALT: u32 = 0xD00002C0;
pub const __OPTEE_TEE_ATTR_HKDF_INFO: u32 = 0xD00003C0;
pub const TEE_ATTR_HKDF_OKM_LENGTH: u32 = 0xF00004C0;

/*
 * Concatenation Key Derivation Function (Concat KDF)
 * NIST SP 800-56A section 5.8.1
 */

pub const TEE_ALG_CONCAT_KDF_SHA1_DERIVE_KEY: u32 = 0x800020C1;
pub const TEE_ALG_CONCAT_KDF_SHA224_DERIVE_KEY: u32 = 0x800030C1;
pub const TEE_ALG_CONCAT_KDF_SHA256_DERIVE_KEY: u32 = 0x800040C1;
pub const TEE_ALG_CONCAT_KDF_SHA384_DERIVE_KEY: u32 = 0x800050C1;
pub const TEE_ALG_CONCAT_KDF_SHA512_DERIVE_KEY: u32 = 0x800060C1;

pub const TEE_TYPE_CONCAT_KDF_Z: u32 = 0xA10000C1;

pub const TEE_ATTR_CONCAT_KDF_Z: u32 = 0xC00001C1;
pub const TEE_ATTR_CONCAT_KDF_OTHER_INFO: u32 = 0xD00002C1;
pub const TEE_ATTR_CONCAT_KDF_DKM_LENGTH: u32 = 0xF00003C1;

/*
 * PKCS #5 v2.0 Key Derivation Function 2 (PBKDF2)
 * RFC 2898 section 5.2
 * https://www.ietf.org/rfc/rfc2898.txt
 */

pub const TEE_ALG_PBKDF2_HMAC_SHA1_DERIVE_KEY: u32 = 0x800020C2;

pub const TEE_TYPE_PBKDF2_PASSWORD: u32 = 0xA10000C2;

pub const TEE_ATTR_PBKDF2_PASSWORD: u32 = 0xC00001C2;
pub const TEE_ATTR_PBKDF2_SALT: u32 = 0xD00002C2;
pub const TEE_ATTR_PBKDF2_ITERATION_COUNT: u32 = 0xF00003C2;
pub const TEE_ATTR_PBKDF2_DKM_LENGTH: u32 = 0xF00004C2;

/*
 * PKCS#1 v1.5 RSASSA pre-hashed sign/verify
 */

pub const TEE_ALG_RSASSA_PKCS1_V1_5: u32 = 0xF0000830;

/*
 *  TDEA CMAC (NIST SP800-38B)
 */
pub const TEE_ALG_DES3_CMAC: u32 = 0xF0000613;

/*
 *  SM4-XTS
 */
pub const TEE_ALG_SM4_XTS: u32 = 0xF0000414;

/*
 * Implementation-specific object storage constants
 */

/* Storage is provided by the Rich Execution Environment (REE) */
pub const TEE_STORAGE_PRIVATE_REE: u32 = 0x80000000;
/* Storage is the Replay Protected Memory Block partition of an eMMC device */
pub const TEE_STORAGE_PRIVATE_RPMB: u32 = 0x80000100;
/* Was TEE_STORAGE_PRIVATE_SQL, which isn't supported any longer */
pub const TEE_STORAGE_PRIVATE_SQL_RESERVED: u32 = 0x80000200;

/*
 * Extension of "Memory Access Rights Constants"
 * #define TEE_MEMORY_ACCESS_READ             0x00000001
 * #define TEE_MEMORY_ACCESS_WRITE            0x00000002
 * #define TEE_MEMORY_ACCESS_ANY_OWNER        0x00000004
 *
 * TEE_MEMORY_ACCESS_NONSECURE : if set TEE_CheckMemoryAccessRights()
 * successfully returns only if target vmem range is mapped non-secure.
 *
 * TEE_MEMORY_ACCESS_SECURE : if set TEE_CheckMemoryAccessRights()
 * successfully returns only if target vmem range is mapped secure.

 */
pub const TEE_MEMORY_ACCESS_NONSECURE: u32 = 0x10000000;
pub const TEE_MEMORY_ACCESS_SECURE: u32 = 0x20000000;

/*
 * Implementation-specific login types
 */

/* Private login method for REE kernel clients */
pub const TEE_LOGIN_REE_KERNEL: u32 = 0x80000000;