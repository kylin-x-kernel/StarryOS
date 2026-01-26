// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 WeiKang Guo <guoweikang.kernel@gmail.com
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.

use fdtree_rs::LinuxFdt;
use spin::Once;

use axplat::mem::{MemIf, PhysAddr, RawRange, VirtAddr, pa, va};
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::config::devices::MMIO_RANGES;
use crate::config::plat::{PHYS_MEMORY_BASE, PHYS_MEMORY_SIZE, PHYS_VIRT_OFFSET};

// default FDT memory size 2MB
const FDT_MEM_SIZE: usize = 0x20_0000;
static FDT_MEM_BASE: AtomicUsize = AtomicUsize::new(0);
static FDT_MEM: Once<[RawRange; 2]> = Once::new();

static DICE_MEM_BASE: AtomicUsize = AtomicUsize::new(0);
static DICE_MEM_SIZE: AtomicUsize = AtomicUsize::new(0);

/// Initializes the reserved memory physical address.
pub(crate) fn init_early(fdt_paddr: usize) {
    FDT_MEM_BASE.store(fdt_paddr, Ordering::SeqCst);
    let fdt = unsafe {
        LinuxFdt::from_ptr(fdt_paddr as *const u8).expect("Failed to parse FDT")
    };

    fdt.dice().map(|dice_node| {
        let dice = dice_node;
        for reg in dice.regions().expect("DICE regions") {
            DICE_MEM_BASE.store(reg.starting_address as usize, Ordering::SeqCst);
            DICE_MEM_SIZE.store(reg.size as usize, Ordering::SeqCst);
            break;
        }
    });
}

struct MemIfImpl;

#[impl_plat_interface]
impl MemIf for MemIfImpl {
    /// Returns all physical memory (RAM) ranges on the platform.
    ///
    /// All memory ranges except reserved ranges (including the kernel loaded
    /// range) are free for allocation.
    fn phys_ram_ranges() -> &'static [RawRange] {
        &[(PHYS_MEMORY_BASE, PHYS_MEMORY_SIZE)]
    }

    /// Returns all reserved physical memory ranges on the platform.
    ///
    /// Reserved memory can be contained in [`phys_ram_ranges`], they are not
    /// allocatable but should be mapped to kernel's address space.
    ///
    /// Note that the ranges returned should not include the range where the
    /// kernel is loaded.
    fn reserved_phys_ram_ranges() -> &'static [RawRange] {
        FDT_MEM
            .call_once(|| {
                [
                    (FDT_MEM_BASE.load(Ordering::Relaxed), FDT_MEM_SIZE),
                    (
                        DICE_MEM_BASE.load(Ordering::Relaxed),
                        DICE_MEM_SIZE.load(Ordering::Relaxed),
                    ),
                ]
            })
            .as_ref()
    }

    /// Returns all device memory (MMIO) ranges on the platform.
    fn mmio_ranges() -> &'static [RawRange] {
        &MMIO_RANGES
    }

    /// Translates a physical address to a virtual address.
    fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
        va!(paddr.as_usize() + PHYS_VIRT_OFFSET)
    }

    /// Translates a virtual address to a physical address.
    fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
        pa!(vaddr.as_usize() - PHYS_VIRT_OFFSET)
    }

    /// Returns the kernel address space base virtual address and size.
    fn kernel_aspace() -> (VirtAddr, usize) {
        (
            va!(crate::config::plat::KERNEL_ASPACE_BASE),
            crate::config::plat::KERNEL_ASPACE_SIZE,
        )
    }
}
