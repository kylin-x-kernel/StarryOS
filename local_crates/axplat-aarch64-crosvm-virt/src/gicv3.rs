// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 WeiKang Guo <guoweikang.kernel@gmail.com
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.

use aarch64_cpu::registers::*;
use arm_gic::gicv3::*;
use core::sync::atomic::{AtomicBool, Ordering};
use kspin::SpinNoIrq;
use log::*;
use core::arch::asm;

use crate::config::plat::CPU_NUM;
use axplat::irq::{HandlerTable, IrqHandler};
use axplat::mem::VirtAddr;

static GICD_INIT: AtomicBool = AtomicBool::new(false);
const MAX_IRQ_COUNT: usize = 1024;
static IRQ_HANDLER_TABLE: HandlerTable<MAX_IRQ_COUNT> = HandlerTable::new();

struct GicV3Wrapper {
    inner: GicV3,
}

unsafe impl Send for GicV3Wrapper {}
unsafe impl Sync for GicV3Wrapper {}

static GIC_V3S: [SpinNoIrq<Option<GicV3Wrapper>>; CPU_NUM] =
    [const { SpinNoIrq::new(None) }; CPU_NUM];

#[inline]
fn get_current_cpu_id() -> usize {
    let mpidr_el1: usize;
    unsafe {
        core::arch::asm!("mrs {}, MPIDR_EL1", out(reg) mpidr_el1);
    }
    mpidr_el1 & 0xff // 获取 Aff0 字段，即 CPU ID
}

/// Initializes GIC
pub fn init_gic(gicd_base: VirtAddr, gicr_base: VirtAddr) {
    info!(
        "Initialize GICv3... from 0x{:x} 0x{:x}",
        gicd_base.as_usize(),
        gicr_base.as_usize()
    );
    const GICR_RD_OFFSET: usize = 0x20000;
    const GICR_TYPER_HI_OFFSET: usize = 0x0008;

    let mut gic_v3_lock = GIC_V3S[get_current_cpu_id()].lock();

    let mpidr_aff: u64 = aarch64_cpu::registers::MPIDR_EL1.get() & 0xffffff;
    let mut cur_gicr_base: usize = gicr_base.as_usize();
    loop {
        let gicr_typer_aff: u64 = unsafe {
            core::ptr::read_volatile((cur_gicr_base + GICR_TYPER_HI_OFFSET) as *const u64)
        };
        trace!("gicr_typer_aff: 0x{:x?}", gicr_typer_aff);
        if mpidr_aff == gicr_typer_aff >> 32 {
            info!("cur_gicr_base: 0x{:x}", cur_gicr_base);
            break;
        }
        cur_gicr_base += GICR_RD_OFFSET;
    }

    let mut v3: GicV3 = unsafe { GicV3::new(gicd_base.as_mut_ptr_of(), cur_gicr_base as *mut u64) };
    if !GICD_INIT.load(Ordering::SeqCst) {
        v3.setup();
        GICD_INIT.store(true, Ordering::SeqCst);
    }
    // 初始化 CPU 接口
    v3.init_cpu();
    *gic_v3_lock = Some(GicV3Wrapper { inner: v3 });
}

/// set trigger type of given IRQ
pub fn set_trigger(irq_num: usize, edge: bool) {
    trace!("GIC set trigger: {}  edge: {}", irq_num, edge);
    let mut gic_v3_lock = GIC_V3S[get_current_cpu_id()].lock();
    let gic_v3 = &mut gic_v3_lock.as_mut().unwrap().inner;
    let intid = IntId::from(irq_num as u32);
    let cfg = if edge { Trigger::Edge } else { Trigger::Level };

    gic_v3.set_trigger(intid, cfg);
}

