use core::arch::asm;

use alloc::vec::Vec;
use core::mem::size_of;

const GUEST_ATTESTATION_DATA_SIZE: usize = 64;
const GUEST_ATTESTATION_NONCE_SIZE: usize = 16;
const HASH_LEN: usize = 32;
const PAGE_SIZE: usize = 4096;
const KVM_HC_VM_ATTESTATION: u32 = 0x100; // 假设的 hypercall 编号,需要根据实际定义调整

#[repr(C)]
pub struct CsvAttestationUserData {
    pub data: [u8; GUEST_ATTESTATION_DATA_SIZE],
    pub mnonce: [u8; GUEST_ATTESTATION_NONCE_SIZE],
    pub hash: HashBlockU,
}

#[repr(C)]
pub struct CsvGuestMem {
    pub va: usize,
    pub size: i32,
}

#[repr(C)]
pub struct HashBlockU {
    pub block: [u8; HASH_LEN],
}


// struct csv_attestation_report {
//     hash_block_t user_pubkey_digest;
//     uint8_t     vm_id[VM_ID_SIZE];
//     uint8_t     vm_version[VM_VERSION_SIZE];
//     uint8_t     user_data[USER_DATA_SIZE];
//     uint8_t      mnonce[GUEST_ATTESTATION_NONCE_SIZE];
//     hash_block_t measure;
//     uint32_t policy;
//     uint32_t sig_usage;
//     uint32_t sig_algo;
//     uint32_t anonce;
//     union {
//         uint32_t sig1[ECC_POINT_SIZE*2/SIZE_INT32];
//         ecc_signature_t ecc_sig1;
//     };
//     CSV_CERT_t pek_cert;
//     uint8_t sn[SN_LEN];
//     uint8_t reserved2[32];
//     hash_block_u      mac;
// };
const VM_ID_SIZE: usize = 16;          
const VM_VERSION_SIZE: usize = 16;     
const USER_DATA_SIZE: usize = 64;
const ECC_POINT_SIZE: usize = 72;
const SIZE_INT32: usize = 4;
const SN_LEN: usize = 64;



