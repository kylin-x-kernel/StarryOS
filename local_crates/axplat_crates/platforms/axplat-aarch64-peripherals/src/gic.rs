// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 The axplat_crates Authors.
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.
// 
// This file has been modified by KylinSoft on 2025.

//! ARM Generic Interrupt Controller (GIC).

#[cfg(feature = "gicv2")]
use arm_gic_driver::v2::*;
#[cfg(feature = "gicv3")]
use arm_gic_driver::v3::*;
use axplat::{irq::{HandlerTable, IpiTarget, IrqHandler}};
use core::arch::asm;
use aarch64_cpu::registers::{DAIF, Readable};
use kspin::SpinNoIrq;
use lazyinit::LazyInit;

#[cfg(feature = "pmr")]
use core::sync::atomic::{AtomicBool, Ordering};

/// The maximum number of IRQs.
const MAX_IRQ_COUNT: usize = 1024;

static GIC: LazyInit<SpinNoIrq<Gic>> = LazyInit::new();

static TRAP_OP: LazyInit<TrapOp> = LazyInit::new();

static IRQ_HANDLER_TABLE: HandlerTable<MAX_IRQ_COUNT> = HandlerTable::new();

#[cfg(feature = "pmr")]
static GICC_PMR: LazyInit<usize> = LazyInit::new();

#[cfg(feature = "pmr")]
const PMR_OFFSET: usize = 0x4;

#[cfg(feature = "pmr")]
static GIC_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "pmr")]
#[inline]
pub fn set_gic_init_status(status: bool) {
    GIC_INITIALIZED.store(status, Ordering::SeqCst);
}

#[cfg(feature = "pmr")]
#[inline]
pub fn is_gic_initialized() -> bool {
    GIC_INITIALIZED.load(Ordering::SeqCst)
}

/// set trigger type of given IRQ
pub fn set_trigger(irq_num: usize, edge: bool) {
    trace!("GIC set trigger: {} {}", irq_num, edge);
    let intid = unsafe { IntId::raw(irq_num as u32) };
    let cfg = if edge {
        Trigger::Edge
    } else {
        Trigger::Level
    };
    GIC.lock().set_cfg(intid, cfg);
}

/// Enables or disables the given IRQ.
pub fn set_enable(irq: usize, enabled: bool) {
    trace!("GIC set enable: {irq} {enabled}");
    let intid = unsafe { IntId::raw(irq as u32) };
    let mut gic = GIC.lock();
    gic.set_irq_enable(intid, enabled);
    if !intid.is_private() {
        gic.set_cfg(intid, Trigger::Edge);
    }
}

/// Registers an IRQ handler for the given IRQ.
///
/// It also enables the IRQ if the registration succeeds. It returns `false`
/// if the registration failed.
pub fn register_handler(irq: usize, handler: IrqHandler) -> bool {
    if IRQ_HANDLER_TABLE.register_handler(irq, handler) {
        trace!("register handler IRQ {irq}");
        set_enable(irq, true);
        return true;
    }
    warn!("register handler for IRQ {irq} failed");
    false
}

/// Unregisters the IRQ handler for the given IRQ.
///
/// It also disables the IRQ if the unregistration succeeds. It returns the
/// existing handler if it is registered, `None` otherwise.
pub fn unregister_handler(irq: usize) -> Option<IrqHandler> {
    trace!("unregister handler IRQ {irq}");
    set_enable(irq, false);
    IRQ_HANDLER_TABLE.unregister_handler(irq)
}

/// Sets the priority for a specific interrupt request (IRQ).
///
/// This function configures the priority level for the given IRQ number. Lower
/// numerical values indicate higher priority. The priority value must be within
/// the valid range supported by the interrupt controller.
#[cfg(feature = "pmr")]
pub fn set_priority(irq: usize, priority: u8) {
    let intid = unsafe { IntId::raw(irq as u32) };
    let gic = GIC.lock();
    gic.set_priority(intid, priority);
}

#[cfg(not(feature = "pmr"))]
pub fn set_priority(_irq: usize, _priority: u8) {
    unreachable!()
}
/// Sets the priority mask for the CPU interface.
///
/// This function configures the priority mask register (PMR) which determines
/// the minimum priority level that can interrupt the processor. Interrupts with
/// priority lower than this mask will be ignored. This is useful for implementing
/// priority-based interrupt masking.
#[cfg(feature = "pmr")]
fn set_priority_mask(priority: u8) {
    unsafe { core::ptr::write_volatile((*GICC_PMR.get_unchecked()) as *mut u32, priority as u32);}
}   