/// Enables or disables the given IRQ.
pub fn set_enable(irq_num: usize, enabled: bool) {
    trace!("GIC set enable: {} {}", irq_num, enabled);
    let mut gic_v3_lock = GIC_V3S[get_current_cpu_id()].lock();
    let gic_v3 = &mut gic_v3_lock.as_mut().unwrap().inner;
    if enabled {
        gic_v3.enable_interrupt(IntId::from(irq_num as u32), true);
    } else {
        gic_v3.enable_interrupt(IntId::from(irq_num as u32), false);
    }
}

/// Registers an IRQ handler for the given IRQ.
///
/// It also enables the IRQ if the registration succeeds. It returns `false`
/// if the registration failed.
pub fn register_handler(irq_num: usize, handler: IrqHandler) -> bool {
    if IRQ_HANDLER_TABLE.register_handler(irq_num, handler) {
        trace!("register handler IRQ {}", irq_num);
        set_enable(irq_num, true);
        return true;
    }
    false
}

/// Unregisters the IRQ handler for the given IRQ.
///
/// It also disables the IRQ if the unregistration succeeds. It returns the
/// existing handler if it is registered, `None` otherwise.
pub fn unregister_handler(irq_num: usize) -> Option<IrqHandler> {
    trace!("unregister handler IRQ {}", irq_num);
    set_enable(irq_num, false);
    IRQ_HANDLER_TABLE.unregister_handler(irq_num)
}

fn end_of_interrupt(irq: usize) {
    GicV3::end_interrupt(IntId::from(irq as u32));
}

fn get_and_acknowledge_interrupt() -> usize {
    let irq = u32::from(GicV3::get_and_acknowledge_interrupt().unwrap()) as usize;
    return irq;
}

pub fn send_ipi(irq: usize, target: axplat::irq::IpiTarget) {
    use arm_gic::gicv3::SgiTarget;

    let sgi_intid = IntId::from(irq as u32);

    match target {
        axplat::irq::IpiTarget::AllExceptCurrent { .. } => {
            GicV3::send_sgi(sgi_intid, SgiTarget::All);
        }
        _ => {
            // 其他情况暂不处理
            unimplemented!();
        }
    }
}

#[allow(dead_code)]
fn test_manual_trigger() {
    let gicd_base = 0xffff00003fff0000 as usize; // GICD base address

    info!("=== Manual Trigger Test ===");

    unsafe {
        // 手动触发 32 号中断
        core::ptr::write_volatile((gicd_base + 0x200 + 1 * 4) as *mut u32, 0x1);

        let ispendr = core::ptr::read_volatile((gicd_base + 0x200 + 1 * 4) as *const u32);
        info!("Manual trigger: ISPENDR = {:#x}", ispendr);
    }

    // 等待几个时钟周期
    for _ in 0..1000 {
        core::hint::spin_loop();
    }

    info!("Did interrupt fire? Check handler logs");
}

