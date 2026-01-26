// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 WeiKang Guo <guoweikang.kernel@gmail.com
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.

use axplat::mem::{VirtAddr, pa, phys_to_virt};
use fdtree_rs::{InterruptController, LinuxFdt};
use log::*;
use spin::Once;

pub static FDT: Once<LinuxFdt> = Once::new();

pub(crate) fn init_fdt(fdt_paddr: VirtAddr) {
    info!("FDT addr is: {:x}", fdt_paddr.as_usize());
    let fdt = unsafe {
        LinuxFdt::from_ptr(fdt_paddr.as_usize() as *const u8).expect("Failed to parse FDT")
    };

    FDT.call_once(|| fdt);

    dice_reg();
}

#[allow(dead_code)]
pub(crate) fn interrupt_controller() -> Option<InterruptController<'static, 'static>> {
    let fdt = FDT.get().expect("FDT is not initialized");
    match fdt.interrupt_controller() {
        Some(ic_node) => Some(ic_node),
        None => {
            warn!("No interrupt-controller node found in FDT");
            None
        }
    }
}

pub fn dice_reg() -> Option<(VirtAddr, usize)> {
    let dice = FDT.get().unwrap().dice();
    if let Some(dice_node) = dice {
        info!("Found DICE node in FDT");
        for reg in dice_node.regions().expect("DICE regions") {
            info!(
                "DICE region: addr=0x{:x}, size=0x{:x}",
                reg.starting_address as usize, reg.size
            );

            let va = phys_to_virt(pa!(reg.starting_address as usize));
            // test read dice memory
            unsafe {
                let test_ptr = va.as_mut_ptr();
                let _ = test_ptr.read_volatile();
            }
            return Some((va, reg.size));
        }
    }
    None
}