/// Gets the current priority mask of the CPU interface.
///
/// This function reads the priority mask register (PMR) of the GIC CPU interface.
/// The PMR defines the minimum interrupt priority level that is allowed to be
/// signaled to the processor. Interrupts with a priority value numerically
/// greater than the PMR are masked and will not be delivered to the CPU.
#[cfg(feature = "pmr")]
fn get_priority_mask() -> u8 {
    unsafe { core::ptr::read_volatile((*GICC_PMR.get_unchecked()) as *const usize as *const u32) as u8 }
}   

/// Enter high-priority IRQ mode.
///
/// This does NOT fully enable interrupts.
/// It sets PMR to `0x80`, blocking normal IRQs (default priority `0xA0`),
/// while clearing DAIF.I to allow only higher-priority IRQs to preempt.
///
/// Commonly used to support high-priority IRQ nesting or as a
/// degraded form of `disable_irqs()` based on priority masking.
#[cfg(feature = "pmr")]
fn open_high_priority_irq_mode(){
    set_priority_mask(0x80);
    unsafe { asm!("msr daifclr, #2") };
}

/// Restore CPU-based IRQ masking.
///
/// This function masks IRQs via the CPU I bit (DAIF.I) and restores
/// the GIC priority mask to `0xFF`, removing the high-priority-only
/// restriction.
///
/// Unlike `open_high_priority_irq_mode()`, this does NOT enable IRQs.
/// It is intended to restore a CPU-masked baseline after temporarily
/// delegating IRQ control to PMR.
#[cfg(feature = "pmr")]
fn close_irq_and_restore_masking(){
    unsafe { asm!("msr daifset, #2") };
    set_priority_mask(0xff);
}

/// Handles the IRQ.
///
/// It is called by the common interrupt handler. It should look up in the
/// IRQ handler table and calls the corresponding handler. If necessary, it
/// also acknowledges the interrupt controller after handling.
#[cfg(feature = "gicv2")]
#[allow(unused_variables)]
pub fn handle_irq(_unused: usize, pmu_irq: usize) -> Option<usize> {
    let ack = TRAP_OP.ack();

    if ack.is_special() {
        return None;
    }

    let irq = match ack {
        Ack::Other(intid) => intid,
        Ack::SGI { intid, cpu_id: _ } => intid,
    }
    .to_u32() as usize;

    trace!("IRQ: {ack:?}");

    #[cfg(feature = "nmi-pmu")]
    if irq != pmu_irq{
        open_high_priority_irq_mode();
    }

    if !IRQ_HANDLER_TABLE.handle(irq) {
        debug!("Unhandled IRQ {ack:?}");
    }

    TRAP_OP.eoi(ack);
    if TRAP_OP.eoi_mode_ns() {
        TRAP_OP.dir(ack);
    }

    #[cfg(feature = "nmi-pmu")]
    if irq != pmu_irq{
        close_irq_and_restore_masking();
    }

    Some(irq)
}

#[cfg(feature = "gicv3")]
pub fn handle_irq(_unused: usize) -> Option<usize> {
    let ack = TRAP_OP.ack1();
    if ack.is_special() {
        return None;
    }

    trace!("Handling IRQ: {ack:?}");

    if !IRQ_HANDLER_TABLE.handle(ack.to_u32() as _) {
        warn!("Unhandled IRQ {:?}", ack);
    }

    TRAP_OP.eoi1(ack);
    if TRAP_OP.eoi_mode() {
        TRAP_OP.dir(ack);
    }

    Some(ack.to_u32() as usize)
}

/// Initializes GIC
#[cfg(feature = "gicv2")]
pub fn init_gic(gicd_base: axplat::mem::VirtAddr, gicc_base: axplat::mem::VirtAddr) {
    info!("Initialize GICv2...");
    let gicd_base = VirtAddr::new(gicd_base.into());
    let gicc_base = VirtAddr::new(gicc_base.into());
    #[cfg(feature = "pmr")]
    {
        GICC_PMR.init_once(usize::from(gicc_base) + PMR_OFFSET);
        set_gic_init_status(true);
    }
    let mut gic = unsafe { Gic::new(gicd_base, gicc_base, None) };
    gic.init();

    GIC.init_once(SpinNoIrq::new(gic));
    let cpu = GIC.lock().cpu_interface();
    TRAP_OP.init_once(cpu.trap_operations());
}