#[allow(dead_code)]
fn debug_irq_32() {
    let irq = 32;
    let gicd_base = 0xffff00003fff0000 as usize; // GICD base address

    unsafe {
        // === 原有检查 ===
        let isenabler =
            core::ptr::read_volatile((gicd_base + 0x100 + (irq / 32) * 4) as *const u32);
        info!(
            "GICD_ISENABLER[1]: {:#x}, bit 0: {}",
            isenabler,
            (isenabler >> (irq % 32)) & 1
        );

        let ispendr = core::ptr::read_volatile((gicd_base + 0x200 + (irq / 32) * 4) as *const u32);
        info!(
            "GICD_ISPENDR[1]: {:#x}, bit 0: {}",
            ispendr,
            (ispendr >> (irq % 32)) & 1
        );

        let ipriorityr = core::ptr::read_volatile((gicd_base + 0x400 + irq) as *const u8);
        info!("GICD_IPRIORITYR[32]: {:#x}", ipriorityr);

        let irouter = core::ptr::read_volatile((gicd_base + 0x6000 + irq * 8) as *const u64);
        info!("GICD_IROUTER[32]: {:#x}", irouter);

        // === 新增检查 ===

        // 1. GICD_CTLR - 全局使能状态
        let gicd_ctlr = core::ptr::read_volatile(gicd_base as *const u32);
        info!(
            "GICD_CTLR: {:#x} (EnableGrp0:{}, EnableGrp1:{})",
            gicd_ctlr,
            gicd_ctlr & 1,
            (gicd_ctlr >> 1) & 1
        );

        // 2. GICD_IGROUPR - Group 配置
        let igroupr = core::ptr::read_volatile((gicd_base + 0x80 + (irq / 32) * 4) as *const u32);
        info!(
            "GICD_IGROUPR[1]: {:#x}, bit 0: {} (0=Group0, 1=Group1)",
            igroupr,
            (igroupr >> (irq % 32)) & 1
        );

        // 3. GICD_IGRPMODR - Group Modifier (Secure/Non-secure)
        let igrpmodr = core::ptr::read_volatile((gicd_base + 0xD00 + (irq / 32) * 4) as *const u32);
        info!(
            "GICD_IGRPMODR[1]: {:#x}, bit 0: {}",
            igrpmodr,
            (igrpmodr >> (irq % 32)) & 1
        );

        // 4. GICD_ICFGR - 触发类型 (边沿/电平)
        let icfgr = core::ptr::read_volatile((gicd_base + 0xC00 + (irq / 16) * 4) as *const u32);
        let cfg_shift = ((irq % 16) * 2) + 1;
        info!(
            "GICD_ICFGR[2]: {:#x}, bit {}: {} (0=level, 1=edge)",
            icfgr,
            cfg_shift,
            (icfgr >> cfg_shift) & 1
        );

        // 5. GICD_ISACTIVER - 激活状态
        let isactiver =
            core::ptr::read_volatile((gicd_base + 0x300 + (irq / 32) * 4) as *const u32);
        info!(
            "GICD_ISACTIVER[1]: {:#x}, bit 0: {}",
            isactiver,
            (isactiver >> (irq % 32)) & 1
        );

        // 6. 尝试写入并读回验证
        info!("=== Write Test ===");
        let old_val = core::ptr::read_volatile((gicd_base + 0x100 + (irq / 32) * 4) as *const u32);
        core::ptr::write_volatile((gicd_base + 0x100 + (irq / 32) * 4) as *mut u32, 0x1);
        let new_val = core::ptr::read_volatile((gicd_base + 0x100 + (irq / 32) * 4) as *const u32);
        info!(
            "Write 0x1 to ISENABLER[1]: before={:#x}, after={:#x}",
            old_val, new_val
        );

        // 7. ICC 寄存器状态
        let mut pmr: u64;
        let mut igrpen1: u64;
        let mut ctlr: u64;
        core::arch::asm!(
            "mrs {0}, S3_0_C4_C6_0",  // ICC_PMR_EL1
            "mrs {1}, S3_0_C12_C12_7", // ICC_IGRPEN1_EL1
            "mrs {2}, S3_0_C12_C12_4", // ICC_CTLR_EL1
            out(reg) pmr,
            out(reg) igrpen1,
            out(reg) ctlr,
        );
        info!("ICC_PMR_EL1: {:#x}", pmr);
        info!("ICC_IGRPEN1_EL1: {:#x}", igrpen1);
        info!("ICC_CTLR_EL1: {:#x}", ctlr);

        // 8. 当前 CPU affinity
        let mpidr = aarch64_cpu::registers::MPIDR_EL1.get();
        info!("Current CPU MPIDR: {:#x}", mpidr & 0xffffff);
    }
}

pub fn handle_irq(_unused: usize) -> Option<usize> {
    let irq = get_and_acknowledge_interrupt();

    if !IRQ_HANDLER_TABLE.handle(irq as u32 as _) {
        debug!("Unhandled IRQ {:?}", irq);
    }

    if irq <= 1019 {
        end_of_interrupt(irq);
    }

    Some(irq)
}

