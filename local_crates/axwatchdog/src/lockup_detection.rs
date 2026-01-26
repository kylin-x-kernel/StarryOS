use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::watchdog_task::WatchdogTask;

/// Default softlockup threshold in nanoseconds (20 seconds).
pub const DEFAULT_SOFTLOCKUP_THRESH_NS: u64 = 20_000_000_000;

/// Default hardlockup threshold in nanoseconds (10 seconds).
pub const DEFAULT_HARDLOCKUP_THRESH_NS: u64 = 10_000_000_000;

#[repr(C, align(64))]
pub struct LockupDetection {
    // === Softlockup Detection ===
    /// Timestamp when watchdog thread last ran (nanoseconds).
    /// Updated by watchdog thread, checked by timer interrupt.
    soft_timestamp: AtomicU64,

    // === Hardlockup Detection ===
    /// Timer interrupt counter (incremented in timer interrupt).
    hrtimer_interrupts: AtomicU32,
    /// Saved hrtimer_interrupts value from last NMI check.
    hrtimer_interrupts_saved: AtomicU32,
}

impl LockupDetection {
    /// Create a new LockupDetection instance.
    pub const fn new() -> Self {
        Self {
            soft_timestamp: AtomicU64::new(0),
            hrtimer_interrupts: AtomicU32::new(0),
            hrtimer_interrupts_saved: AtomicU32::new(0),
        }
    }

    // =========================================================================
    // Softlockup detection
    // =========================================================================

    /// Update the soft timestamp (called by watchdog thread).
    ///
    /// The watchdog thread should call this every time it gets scheduled.
    #[inline]
    pub fn touch_softlockup(&self, timestamp_ns: u64) {
        self.soft_timestamp.store(timestamp_ns, Ordering::Release);
    }

    /// Get the soft timestamp.
    #[inline]
    pub fn soft_timestamp(&self) -> u64 {
        self.soft_timestamp.load(Ordering::Acquire)
    }

    /// Check for softlockup condition.
    ///
    /// Call this from timer interrupt context.
    /// Returns true if softlockup is detected.
    #[inline]
    pub fn check_softlockup(&self, now_ns: u64, threshold_ns: u64) -> bool {
        let last = self.soft_timestamp.load(Ordering::Acquire);
        if last == 0 {
            // Not yet initialized
            return false;
        }
        now_ns.saturating_sub(last) > threshold_ns
    }

    // =========================================================================
    // Hardlockup detection
    // =========================================================================

    /// Increment hrtimer interrupt counter (called from timer interrupt).
    #[inline]
    pub fn timer_tick(&self) {
        self.hrtimer_interrupts.fetch_add(1, Ordering::Release);
    }

    /// Get current hrtimer interrupt count.
    #[inline]
    pub fn hrtimer_interrupts(&self) -> u32 {
        self.hrtimer_interrupts.load(Ordering::Acquire)
    }

    /// Check for hardlockup condition (called from NMI).
    ///
    /// Returns true if hardlockup is detected (timer interrupts stopped).
    #[inline]
    pub fn check_hardlockup(&self) -> bool {
        let current = self.hrtimer_interrupts.load(Ordering::Acquire);
        let saved = self.hrtimer_interrupts_saved.load(Ordering::Acquire);

        // Update saved value for next check
        self.hrtimer_interrupts_saved
            .store(current, Ordering::Release);
        // If counts are equal, no timer interrupts occurred
        current == saved && current != 0
    }
}

#[percpu::def_percpu]
pub static LOCKUP_DETECTION: LockupDetection = LockupDetection::new();

/// Touch softlockup timestamp (called from watchdog thread).
#[inline]
pub fn touch_softlockup(timestamp_ns: u64) {
    unsafe {
        LOCKUP_DETECTION
            .current_ref_mut_raw()
            .touch_softlockup(timestamp_ns);
    }
}

/// Timer tick (called from timer interrupt).
#[inline]
pub fn timer_tick() {
    unsafe {
        LOCKUP_DETECTION.current_ref_mut_raw().timer_tick();
    }
}

/// Check softlockup of a CPU.
#[inline]
pub fn check_softlockup(now_ns: u64) -> bool {
    unsafe {
        LOCKUP_DETECTION
            .current_ref_mut_raw()
            .check_softlockup(now_ns, DEFAULT_SOFTLOCKUP_THRESH_NS)
    }
}

pub fn register_hardlockup_detection_task() {
    let task: &'static LockupDetection = unsafe { LOCKUP_DETECTION.current_ref_raw() };
    crate::watchdog_task::register_watchdog_task(task);
}

impl WatchdogTask for LockupDetection {
    fn name(&self) -> &str {
        "HardLockupDetection"
    }

    fn check(&self) -> bool {
        !self.check_hardlockup()
    }
}