/// Initializes GIC
#[cfg(feature = "gicv3")]
pub fn init_gic(gicd_base: axplat::mem::VirtAddr, gicr_base: axplat::mem::VirtAddr) {
    info!("Initialize GICv3...");
    let gicd_base = VirtAddr::new(gicd_base.into());
    let gicr_base = VirtAddr::new(gicr_base.into());

    let mut gic = unsafe { Gic::new(gicd_base, gicr_base) };
    gic.init();
    GIC.init_once(SpinNoIrq::new(gic));
    let cpu = GIC.lock().cpu_interface();
    TRAP_OP.init_once(cpu.trap_operations());
}

/// Initializes GICC (for all CPUs).
///
/// It must be called after [`init_gic`].
#[cfg(feature = "gicv2")]
pub fn init_gicc() {
    debug!("Initialize GIC CPU Interface...");
    let mut cpu = GIC.lock().cpu_interface();
    cpu.init_current_cpu();
    cpu.set_eoi_mode_ns(false);
}

/// Initializes GICR (for all CPUs).
#[cfg(feature = "gicv3")]
pub fn init_gicr() {
    debug!("Initialize GIC CPU Interface...");
    let mut cpu = GIC.lock().cpu_interface();
    let _ = cpu.init_current_cpu();
    cpu.set_eoi_mode(false);
}

/// Sends an inter-processor interrupt (IPI) to the specified target CPU or all CPUs.
#[cfg(feature = "gicv2")]
pub fn send_ipi(irq_num: usize, target: IpiTarget) {
    match target {
        IpiTarget::Current { cpu_id: _ } => {
            GIC.lock()
                .send_sgi(IntId::sgi(irq_num as u32), SGITarget::Current);
        }
        IpiTarget::Other { cpu_id } => {
            let target_list = TargetList::new(&mut [cpu_id].into_iter());
            GIC.lock().send_sgi(
                IntId::sgi(irq_num as u32),
                SGITarget::TargetList(target_list),
            );
        }
        IpiTarget::AllExceptCurrent {
            cpu_id: _,
            cpu_num: _,
        } => {
            GIC.lock()
                .send_sgi(IntId::sgi(irq_num as u32), SGITarget::AllOther);
        }
    }
}

#[cfg(feature = "gicv3")]
pub fn send_ipi(irq_num: usize, target: IpiTarget) {
    match target {
        IpiTarget::Current { cpu_id: _ } => {
            GIC.lock().cpu_interface()
                .send_sgi(IntId::sgi(irq_num as u32), SGITarget::current());
            }
        IpiTarget::Other { cpu_id } => {
            let affinity = Affinity::from_mpidr(cpu_id as u64);
            let target_list = TargetList::new([affinity]);
            GIC.lock().cpu_interface().send_sgi(
                IntId::sgi(irq_num as u32),
                SGITarget::List(target_list),
            );
        }
        IpiTarget::AllExceptCurrent {
            cpu_id: _,
            cpu_num: _,
        } => {
            GIC.lock().cpu_interface()
                .send_sgi(IntId::sgi(irq_num as u32), SGITarget::All);
        }
    }
}

/// Allows the current CPU to respond to interrupts.
///
/// In AArch64, it unmasks IRQs by clearing the I bit in the `DAIF` register.
#[cfg(not(feature = "pmr"))]
#[inline]
pub fn enable_irqs() {
    // Default implementation: via DAIF register
    unsafe { asm!("msr daifclr, #2") };
}

/// Makes the current CPU ignore interrupts.
///
/// In AArch64, it masks IRQs by setting the I bit in the `DAIF` register.
#[cfg(not(feature = "pmr"))]
#[inline]
pub fn disable_irqs() {
    // Default implementation: via DAIF register
    unsafe { asm!("msr daifset, #2") };
}

/// Returns whether the current CPU is allowed to respond to interrupts.
///
/// In AArch64, it checks the I bit in the `DAIF` register.
#[cfg(not(feature = "pmr"))]
#[inline]
pub fn irqs_enabled() -> bool {
    !DAIF.matches_all(DAIF::I::Masked)
}

#[cfg(not(feature = "pmr"))]
#[inline]
pub fn local_irq_save_and_disable() -> usize {
    let flags: usize;
    // save `DAIF` flags
    unsafe { asm!("mrs {}, daif", out(reg) flags) };
    disable_irqs();
    flags
}

#[cfg(not(feature = "pmr"))]
#[inline]
pub fn local_irq_restore(flags: usize) {
    unsafe { asm!("msr daif, {}", in(reg) flags) };
}