#[repr(C)]
pub struct CsvAttestationReport {
    pub user_pubkey_digest: HashBlockU,
    pub vm_id: [u8; VM_ID_SIZE],
    pub vm_version: [u8; VM_VERSION_SIZE],
    pub user_data: [u8; USER_DATA_SIZE],
    pub mnonce: [u8; GUEST_ATTESTATION_NONCE_SIZE],
    pub measure: HashBlockU,
    pub policy: u32,
    pub sig_usage: u32,
    pub sig_algo: u32,
    pub anonce: u32,
    pub signature: AttestationSignature,          // 封装匿名联合体
    pub pek_cert: HygonCsvCert,
    pub sn: [u8; SN_LEN],
    pub reserved2: [u8; 32],
    pub mac: HashBlockU,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct EccSignatureT {
    pub sig_r: [u32; ECC_POINT_SIZE / SIZE_INT32],
    pub sig_s: [u32; ECC_POINT_SIZE / SIZE_INT32],
}

#[repr(C)]
pub union AttestationSignature {
    pub sig1: [u32; ECC_POINT_SIZE * 2 / SIZE_INT32],
    pub ecc_sig1: EccSignatureT,
}



// union {
//         uint32_t        sig1[ECC_POINT_SIZE * 2 / SIZE_INT32];
//         ecc_signature_t ecc_sig1;
//     };


// struct _hygon_csv_cert {
//     uint32_t version;
//     uint8_t  api_major;
//     uint8_t  api_minor;
//     uint8_t  reserved1;    // asm!(
    //     "vmmcall",
    //     inlateout("rax") nr => _,
    //     in("rbx") p1,
    //     in("rcx") len,
    //     lateout("rax") ret,
    //     clobber_abi("system"),
    // );
//     uint8_t  reserved2;
//     uint32_t pubkey_usage;
//     uint32_t pubkey_algo;
//     union {
//         uint32_t     pubkey[(SIZE_INT32 + ECC_POINT_SIZE * 2 + HYGON_USER_ID_SIZE) / SIZE_INT32];
//         ecc_pubkey_t ecc_pubkey;//     union {
//         uint32_t        sig1[ECC_POINT_SIZE * 2 / SIZE_INT32];
//         ecc_signature_t ecc_sig1;
//     };
//     };
//     uint32_t reserved3[CSV_CERT_RSVD3_SIZE / SIZE_INT32];
//     uint32_t sig1_algo;
//     union {
//         uint32_t        sig1[ECC_POINT_SIZE * 2 / SIZE_INT32];
//         ecc_signature_t ecc_sig1;
//     };
//     uint32_t reserved4[CSV_CERT_RSVD4_SIZE / SIZE_INT32];
//     uint32_t sig2_usage;
//     uint32_t sig2_algo;
//     union {
//         uint32_t        sig2[ECC_POINT_SIZE * 2 / SIZE_INT32];
//         ecc_signature_t ecc_sig2;
//     };
//     uint32_t reserved5[CSV_CERT_RSVD5_SIZE / SIZE_INT32];
// } __attribute__((packed));
// typedef struct _hygon_csv_cert  CSV_CERT_t;

const CSV_CERT_RSVD3_SIZE: usize = 624;
const CSV_CERT_RSVD4_SIZE: usize = 368;
const CSV_CERT_RSVD5_SIZE: usize = 368;
const HYGON_USER_ID_SIZE: usize = 256;

#[repr(C, packed)]
pub struct HygonCsvCert {
    pub version: u32,
    pub api_major: u8,
    pub api_minor: u8,
    pub reserved1: u8,
    pub reserved2: u8,
    pub pubkey_usage: u32,
    pub pubkey_algo: u32,
    pub pubkey_data: CertPubkeyData,        // 封装 pubkey 联合体
    pub reserved3: [u32; CSV_CERT_RSVD3_SIZE / SIZE_INT32],
    pub sig1_usage: u32,
    pub sig1_algo: u32,
    pub sig1_data: AttestationSignature,       // 封装 sig1 联合体
    pub reserved4: [u32; CSV_CERT_RSVD4_SIZE / SIZE_INT32],
    pub sig2_usage: u32,
    pub sig2_algo: u32,
    pub sig2_data: AttestationSignature,       // 封装 sig2 联合体
    pub reserved5: [u32; CSV_CERT_RSVD5_SIZE / SIZE_INT32],
}

#[repr(C)]
pub union CertPubkeyData {
    pub pubkey: [u32; (SIZE_INT32 + ECC_POINT_SIZE * 2 + HYGON_USER_ID_SIZE) / SIZE_INT32],
    pub ecc_sig: EccSignatureT,
}






// long hypercall(unsigned int nr, unsigned long p1, unsigned int len)
// {
//     long ret = 0;

//     asm volatile("vmmcall"
//              : "=a"(ret)
//              : "a"(nr), "b"(p1), "c"(len)
//              : "memory");
//     return ret;
// }


pub unsafe fn hypercall(nr: u32, p1: usize, len: usize) -> i64 {
    let ret: i64 = 0;

    #[cfg(target_arch = "x86_64")]
    asm!(
        "vmmcall",
        inlateout("rax") nr => _,
        in("rbx") p1,
        in("rcx") len,
        lateout("rax") ret,
        clobber_abi("system"),
    );

    ret
}

use crate::syscall::sys_getrandom;
use crate::tee::tee_svc_cryp::{
    syscall_cryp_obj_alloc, syscall_obj_generate_key, tee_cryp_obj_secret_wrapper, TeeCryptObj,
};
use crate::tee::tee_svc_cryp2::{
    syscall_cryp_state_alloc, syscall_hash_final, syscall_hash_init, syscall_hash_update,
};
use crate::tee::tee_obj::tee_obj_get;
use tee_raw_sys::{
    TEE_ALG_SM3, TEE_ALG_HMAC_SM3, TEE_OperationMode,
    TEE_TYPE_SM4, TEE_TYPE_HMAC_SM3,
};
use tee_raw_sys::TEE_OperationMode::{TEE_MODE_DIGEST, TEE_MODE_MAC};


/// 获取远程认证报告
///
/// # Safety
/// 调用者必须确保 report 指针是有效的
///
/// # Arguments
/// * `report` - 指向接收认证报告的指针
///
/// # Returns
/// * Ok(()) - 成功获取认证报告
/// * Err(()) - 获取失败
pub unsafe fn get_attestation_report(report: *mut CsvAttestationReport) -> Result<(), ()> {
    if report.is_null() {
        error!("NULL pointer for report");
        return Err(());
    }

    // 准备用户数据
    let mut user_data = CsvAttestationUserData {
        data: {
            let mut buf = [0u8; GUEST_ATTESTATION_DATA_SIZE];
            let user_data_str = b"user data";
            let copy_len = user_data_str.len().min(GUEST_ATTESTATION_DATA_SIZE);
            buf[..copy_len].copy_from_slice(user_data_str);
            buf
        },
        mnonce: {
            let mut nonce = [0u8; GUEST_ATTESTATION_NONCE_SIZE];
            let _ = sys_getrandom(nonce.as_mut_ptr(), GUEST_ATTESTATION_NONCE_SIZE, 0);
            nonce
        },
        hash: HashBlockU {
            block: [0u8; HASH_LEN],
        },
    };

    // 计算 SM3 哈希值
    let mut state: u32 = 0;
    let res = syscall_cryp_state_alloc(TEE_ALG_SM3, TEE_MODE_DIGEST, None, None, &mut state);
    if res.is_err() {
        error!("Failed to allocate SM3 state");
        return Err(());
    }

    let res = syscall_hash_init(state);
    if res.is_err() {
        error!("Failed to init SM3 hash");
        return Err(());
    }

    // 计算 data + mnonce 的哈希值
    let mut hash_input = Vec::with_capacity(GUEST_ATTESTATION_DATA_SIZE + GUEST_ATTESTATION_NONCE_SIZE);
    hash_input.extend_from_slice(&user_data.data);
    hash_input.extend_from_slice(&user_data.mnonce);

    let res = syscall_hash_update(state, &hash_input);
    if res.is_err() {
        error!("Failed to update SM3 hash");
        return Err(());
    }

    let res = syscall_hash_final(state, &[], &mut user_data.hash.block);
    if res.is_err() {
        error!("Failed to finalize SM3 hash");
        return Err(());
    }

    let user_data_pa = &user_data as *const _ as usize;
    let ret = hypercall(KVM_HC_VM_ATTESTATION, user_data_pa, PAGE_SIZE);

    if ret != 0 {
        error!("hypercall failed: {}", ret);
        return Err(());
    }

    // TODO: 从 hypercall 返回的数据中填充 report 结构

    Ok(())
}

pub unsafe fn verify_session_mac(report: &CsvAttestationReport, key: &[u8]) -> Result<(), ()> {
    // 执行 HMAC-SM3 计算
    let computed_mac = unsafe { hmac_sm3_calculation(report, key)? };

    // 比较计算出的 MAC 和报告中的 MAC
    if computed_mac == report.mac.block {
        info!("attestation report MAC verify success");
        Ok(())
    } else {
        error!("attestation report MAC verify failed");
        Err(())
    }
}

unsafe fn hmac_sm3_calculation(report: &CsvAttestationReport, key: &[u8]) -> Result<[u8; HASH_LEN], ()> {
    let mut state: u32 = 0;
    let mut obj_id: core::ffi::c_uint = 0;

    // 分配 SM4 对象用于存储密钥
    let result = syscall_cryp_obj_alloc(TEE_TYPE_SM4 as _, 128, &mut obj_id);
    if result.is_err() {
        error!("Failed to allocate crypto object");
        return Err(());
    }

    // 生成密钥（这里使用提供的密钥）
    let result = syscall_obj_generate_key(obj_id as u64, 128, core::ptr::null(), 0);
    if result.is_err() {
        error!("Failed to generate key");
        return Err(());
    }

    // 获取对象并设置密钥
    let obj_arc = tee_obj_get(obj_id as u64);
    if obj_arc.is_err() {
        error!("Failed to get crypto object");
        return Err(());
    }
    let obj_arc = obj_arc.unwrap();
    let mut obj = obj_arc.lock();

    // 设置密钥数据
    let mut secret = tee_cryp_obj_secret_wrapper::new(32);
    secret.set_secret_data(key);
    let _ = core::mem::replace(&mut obj.attr[0], TeeCryptObj::obj_secret(secret));
    let _ = core::mem::replace(&mut obj.info.objectType, TEE_TYPE_HMAC_SM3);
    drop(obj);

    // 分配 HMAC-SM3 状态
    let res = syscall_cryp_state_alloc(TEE_ALG_HMAC_SM3, TEE_MODE_MAC, Some(obj_id as _), None, &mut state);
    if res.is_err() {
        error!("Failed to allocate HMAC-SM3 state");
        return Err(());
    }

    // 初始化 HMAC
    let res = syscall_hash_init(state);
    if res.is_err() {
        error!("Failed to init HMAC");
        return Err(());
    }

    // 获取 PEK 证书部分的数据
    let pek_cert_ptr = &report.pek_cert as *const _ as *const u8;
    let pek_cert_size = size_of::<HygonCsvCert>();

    // 构建 HMAC 输入数据
    let mut data_to_hash = Vec::with_capacity(
        pek_cert_size +
        report.sn.len() +
        report.reserved2.len()
    );

    // 添加 PEK 证书数据
    data_to_hash.extend_from_slice(
        unsafe { core::slice::from_raw_parts(pek_cert_ptr, pek_cert_size) }
    );

    // 添加 SN 数据
    data_to_hash.extend_from_slice(&report.sn);

    // 添加 reserved2 数据
    data_to_hash.extend_from_slice(&report.reserved2);

    // 更新 HMAC
    let res = syscall_hash_update(state, &data_to_hash);
    if res.is_err() {
        error!("Failed to update HMAC");
        return Err(());
    }

    // 完成计算
    let mut hash: [u8; HASH_LEN] = [0; HASH_LEN];
    let res = syscall_hash_final(state, &[], &mut hash);
    if res.is_err() {
        error!("Failed to finalize HMAC");
        return Err(());
    }

    Ok(hash)
}