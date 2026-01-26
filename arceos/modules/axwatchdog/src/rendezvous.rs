use core::sync::atomic::{AtomicUsize, Ordering};

use axhal::percpu::this_cpu_id;

/// Rendezvous phases.
#[repr(usize)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum Phase {
    /// No rendezvous in progress.
    Idle      = 0,
    /// Triggered: all CPUs must enter NMI and mark arrived.
    Triggered = 1,
    /// Dump done: non-cause CPUs can stop spinning if desired.
    DumpDone  = 2,
}

impl Phase {
    #[inline]
    fn load() -> Self {
        match PHASE.load(Ordering::Acquire) {
            0 => Phase::Idle,
            1 => Phase::Triggered,
            2 => Phase::DumpDone,
            _ => Phase::Idle,
        }
    }
}

static PHASE: AtomicUsize = AtomicUsize::new(Phase::Idle as usize);

/// The CPU id which detected the failure and triggered the rendezvous.
static CAUSE_CPU: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Per-cpu arrived bitmap stored in an AtomicUsize where bit i means CPU i arrived.
static ARRIVED_BITMAP: AtomicUsize = AtomicUsize::new(0);

#[inline]
pub fn is_triggered() -> bool {
    Phase::load() == Phase::Triggered
}

#[inline]
pub fn is_dump_done() -> bool {
    Phase::load() == Phase::DumpDone
}

/// Try to trigger rendezvous.
///
/// Returns `true` if this CPU became the *cause CPU*.
#[inline]
pub fn try_trigger() {
    let cpu = this_cpu_id();
    if PHASE
        .compare_exchange(
            Phase::Idle as usize,
            Phase::Triggered as usize,
            Ordering::AcqRel,
            Ordering::Relaxed,
        )
        .is_ok()
    {
        CAUSE_CPU.store(cpu, Ordering::Release);
    }
}

#[inline]
pub fn cause_cpu() -> Option<usize> {
    if Phase::load() == Phase::Idle {
        return None;
    }
    let cpu = CAUSE_CPU.load(Ordering::Acquire);
    (cpu != usize::MAX).then_some(cpu)
}

/// Mark current cpu as arrived.
#[inline]
pub fn mark_arrived() {
    let id = this_cpu_id();
    if id >= usize::BITS as usize {
        // Cannot represent this CPU in the bitmap without overflowing the shift.
        return;
    }
    ARRIVED_BITMAP.fetch_or(1usize << id, Ordering::AcqRel);
}

#[inline]
pub fn arrived_bitmap() -> usize {
    ARRIVED_BITMAP.load(Ordering::Acquire)
}

#[inline]
pub fn all_arrived_mask() -> usize {
    let n = axconfig::plat::CPU_NUM;
    if n >= usize::BITS as usize {
        usize::MAX
    } else {
        (1usize << n) - 1
    }
}

/// Busy-wait until all CPUs have arrived.
///
/// This is a *strong* rendezvous: no timeout.
#[inline]
pub fn wait_all_arrived_strong() {
    let expect = all_arrived_mask();
    while arrived_bitmap() & expect != expect {
        core::hint::spin_loop();
    }
}

/// Mark dump done so other CPUs can release from spinning.
#[inline]
pub fn mark_dump_done() {
    PHASE.store(Phase::DumpDone as usize, Ordering::Release);
}

/// Reset rendezvous state.
///
/// Note: in your intended flow, other CPUs may keep spinning in NMI forever.
/// If you want them to be able to return from NMI, call this after
/// `mark_dump_done()` and after ensuring they observed it.
#[inline]
pub fn reset() {
    ARRIVED_BITMAP.store(0, Ordering::Release);
    CAUSE_CPU.store(usize::MAX, Ordering::Release);
    PHASE.store(Phase::Idle as usize, Ordering::Release);
}
