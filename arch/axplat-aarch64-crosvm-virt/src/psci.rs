// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 WeiKang Guo <guoweikang.kernel@gmail.com
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.

use spin::Once;

use axplat::psci::PsciIf;
use crate::serial::{boot_print_str, boot_print_usize};

/// kvm guard granule
pub static GUARD_GRANULE: Once<usize> = Once::new();

const ARM_SMCCC_VENDOR_HYP_KVM_MEM_UNSHARE_FUNC_ID: u32 =
    ((1) << 31) | ((1) << 30) | (((6) & 0x3F) << 24) | ((4) & 0xFFFF);

const ARM_SMCCC_VENDOR_HYP_KVM_MMIO_GUARD_INFO_FUNC_ID: u32 =
    ((1) << 31) | ((1) << 30) | (((6) & 0x3F) << 24) | ((5) & 0xFFFF);

const ARM_SMCCC_VENDOR_HYP_KVM_MEM_SHARE_FUNC_ID: u32 =
    ((1) << 31) | ((1) << 30) | (((6) & 0x3F) << 24) | ((3) & 0xFFFF);

pub fn psci_hvc_call(func: u32, arg0: usize, arg1: usize, arg2: usize) -> (usize, usize) {
    let ret0;
    let ret1;
    unsafe {
        core::arch::asm!(
            "hvc #0",
            inlateout("x0") func as usize => ret0,
            inlateout("x1") arg0 => ret1,
            in("x2") arg1,
            in("x3") arg2,
        )
    }
    (ret0, ret1)
}

/// 获取KVM的内存保护粒度
pub fn kvm_guard_granule_init() {
    let (guard_granule, guard_has_range) = psci_hvc_call(ARM_SMCCC_VENDOR_HYP_KVM_MMIO_GUARD_INFO_FUNC_ID, 0, 0, 0);
    assert_eq!(guard_has_range, 0x1);
    GUARD_GRANULE.call_once(|| guard_granule);
    boot_print_str("KVM MMIO guard granule: ");
    boot_print_usize(guard_granule);
}

fn __invoke_mmioguard(phys_addr: usize, nr_granules: usize, map: bool) -> usize {
    let func_id: u32 = if map {
        // ARM_SMCCC_VENDOR_HYP_KVM_MMIO_RGUARD_MAP_FUNC_ID
        ((1) << 31) | ((1) << 30) | (((6) & 0x3F) << 24) | ((10) & 0xFFFF) 
    } else {
        // ARM_SMCCC_VENDOR_HYP_KVM_MMIO_RGUARD_UNMAP_FUNC_ID
        ((1) << 31) | ((1) << 30) | (((6) & 0x3F) << 24) | ((11) & 0xFFFF) 
    };
    // 无法批量操作，每次只能操作1个页面
    let (result, done) = psci_hvc_call(func_id, phys_addr, 1, 0);
    if result != 0 {
        boot_print_str("[error] psci_hvc_call failed\r\n");
        boot_print_str("    func = ");
        boot_print_usize(func_id as _);
        boot_print_str("    arg0 = ");
        boot_print_usize(phys_addr);
        boot_print_str("    arg1 = ");
        boot_print_usize(nr_granules);
        boot_print_str("    ret0 = ");
        boot_print_usize(result);
        boot_print_str("    ret1 = ");
        boot_print_usize(done);
        panic!();
    }
    return done;
}

fn __do_xmap_granules(phys_addr: usize, nr_granules: usize, map: bool) -> usize {
    let mut nr_xmapped = 0;
    let mut nr_granules = nr_granules as isize;
    let mut phys_addr = phys_addr;

    while nr_granules > 0 {
        let __nr_xmapped = __invoke_mmioguard(phys_addr, nr_granules as usize, map);
        nr_xmapped += __nr_xmapped;
        if __nr_xmapped as isize > nr_granules {
            boot_print_str("[warning] __invoke_mmioguard");
            break;
        }
        phys_addr += __nr_xmapped * { GUARD_GRANULE.get().unwrap()};
        nr_granules -= __nr_xmapped as isize;
    }

    return nr_xmapped;
}

/// Maps a physical memory region for KVM MMIO access.
pub fn do_xmap_granules(phys_addr: usize, size: usize) {
    let nr_granules = size / { GUARD_GRANULE.get().unwrap()};
    let ret = __do_xmap_granules(phys_addr, nr_granules, true);
    assert_eq!(ret, nr_granules);
}


struct PsciImpl;

#[impl_plat_interface]
impl PsciIf for PsciImpl {

    fn unshare_dma_buffer(paddr: usize, size: usize) {
        let page_size = 0x1000;
        let pages = size / page_size;
        for i in 0..pages {
            let (ret0, _ret1) = psci_hvc_call(
                ARM_SMCCC_VENDOR_HYP_KVM_MEM_UNSHARE_FUNC_ID,
                paddr +  page_size * i,
                1,
                0,
            );
            if ret0 != 0 {
                log::warn!(
                    "[virtio hal impl] cannot unshare 0x{:x}",
                    paddr + page_size * i
                );
            }
        }
    }

    fn share_dma_buffer(paddr: usize, size: usize) {
        let page_size = 0x1000;
        let pages = size / page_size;
        for i in 0..pages {
            let (ret0, _ret1) = psci_hvc_call(
                ARM_SMCCC_VENDOR_HYP_KVM_MEM_SHARE_FUNC_ID,
                paddr + page_size * i,
                1,
                0,
            );
            if ret0 != 0 {
                log::warn!(
                    "[virtio hal impl] cannot share 0x{:x}",
                    paddr + page_size * i
                );
            }
        }
    }
}
