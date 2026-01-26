use aarch64_pmuv3::pmuv3::{PmuCounter, PmuEvent};
use lazyinit::LazyInit;
use axplat::pmu::OverflowHandler;

const MAX_PMU_COUNTERS: usize = 32;

/// Per-CPU PMU manager.
///
/// This structure tracks the lifecycle of PMU counters on a per-CPU basis.
/// Each slot may or may not be initialized, hence the use of `Option`.
pub struct PmuManager {
    counters: [Option<PmuCounter>; MAX_PMU_COUNTERS],
    overflow_handlers: [Option<OverflowHandler>; MAX_PMU_COUNTERS],
}

/// Per-CPU lazy-initialized PMU manager.
///
/// The PMU is brought up on demand for each CPU.
#[percpu::def_percpu]
static PMU: LazyInit<PmuManager> = LazyInit::new();

/// Ensure that the per-CPU PMU manager is initialized.
///
/// This performs one-time initialization per CPU and returns
/// a mutable reference to the PMU manager.
#[inline]
unsafe fn ensure_pmu_inited() -> &'static mut PmuManager {
    let pmu = unsafe { PMU.current_ref_mut_raw() };
    pmu.call_once(|| PmuManager {
        counters: [const { None }; MAX_PMU_COUNTERS],
        overflow_handlers: [const { None }; MAX_PMU_COUNTERS],
    });
    pmu
}

/// Register an overflow handler for a PMU counter.
///
/// The handler will be invoked in interrupt context when the
/// corresponding counter overflows.
pub fn register_overflow_handler(index: u32, handler: OverflowHandler) -> bool {
    let idx = index as usize;

    if idx >= MAX_PMU_COUNTERS {
        return false;
    }

    unsafe {
        let pmu = PMU.current_ref_mut_raw();

        // Counter must be initialized first.
        if pmu.counters[idx].is_none() {
            return false;
        }

        pmu.overflow_handlers[idx] = Some(handler);
        true
    }
}

/// Initialize the cycle counter.
///
/// The cycle counter is mapped to the last PMU counter slot.
/// Returns `false` if the counter is already initialized or
/// if the underlying PMU is not supported.
pub fn init_cycle_counter(threshold: u64) -> bool {
    unsafe {
        let pmu_mgr = ensure_pmu_inited();

        let idx = MAX_PMU_COUNTERS - 1;

        // Do not overwrite an existing counter.
        if pmu_mgr.counters[idx].is_some() {
            return false;
        }

        let counter = PmuCounter::new_cycle_counter(threshold);

        // Bail out early if PMU is not supported on this CPU.
        if counter.check_pmu_support().is_err() {
            return false;
        }

        pmu_mgr.counters[idx] = Some(counter);
        true
    }
}

/// Initialize an event counter at the given index.
///
/// Event counters must not use the cycle counter slot.
/// Returns `false` on invalid index, duplicate initialization,
/// or unsupported PMU hardware.
pub fn init_event_counter(index: u32, threshold: u64, event: PmuEvent) -> bool {
    let idx = index as usize;

    // Reserve the last slot for the cycle counter.
    if idx >= MAX_PMU_COUNTERS - 1 {
        return false;
    }

    unsafe {
        let pmu_mgr = ensure_pmu_inited();

        // Do not overwrite an existing counter.
        if pmu_mgr.counters[idx].is_some() {
            return false;
        }

        let counter = PmuCounter::new_event_counter(index, threshold, event);

        // Check PMU availability before installing the counter.
        if counter.check_pmu_support().is_err() {
            return false;
        }

        pmu_mgr.counters[idx] = Some(counter);
        true
    }
}

/// Apply a mutable operation to a PMU counter if it exists.
///
/// Invalid indices or uninitialized counters are silently ignored.
/// This helper centralizes unsafe access and bounds checking.
#[inline]
unsafe fn with_counter_mut<F>(index: u32, f: F)
where
    F: FnOnce(&mut PmuCounter),
{
    if let Some(Some(counter)) =
        unsafe { PMU.current_ref_mut_raw().counters.get_mut(index as usize) }
    {
        f(counter);
    }
}

/// Enable the specified PMU counter.
///
/// This is a best-effort operation and is a no-op if the counter
/// does not exist or is not initialized.
pub fn enable(index: u32) {
    unsafe {
        with_counter_mut(index, |c| c.enable());
    }
}

/// Disable the specified PMU counter.
///
/// This is a best-effort operation and is a no-op if the counter
/// does not exist or is not initialized.
pub fn disable(index: u32) {
    unsafe {
        with_counter_mut(index, |c| c.disable());
    }
}

/// Query whether the specified PMU counter is enabled.
///
/// Returns `false` if the index is invalid or the counter
/// has not been initialized.
pub fn is_enabled(index: u32) -> bool {
    unsafe {
        PMU.current_ref_mut_raw()
            .counters
            .get(index as usize)
            .and_then(|c| c.as_ref())
            .map(|c| c.is_enabled())
            .unwrap_or(false)
    }
}

/// Handle PMU counter overflows.
///
/// This function scans all initialized counters and handles
/// every pending overflow. It must be called from the PMU IRQ
/// handler.
///
/// Returns `true` if at least one counter overflow was handled.
pub fn handle_overflows() -> bool {
    unsafe {
        let pmu = PMU.current_ref_mut_raw();
        let mut handled_any = false;

        for idx in 0..MAX_PMU_COUNTERS {
            // Copy the handler first (no borrow conflict)
            let handler = pmu.overflow_handlers[idx];

            let Some(counter) = pmu.counters[idx].as_mut() else {
                continue;
            };

            if counter.handle_overflow().is_ok() {
                handled_any = true;

                if let Some(h) = handler {
                    h();
                }
            }
        }

        handled_any
    }
}

/// Update the overflow threshold of a PMU counter.
///
/// This operation is ignored if the counter does not exist
/// or has not been initialized.
pub fn set_threshold(index: u32, threshold: u64) {
    unsafe {
        with_counter_mut(index, |c| c.set_threshold(threshold));
    }
}

/// Default implementation of [`axplat::pmu::PmuIf`]
#[macro_export]
macro_rules! pmu_if_impl {
    ($name:ident) => {
        struct $name;

        use axplat::pmu::OverflowHandler;

        #[impl_plat_interface]
        impl axplat::pmu::PmuIf for $name {
            /// Pmu interrupt handle func
            fn handle_overflows() -> bool{
                $crate::pmu::handle_overflows()
            }

            /// Register an overflow handler for a PMU counter.
            fn register_overflow_handler(index: u32, handler: OverflowHandler) -> bool{
                $crate::pmu::register_overflow_handler(index, handler)
            }
        }
    };
}