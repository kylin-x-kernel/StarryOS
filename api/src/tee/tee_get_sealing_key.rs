use core::arch::asm;
use core::ptr;
use alloc::vec::Vec;
use core::mem::size_of;

pub const GUEST_ATTESTATION_DATA_SIZE: usize = 64;
pub const GUEST_ATTESTATION_NONCE_SIZE: usize = 16;
pub const HASH_LEN: usize = 32;
pub const PAGE_SIZE: usize = 4096;
pub const KVM_HC_VM_ATTESTATION: u32 = 0x100;

pub const VM_ID_SIZE: usize = 16;
pub const VM_VERSION_SIZE: usize = 16;
pub const USER_DATA_SIZE: usize = 64;
pub const ECC_POINT_SIZE: usize = 72;
pub const SIZE_INT32: usize = 4;
pub const SN_LEN: usize = 64;
pub const CSV_CERT_RSVD3_SIZE: usize = 624;
pub const CSV_CERT_RSVD4_SIZE: usize = 368;
pub const CSV_CERT_RSVD5_SIZE: usize = 368;
pub const HYGON_USER_ID_SIZE: usize = 256;

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
    pub signature: AttestationSignature,
    pub pek_cert: HygonCsvCert,
    pub sn: [u8; SN_LEN],
    pub reserved2: [u8; 32],
    pub mac: HashBlockU,
}

#[repr(C, packed)]  // ✓ 添加 packed
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

#[repr(C, packed)]
pub struct HygonCsvCert {
    pub version: u32,
    pub api_major: u8,
    pub api_minor: u8,
    pub reserved1: u8,
    pub reserved2: u8,
    pub pubkey_usage: u32,
    pub pubkey_algo: u32,
    pub pubkey_data: CertPubkeyData,
    pub reserved3: [u32; CSV_CERT_RSVD3_SIZE / SIZE_INT32],
    pub sig1_usage: u32,
    pub sig1_algo: u32,
    pub sig1_data: AttestationSignature,
    pub reserved4: [u32; CSV_CERT_RSVD4_SIZE / SIZE_INT32],
    pub sig2_usage: u32,
    pub sig2_algo: u32,
    pub sig2_data: AttestationSignature,
    pub reserved5: [u32; CSV_CERT_RSVD5_SIZE / SIZE_INT32],
}

#[repr(C)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct EccPubkeyT {
    pub curve_id: u32,
    pub qx: [u32; ECC_POINT_SIZE / SIZE_INT32],
    pub qy: [u32; ECC_POINT_SIZE / SIZE_INT32],
    pub user_id: [u32; HYGON_USER_ID_SIZE / SIZE_INT32],
}

#[repr(C)]
pub union CertPubkeyData {
    pub pubkey: [u32; (SIZE_INT32 + ECC_POINT_SIZE * 2 + HYGON_USER_ID_SIZE) / SIZE_INT32],
    pub ecc_pubkey: EccPubkeyT,  // ✓ 修正：改为 ecc_pubkey
}

// ==================== Helper Functions ====================