/// Enables IRQ handling on the current CPU.
///
/// This clears the IRQ mask bit in DAIF and sets the GIC priority mask
/// to allow all interrupt priorities.
#[cfg(feature = "pmr")]
#[inline]
pub fn enable_irqs() {
    set_priority_mask(0xff);
    unsafe { asm!("msr daifclr, #2") };
}

/// Disables IRQ handling on the current CPU.
///
/// This masks IRQs via DAIF and raises the GIC priority mask to block
/// normal interrupts.
#[cfg(feature = "pmr")]
#[inline]
pub fn disable_irqs() {
    open_high_priority_irq_mode();
}

/// Returns whether IRQs are currently enabled on this CPU.
///
/// IRQs are considered enabled only if the DAIF IRQ mask is clear
/// and the GIC priority mask allows interrupts.
#[cfg(feature = "pmr")]
#[inline]
pub fn irqs_enabled() -> bool {
    !DAIF.matches_all(DAIF::I::Masked) && get_priority_mask() > 0xa0
}

/// Save the current interrupt state and disable IRQs.
///
/// This function may be called during early boot, before the GIC
/// is initialized. In that case, it falls back to manipulating the DAIF register
/// directly to mask IRQs.
///
/// After the GIC has been initialized, IRQ masking is performed via the GIC
/// priority mask (PMR) instead.
/// 
/// TODO: adapt gicv3
#[cfg(feature = "pmr")]
#[inline]
pub fn local_irq_save_and_disable() -> usize {
    if is_gic_initialized(){
        let pmr = get_priority_mask();
        set_priority_mask(0x80);
        pmr as usize
    }
    else{
        let flags: usize;
        // save `DAIF` flags, mask `I` bit (disable IRQs)
        unsafe { asm!("mrs {}, daif; msr daifset, #2", out(reg) flags) };
        flags
    }
}

/// Restore the interrupt state saved by [`local_irq_save_and_disable`].
///
/// If the GIC has already been initialized, the saved value is interpreted as a
/// GIC priority mask and restored via the PMR. Otherwise, the saved DAIF value
/// is written back directly (early boot path).
/// 
/// TODO: adapt gicv3
#[cfg(feature = "pmr")]
#[inline]
pub fn local_irq_restore(flags: usize) {
    if is_gic_initialized(){
        set_priority_mask(flags as u8);
    }
    else{
        unsafe { asm!("msr daif, {}", in(reg) flags) };
    }
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
                $crate::gic::set_enable(irq, enabled);
            }

            /// Registers an IRQ handler for the given IRQ.
            ///
            /// It also enables the IRQ if the registration succeeds. It returns `false`
            /// if the registration failed.
            fn register(irq: usize, handler: axplat::irq::IrqHandler) -> bool {
                $crate::gic::register_handler(irq, handler)
            }

            /// Unregisters the IRQ handler for the given IRQ.
            ///
            /// It also disables the IRQ if the unregistration succeeds. It returns the
            /// existing handler if it is registered, `None` otherwise.
            fn unregister(irq: usize) -> Option<axplat::irq::IrqHandler> {
                $crate::gic::unregister_handler(irq)
            }

            /// Handles the IRQ.
            ///
            /// It is called by the common interrupt handler. It should look up in the
            /// IRQ handler table and calls the corresponding handler. If necessary, it
            /// also acknowledges the interrupt controller after handling.
            fn handle(irq: usize) -> Option<usize> {
                let pmu_irq = crate::config::devices::PMU_IRQ;
                $crate::gic::handle_irq(irq, pmu_irq)
            }

            /// Sends an inter-processor interrupt (IPI) to the specified target CPU or all CPUs.
            fn send_ipi(irq_num: usize, target: axplat::irq::IpiTarget) {
                $crate::gic::send_ipi(irq_num, target);
            }

            /// Sets the priority for a specific interrupt request (IRQ).
            fn set_priority(irq: usize, priority: u8) {
                $crate::gic::set_priority(irq, priority);
            }

            /// Save irq status and disable
            fn local_irq_save_and_disable() -> usize {
                $crate::gic::local_irq_save_and_disable()
            }

            /// Restore irq status
            fn local_irq_restore(flag: usize) {
                $crate::gic::local_irq_restore(flag);
            }

            /// Allows the current CPU to respond to interrupts.
            fn enable_irqs(){
                $crate::gic::enable_irqs();
            }

            /// Makes the current CPU ignore interrupts.
            fn disable_irqs(){
                $crate::gic::disable_irqs();
            }

            /// Returns whether the current CPU is allowed to respond to interrupts.
            fn irqs_enabled() -> bool {
                $crate::gic::irqs_enabled()
            }
        }
    };
}
