// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 WeiKang Guo <guoweikang.kernel@gmail.com
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.

use log::*;

use axplat::init::InitIf;
#[allow(unused_imports)]
use crate::config::devices::{GICR_PADDR, GICD_PADDR, TIMER_IRQ, UART_IRQ, UART_PADDR, RTC_PADDR};
use crate::config::plat::PSCI_METHOD;
use axplat::mem::{pa, phys_to_virt};

use crate::serial::*;

struct InitIfImpl;

#[impl_plat_interface]
impl InitIf for InitIfImpl {
    /// Initializes the platform at the early stage for the primary core.
    ///
    /// This function should be called immediately after the kernel has booted,
    /// and performed earliest platform configuration and initialization (e.g.,
    /// early console, clocking).
    fn init_early(_cpu_id: usize, dtb: usize) {
        boot_print_str("[boot] platform init early\r\n");
        crate::mem::init_early(dtb);
        axcpu::init::init_trap();
        axplat_aarch64_peripherals::ns16550a::init_early(phys_to_virt(pa!(UART_PADDR)));
        axplat_aarch64_peripherals::psci::init(PSCI_METHOD);
        axplat_aarch64_peripherals::generic_timer::init_early();
        #[cfg(feature = "rtc")]
        axplat_aarch64_peripherals::pl031::init_early(phys_to_virt(pa!(RTC_PADDR)));
    }

    /// Initializes the platform at the early stage for secondary cores.
    #[cfg(feature = "smp")]
    fn init_early_secondary(_cpu_id: usize) {
        axcpu::init::init_trap();
    }

    /// Initializes the platform at the later stage for the primary core.
    ///
    /// This function should be called after the kernel has done part of its
    /// initialization (e.g, logging, memory management), and finalized the rest of
    /// platform configuration and initialization.
    fn init_later(cpu_id: usize, dtb: usize) {
        // now we could use logging
        info!("cpu_id {}", cpu_id);
        crate::fdt::init_fdt(phys_to_virt(pa!(dtb)));

        #[cfg(feature = "irq")]
        {
            // hack: use our gicv3 implementation to init gic
            // use arm-gic-driver crate will cause databort
            crate::gicv3::init_gic(
                phys_to_virt(pa!(GICD_PADDR)),
                phys_to_virt(pa!(GICR_PADDR)),
            );
            // crosvm set uart as edge trigger irq
            info!("set UART IRQ {} as edge trigger", UART_IRQ);
            crate::gicv3::set_trigger(UART_IRQ, true);
            axplat_aarch64_peripherals::generic_timer::enable_irqs(TIMER_IRQ);
        }
    }

    /// Initializes the platform at the later stage for secondary cores.
    #[cfg(feature = "smp")]
    fn init_later_secondary(_cpu_id: usize) {
        #[cfg(feature = "irq")]
        {
            crate::gicv3::init_gic(
                phys_to_virt(pa!(GICD_PADDR)),
                phys_to_virt(pa!(GICR_PADDR)),
            );
            axplat_aarch64_peripherals::generic_timer::enable_irqs(TIMER_IRQ);
        }
    }
}