/// 执行 hypercall 调用
///
/// # Safety
/// 此函数使用内联汇编，仅应在 x86_64 架构上使用
pub unsafe fn hypercall(nr: u32, p1: usize, len: usize) -> i64 {
    #[cfg(target_arch = "x86_64")]
    {
        let ret: i64;
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

    #[cfg(not(target_arch = "x86_64"))]
    {
        // 对于其他架构，返回错误
        -1
    }
}

use crate::syscall::sys_getrandom;
use crate::tee::tee_svc_cryp::{
    syscall_cryp_obj_alloc, syscall_obj_generate_key,
    tee_cryp_obj_secret_wrapper, TeeCryptObj,
};
use crate::tee::tee_svc_cryp2::{
    syscall_cryp_state_alloc, syscall_hash_final,
    syscall_hash_init, syscall_hash_update,
};
use crate::tee::tee_obj::tee_obj_get;
use tee_raw_sys::{
    TEE_ALG_SM3, TEE_ALG_HMAC_SM3,
    TEE_TYPE_SM4, TEE_TYPE_HMAC_SM3,
    TEE_OperationMode,
};
use tee_raw_sys::TEE_OperationMode::{TEE_MODE_DIGEST, TEE_MODE_MAC};

// ==================== Trait Implementations ====================

impl CsvAttestationUserData {
    /// 使用指定的 mnonce 创建用户数据
    pub fn with_mnonce(mnonce: &[u8; GUEST_ATTESTATION_NONCE_SIZE]) -> Self {
        Self {
            data: Self::build_user_data(),
            mnonce: *mnonce,
            hash: HashBlockU {
                block: [0u8; HASH_LEN],
            },
        }
    }

    fn build_user_data() -> [u8; GUEST_ATTESTATION_DATA_SIZE] {
        let mut data = [0u8; GUEST_ATTESTATION_DATA_SIZE];
        let user_data_str = b"user data";
        let copy_len = user_data_str.len().min(GUEST_ATTESTATION_DATA_SIZE);
        data[..copy_len].copy_from_slice(user_data_str);
        data
    }

    /// 构建哈希输入数据（data + mnonce）
    pub fn hash_input(&self) -> Vec<u8> {
        let mut input = Vec::with_capacity(GUEST_ATTESTATION_DATA_SIZE + GUEST_ATTESTATION_NONCE_SIZE);
        input.extend_from_slice(&self.data);
        input.extend_from_slice(&self.mnonce);
        input
    }

    /// 计算并设置 SM3 哈希值
    pub fn compute_sm3_hash(&mut self) -> Result<(), u32> {
        let input = self.hash_input();
        let hash = compute_sm3_hash(&input)?;
        self.hash.block.copy_from_slice(&hash);
        Ok(())
    }

    /// 获取物理地址
    pub fn physical_address(&self) -> usize {
        self as *const Self as usize
    }
}

impl CsvAttestationReport {
    /// 从用户数据填充报告
    pub unsafe fn fill_from_user_data(&mut self, user_data: &CsvAttestationUserData) {
        self.user_data.copy_from_slice(&user_data.data);
        self.mnonce.copy_from_slice(&user_data.mnonce);
        self.measure.block.copy_from_slice(&user_data.hash.block);
    }

    /// 验证 mnonce 匹配
    pub fn verify_mnonce(&self, expected_mnonce: &[u8; GUEST_ATTESTATION_NONCE_SIZE]) -> bool {
        let computed_mnonce = self.compute_mnonce();
        &computed_mnonce == expected_mnonce
    }

    /// 计算恢复的 mnonce
    pub fn compute_mnonce(&self) -> [u8; GUEST_ATTESTATION_NONCE_SIZE] {
        let mnonce_as_u32: [u32; GUEST_ATTESTATION_NONCE_SIZE / 4] =
            unsafe { core::mem::transmute(self.mnonce) };

        let mut r_mnonce_as_u32: [u32; GUEST_ATTESTATION_NONCE_SIZE / 4] = [0u32; 4];

        for (i, &mnonce) in mnonce_as_u32.iter().enumerate() {
            r_mnonce_as_u32[i] = mnonce ^ self.anonce;
        }

        unsafe { core::mem::transmute(r_mnonce_as_u32) }
    }

    /// 获取密封密钥
    pub fn sealing_key(&self) -> [u8; 32] {
        self.reserved2
    }
}

// ==================== Hash Functions ====================

/// 计算 SM3 哈希值
fn compute_sm3_hash(data: &[u8]) -> Result<[u8; HASH_LEN], u32> {
    let mut state = 0u32;
    syscall_cryp_state_alloc(TEE_ALG_SM3, TEE_MODE_DIGEST, None, None, &mut state)?;
    syscall_hash_init(state)?;
    syscall_hash_update(state, data)?;
    let mut hash = [0u8; HASH_LEN];
    syscall_hash_final(state, &[], &mut hash)?;
    Ok(hash)
}

// ==================== Public API Functions ====================

/// 获取远程认证报告
///
/// # Safety
/// 调用者必须确保 report 指针是有效的
///
/// # Arguments
/// * `report` - 指向接收认证报告的指针
/// * `mnonce` - 用于认证的 mnonce 值
///
/// # Returns
/// * `Ok(())` - 成功获取认证报告
/// * `Err(())` - 获取失败
pub unsafe fn get_attestation_report(
    report: *mut CsvAttestationReport,
    mnonce: &[u8; GUEST_ATTESTATION_NONCE_SIZE],
) -> Result<(), u32> {
    if report.is_null() {
        error!("NULL pointer for report");
        return Err(1);
    }

    let mut user_data = CsvAttestationUserData::with_mnonce(mnonce);

    user_data.compute_sm3_hash()?;

    let user_data_pa = user_data.physical_address();
    let ret = hypercall(KVM_HC_VM_ATTESTATION, user_data_pa, PAGE_SIZE);

    if ret != 0 {
        error!("hypercall failed: {}", ret);
        return Err(1);
    }

    (*report).fill_from_user_data(&user_data);

    Ok(())
}

/// 验证会话 MAC
///
/// # Arguments
/// * `report` - 认证报告引用
/// * `key` - 用于 HMAC 计算的密钥
///
/// # Returns
/// * `Ok(())` - MAC 验证成功
/// * `Err(())` - MAC 验证失败
pub unsafe fn verify_session_mac(
    report: &CsvAttestationReport,
    key: &[u8],
) -> Result<(), u32> {
    let computed_mac = compute_hmac_sm3(report, key)?;

    if computed_mac == report.mac.block {
        info!("attestation report MAC verify success");
        Ok(())
    } else {
        error!("attestation report MAC verify failed");
        Err(1)
    }
}

/// 计算 HMAC-SM3
///
/// # Arguments
/// * `report` - 认证报告引用
/// * `key` - HMAC 密钥
///
/// # Returns
/// * `Ok([u8; 32])` - 计算得到的 MAC 值
/// * `Err(())` - 计算失败
fn compute_hmac_sm3(
    report: &CsvAttestationReport,
    key: &[u8],
) -> Result<[u8; HASH_LEN], u32> {
    let mut state = 0u32;
    let mut obj_id: core::ffi::c_uint = 0;

    syscall_cryp_obj_alloc(TEE_TYPE_SM4 as _, 128, &mut obj_id)?;
    syscall_obj_generate_key(obj_id as u64, 128, core::ptr::null(), 0)?;

    let obj_arc = tee_obj_get(obj_id as u64)?;
    let mut obj = obj_arc.lock();

    let mut secret = tee_cryp_obj_secret_wrapper::new(32);
    secret.set_secret_data(key);
    let _ = core::mem::replace(&mut obj.attr[0], TeeCryptObj::obj_secret(secret));
    let _ = core::mem::replace(&mut obj.info.objectType, TEE_TYPE_HMAC_SM3);
    drop(obj);

    syscall_cryp_state_alloc(
        TEE_ALG_HMAC_SM3,
        TEE_MODE_MAC,
        Some(obj_id as _),
        None,
        &mut state,
    )?;
    syscall_hash_init(state)?;

    let pek_cert_size = size_of::<HygonCsvCert>();
    let pek_cert_ptr = &report.pek_cert as *const _ as *const u8;

    let mut data_to_hash = Vec::with_capacity(pek_cert_size + report.sn.len() + report.reserved2.len());

    data_to_hash.extend_from_slice(unsafe {
        core::slice::from_raw_parts(pek_cert_ptr, pek_cert_size)
    });
    data_to_hash.extend_from_slice(&report.sn);
    data_to_hash.extend_from_slice(&report.reserved2);

    syscall_hash_update(state, &data_to_hash)?;

    let mut hash = [0u8; HASH_LEN];
    syscall_hash_final(state, &[], &mut hash)?;

    Ok(hash)
}

/// 获取密封密钥
///
/// # Arguments
/// * `key_buf` - 用于存储密封密钥的缓冲区
/// * `buf_len` - 缓冲区长度
///
/// # Returns
/// * `Ok(())` - 成功获取密封密钥
/// * `Err(())` - 获取失败
///
/// # Safety
/// 调用者必须确保 key_buf 指向的内存区域有效且可写，且 buf_len 与实际分配大小匹配
pub unsafe fn vmmcall_get_sealing_key(
    key_buf: *mut u8,
    buf_len: usize,
) -> Result<(), u32> {
    if buf_len < 32 {
        error!("The allocated length is too short to hold the sealing key!");
        error!("The length should not be less than 32");
        return Err(1);
    }

    if key_buf.is_null() {
        error!("Key buffer pointer is null");
        return Err(1);
    }

    let mut report: CsvAttestationReport = core::mem::zeroed();
    let mut default_key = [0u8; GUEST_ATTESTATION_NONCE_SIZE];
    sys_getrandom(default_key.as_mut_ptr(), GUEST_ATTESTATION_NONCE_SIZE, 0);

    get_attestation_report(&mut report, &default_key)?;
    verify_session_mac(&report, &default_key)?;

    if !report.verify_mnonce(&default_key) {
        error!("mnonce verification failed");
        return Err(1);
    }

    let sealing_key = report.sealing_key();
    ptr::copy_nonoverlapping(sealing_key.as_ptr(), key_buf, sealing_key.len());

    Ok(())
}

// ==================== Utility Functions ====================

/// 比较两个 mnonce 数组
///
/// 此函数已弃用，建议使用 `CsvAttestationReport::verify_mnonce` 代替
#[deprecated(note = "Use CsvAttestationReport::verify_mnonce instead")]
pub unsafe fn compare_mnonce(
    a: &[u8; GUEST_ATTESTATION_NONCE_SIZE],
    b: &[u8; GUEST_ATTESTATION_NONCE_SIZE],
) -> bool {
    a == b
}
