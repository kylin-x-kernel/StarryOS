use alloc::{boxed::Box, sync::Arc};
use core::sync::atomic::{AtomicUsize, Ordering};

use axhal::percpu::this_cpu_id;

use crate::WeakAxTaskRef;

/// Max number of task weak refs tracked per CPU for watchdog/NMI dumping.
///
/// This is a best-effort debug facility: if it fills up, we will simply drop
/// new records (and dumps may miss some tasks).
const GLOBAL_TASK_QUEUE_SLOTS: usize = 4096;

/// Lock-free per-CPU task registry for watchdog/NMI dumping.
///
/// Safety / design notes:
/// - Writers (task creation + GC) run on the owning CPU, but NMI may read any CPU.
/// - Each slot stores a raw pointer to a heap-allocated `WeakAxTaskRef` (usize).
/// - Readers snapshot the pointers with `Acquire` loads, dereference, and call
///   `upgrade()` (which is internally atomic).
/// - GC sweeps invalid weak refs and frees their boxes.
struct GlobalTaskRegistry {
    slots: [[AtomicUsize; GLOBAL_TASK_QUEUE_SLOTS]; axconfig::plat::CPU_NUM],
}

impl GlobalTaskRegistry {
    const fn new() -> Self {
        const ZERO: AtomicUsize = AtomicUsize::new(0);
        Self {
            slots: [const { [ZERO; GLOBAL_TASK_QUEUE_SLOTS] }; axconfig::plat::CPU_NUM],
        }
    }

    #[inline]
    fn try_insert(&self, cpu_id: usize, weak: WeakAxTaskRef) {
        // Allocate once; if we fail to insert, free it immediately.
        let boxed = Box::new(weak);
        let ptr = Box::into_raw(boxed) as usize;

        for slot in &self.slots[cpu_id] {
            if slot
                .compare_exchange(0, ptr, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
        }

        warn!("global task queue on cpu {} is full!", cpu_id);

        // registry full, drop record
        unsafe { drop(Box::from_raw(ptr as *mut WeakAxTaskRef)) };
    }

    #[inline]
    fn sweep_invalid(&self, cpu_id: usize) {
        for slot in &self.slots[cpu_id] {
            let ptr = slot.load(Ordering::Acquire);
            if ptr == 0 {
                continue;
            }
            // Safety: ptr is either 0 or a valid Box<WeakAxTaskRef> installed by try_insert.
            let weak = unsafe { &*(ptr as *const WeakAxTaskRef) };
            if weak.upgrade().is_none() {
                // Try to claim the slot and free.
                if slot
                    .compare_exchange(ptr, 0, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    unsafe { drop(Box::from_raw(ptr as *mut WeakAxTaskRef)) };
                }
            }
        }
    }

    #[inline]
    fn for_each(&self, cpu_id: usize, mut f: impl FnMut(&WeakAxTaskRef)) {
        for slot in &self.slots[cpu_id] {
            let ptr = slot.load(Ordering::Acquire);
            if ptr == 0 {
                continue;
            }
            // Safety: ptr is either 0 or a valid Box<WeakAxTaskRef>.
            let weak = unsafe { &*(ptr as *const WeakAxTaskRef) };
            f(weak);
        }
    }
}

static GLOBAL_TASK_REGISTRY: GlobalTaskRegistry = GlobalTaskRegistry::new();

/// Record a task into the current CPU's watchdog registry.
#[inline]
pub(crate) fn record_task_for_watchdog(task: &crate::AxTaskRef) {
    GLOBAL_TASK_REGISTRY.try_insert(this_cpu_id(), Arc::downgrade(task));
}

/// Sweep invalid weak refs from the current CPU's watchdog registry.
#[inline]
pub(crate) fn sweep_watchdog_tasks(cpu_id: usize) {
    GLOBAL_TASK_REGISTRY.sweep_invalid(cpu_id);
}

/// Iterate the given CPU's watchdog registry.
#[inline]
pub(crate) fn for_each_watchdog_task(cpu_id: usize, f: impl FnMut(&WeakAxTaskRef)) {
    GLOBAL_TASK_REGISTRY.for_each(cpu_id, f);
}