/// Allows the current CPU to respond to interrupts.
///
/// In AArch64, it unmasks IRQs by clearing the I bit in the `DAIF` register.
#[inline]
pub fn enable_irqs() {
    // Default implementation: via DAIF register
    unsafe { asm!("msr daifclr, #2") };
}

/// Makes the current CPU ignore interrupts.
///
/// In AArch64, it masks IRQs by setting the I bit in the `DAIF` register.
#[inline]
pub fn disable_irqs() {
    // Default implementation: via DAIF register
    unsafe { asm!("msr daifset, #2") };
}

/// Returns whether the current CPU is allowed to respond to interrupts.
///
/// In AArch64, it checks the I bit in the `DAIF` register.
#[inline]
pub fn irqs_enabled() -> bool {
    !DAIF.matches_all(DAIF::I::Masked)
}

#[inline]
pub fn local_irq_save_and_disable() -> usize {
    let flags: usize;
    // save `DAIF` flags
    unsafe { asm!("mrs {}, daif", out(reg) flags) };
    disable_irqs();
    flags
}

#[inline]
pub fn local_irq_restore(flags: usize) {
    unsafe { asm!("msr daif, {}", in(reg) flags) };
}

/// Default implementation of [`axplat::irq::IrqIf`] using the GIC.
#[macro_export]
macro_rules! irq_if_impl {
    ($name:ident) => {
        struct $name;

        #[impl_plat_interface]
        impl axplat::irq::IrqIf for $name {
            /// Enables or disables the given IRQ.
            fn set_enable(irq: usize, enabled: bool) {
                $crate::gicv3::set_enable(irq, enabled);
            }

            /// Registers an IRQ handler for the given IRQ.
            ///
            /// It also enables the IRQ if the registration succeeds. It returns `false`
            /// if the registration failed.
            fn register(irq: usize, handler: axplat::irq::IrqHandler) -> bool {
                $crate::gicv3::register_handler(irq, handler)
            }

            /// Unregisters the IRQ handler for the given IRQ.
            ///
            /// It also disables the IRQ if the unregistration succeeds. It returns the
            /// existing handler if it is registered, `None` otherwise.
            fn unregister(irq: usize) -> Option<axplat::irq::IrqHandler> {
                $crate::gicv3::unregister_handler(irq)
            }

            /// Handles the IRQ.
            ///
            /// It is called by the common interrupt handler. It should look up in the
            /// IRQ handler table and calls the corresponding handler. If necessary, it
            /// also acknowledges the interrupt controller after handling.
            fn handle(irq: usize) -> Option<usize> {
                $crate::gicv3::handle_irq(irq)
            }

            /// Sends an inter-processor interrupt (IPI) to the specified target CPU or all CPUs.
            fn send_ipi(irq_num: usize, target: axplat::irq::IpiTarget) {
                $crate::gicv3::send_ipi(irq_num, target);
            }

            /// Sets the priority for a specific interrupt request (IRQ).
            /// Not used in crsovm
            fn set_priority(_irq: usize, _priority: u8) {
                todo!()
            }

            /// Save irq status and disable
            fn local_irq_save_and_disable() -> usize {
                $crate::gicv3::local_irq_save_and_disable()
            }

            /// Restore irq status
            fn local_irq_restore(flag: usize) {
                $crate::gicv3::local_irq_restore(flag);
            }

            /// Allows the current CPU to respond to interrupts.
            fn enable_irqs(){
                $crate::gicv3::enable_irqs();
            }

            /// Makes the current CPU ignore interrupts.
            fn disable_irqs(){
                $crate::gicv3::disable_irqs();
            }

            /// Returns whether the current CPU is allowed to respond to interrupts.
            fn irqs_enabled() -> bool {
                $crate::gicv3::irqs_enabled()
            }
        }
    };
}
