//! Task APIs for multi-task configuration.

use alloc::{
    string::String,
    sync::{Arc, Weak},
};
use core::sync::atomic::AtomicUsize;

#[cfg(feature = "watchdog")]
use axhal::context::TrapFrame;
use kernel_guard::NoPreemptIrqSave;

pub(crate) use crate::run_queue::{current_run_queue, select_run_queue};
#[doc(cfg(all(feature = "multitask", feature = "task-ext")))]
#[cfg(feature = "task-ext")]
pub use crate::task::{AxTaskExt, TaskExt};
#[doc(cfg(all(feature = "multitask", feature = "irq")))]
#[cfg(feature = "irq")]
pub use crate::timers::register_timer_callback;
#[doc(cfg(feature = "multitask"))]
pub use crate::{
    task::{CurrentTask, TaskId, TaskInner, TaskState},
    wait_queue::WaitQueue,
};

/// The reference type of a task.
pub type AxTaskRef = Arc<AxTask>;

/// The weak reference type of a task.
pub type WeakAxTaskRef = Weak<AxTask>;

/// The wrapper type for [`cpumask::CpuMask`] with SMP configuration.
pub type AxCpuMask = cpumask::CpuMask<{ axconfig::plat::CPU_NUM }>;

static CPU_NUM: AtomicUsize = AtomicUsize::new(1);

cfg_if::cfg_if! {
    if #[cfg(feature = "sched-rr")] {
        const MAX_TIME_SLICE: usize = 5;
        pub(crate) type AxTask = axsched::RRTask<TaskInner, MAX_TIME_SLICE>;
        pub(crate) type Scheduler = axsched::RRScheduler<TaskInner, MAX_TIME_SLICE>;
    } else if #[cfg(feature = "sched-cfs")] {
        pub(crate) type AxTask = axsched::CFSTask<TaskInner>;
        pub(crate) type Scheduler = axsched::CFScheduler<TaskInner>;
    } else {
        // If no scheduler features are set, use FIFO as the default.
        pub(crate) type AxTask = axsched::FifoTask<TaskInner>;
        pub(crate) type Scheduler = axsched::FifoScheduler<TaskInner>;
    }
}

#[cfg(feature = "preempt")]
struct KernelGuardIfImpl;

#[cfg(feature = "preempt")]
#[crate_interface::impl_interface]
impl kernel_guard::KernelGuardIf for KernelGuardIfImpl {
    fn disable_preempt() {
        if let Some(curr) = current_may_uninit() {
            curr.disable_preempt();
        }
    }

    fn enable_preempt() {
        if let Some(curr) = current_may_uninit() {
            curr.enable_preempt(true);
        }
    }

    fn local_irq_save_and_disable() -> usize {
        axhal::irq::local_irq_save_and_disable()
    }

    fn local_irq_restore(flags: usize) {
        axhal::irq::local_irq_restore(flags);
    }
}

/// Gets the current task, or returns [`None`] if the current task is not
/// initialized.
pub fn current_may_uninit() -> Option<CurrentTask> {
    CurrentTask::try_get()
}

/// Gets the current task.
///
/// # Panics
///
/// Panics if the current task is not initialized.
pub fn current() -> CurrentTask {
    CurrentTask::get()
}

/// Initializes the task scheduler (for the primary CPU).
pub fn init_scheduler() {
    init_scheduler_with_cpu_num(axconfig::plat::CPU_NUM);
}

/// Initializes the task scheduler with cpu_num (for the primary CPU).
pub fn init_scheduler_with_cpu_num(cpu_num: usize) {
    info!("Initialize scheduling...");
    CPU_NUM.store(cpu_num, core::sync::atomic::Ordering::Relaxed);

    crate::run_queue::init();

    info!("  use {} scheduler.", Scheduler::scheduler_name());
}

pub(crate) fn active_cpu_num() -> usize {
    CPU_NUM.load(core::sync::atomic::Ordering::Relaxed)
}

/// Initializes the task scheduler for secondary CPUs.
pub fn init_scheduler_secondary() {
    crate::run_queue::init_secondary();
}

/// Handles periodic timer ticks for the task manager.
///
/// For example, advance scheduler states, checks timed events, etc.
#[cfg(feature = "irq")]
#[doc(cfg(feature = "irq"))]
pub fn on_timer_tick() {
    use kernel_guard::NoOp;
    crate::timers::check_events();
    // Since irq and preemption are both disabled here,
    // we can get current run queue with the default `kernel_guard::NoOp`.
    current_run_queue::<NoOp>().scheduler_timer_tick();
}

/// Adds the given task to the run queue, returns the task reference.
pub fn spawn_task(task: TaskInner) -> AxTaskRef {
    let task_ref = task.into_arc();
    select_run_queue::<NoPreemptIrqSave>(&task_ref).add_task(task_ref.clone());
    task_ref
}

/// Spawns a new task with the given parameters.
///
/// Returns the task reference.
pub fn spawn_raw<F>(f: F, name: String, stack_size: usize) -> AxTaskRef
where
    F: FnOnce() + Send + 'static,
{
    spawn_task(TaskInner::new(f, name, stack_size))
}

/// Spawns a new task with the given name and the default stack size ([`axconfig::TASK_STACK_SIZE`]).
///
/// Returns the task reference.
pub fn spawn_with_name<F>(f: F, name: String) -> AxTaskRef
where
    F: FnOnce() + Send + 'static,
{
    spawn_raw(f, name, axconfig::TASK_STACK_SIZE)
}

/// Spawns a new task with the default parameters.
///
/// The default task name is an empty string. The default task stack size is
/// [`axconfig::TASK_STACK_SIZE`].
///
/// Returns the task reference.
pub fn spawn<F>(f: F) -> AxTaskRef
where
    F: FnOnce() + Send + 'static,
{
    spawn_with_name(f, String::new())
}

/// Set the priority for current task.
///
/// The range of the priority is dependent on the underlying scheduler. For
/// example, in the [CFS] scheduler, the priority is the nice value, ranging from
/// -20 to 19.
///
/// Returns `true` if the priority is set successfully.
///
/// [CFS]: https://en.wikipedia.org/wiki/Completely_Fair_Scheduler
pub fn set_priority(prio: isize) -> bool {
    current_run_queue::<NoPreemptIrqSave>().set_current_priority(prio)
}

/// Set the affinity for the current task.
/// [`AxCpuMask`] is used to specify the CPU affinity.
/// Returns `true` if the affinity is set successfully.
///
/// TODO: support set the affinity for other tasks.
pub fn set_current_affinity(cpumask: AxCpuMask) -> bool {
    if cpumask.is_empty() {
        false
    } else {
        let curr = current().clone();

        curr.set_cpumask(cpumask);
        // After setting the affinity, we need to check if current cpu matches
        // the affinity. If not, we need to migrate the task to the correct CPU.
        #[cfg(feature = "smp")]
        if !cpumask.get(axhal::percpu::this_cpu_id()) {
            const MIGRATION_TASK_STACK_SIZE: usize = 4096;
            // Spawn a new migration task for migrating.
            let migration_task = TaskInner::new(
                move || crate::run_queue::migrate_entry(curr),
                "migration-task".into(),
                MIGRATION_TASK_STACK_SIZE,
            )
            .into_arc();

            // Migrate the current task to the correct CPU using the migration task.
            current_run_queue::<NoPreemptIrqSave>().migrate_current(migration_task);

            assert!(
                cpumask.get(axhal::percpu::this_cpu_id()),
                "Migration failed"
            );
        }
        true
    }
}

/// Current task gives up the CPU time voluntarily, and switches to another
/// ready task.
pub fn yield_now() {
    current_run_queue::<NoPreemptIrqSave>().yield_current()
}

/// Current task is going to sleep for the given duration.
///
/// If the feature `irq` is not enabled, it uses busy-wait instead.
pub fn sleep(dur: core::time::Duration) {
    sleep_until(axhal::time::wall_time() + dur);
}

/// Current task is going to sleep, it will be woken up at the given deadline.
///
/// If the feature `irq` is not enabled, it uses busy-wait instead.
pub fn sleep_until(deadline: axhal::time::TimeValue) {
    #[cfg(feature = "irq")]
    crate::future::block_on(crate::future::sleep_until(deadline));
    #[cfg(not(feature = "irq"))]
    axhal::time::busy_wait_until(deadline);
}

/// Exits the current task.
pub fn exit(exit_code: i32) -> ! {
    current_run_queue::<NoPreemptIrqSave>().exit_current(exit_code)
}

/// The idle task routine.
///
/// It runs an infinite loop that keeps calling [`yield_now()`].
pub fn run_idle() -> ! {
    loop {
        yield_now();
        trace!("idle task: waiting for IRQs...");
        #[cfg(feature = "irq")]
        axhal::asm::wait_for_irqs();
    }
}

#[cfg(feature = "watchdog")]
#[inline(always)]
fn dump_println(force: bool, args: core::fmt::Arguments<'_>) {
    if force {
        axplat::console_force_println!("{}", args);
    } else {
        // Use log output in normal (non-NMI) contexts.
        error!("{}", args);
    }
}

#[cfg(feature = "watchdog")]
pub fn dump_cpu_task_backtrace(cpu_id: usize, force: bool) {
    crate::global_task_queue::for_each_watchdog_task(cpu_id, |weaktask| {
        if let Some(task) = weaktask.upgrade()
            && !task.inner().is_running()
        {
            let ctx = task.inner().ctx();
            #[cfg(target_arch = "aarch64")]
            let bt = axbacktrace::Backtrace::capture_trap(
                ctx.r29 as usize, // fp
                ctx.lr as usize,  // ip
                ctx.lr as usize,  // ra
            );

            #[cfg(not(target_arch = "aarch64"))]
            let bt = {
                panic!(
                    "dump_cpu_task_backtrace: unimplemented arch {}",
                    core::env!("CARGO_CFG_TARGET_ARCH")
                );
            };
            dump_println(
                force,
                format_args!("cpu_id: {}, {:?}\n{bt}", cpu_id, task.inner()),
            );
        }
    });
}

#[cfg(feature = "watchdog")]
#[inline(always)]
pub fn dump_cur_task_backtrace(cpu_id: usize, tf: &TrapFrame, force: bool) {
    #[cfg(target_arch = "aarch64")]
    let bt = axbacktrace::Backtrace::capture_trap(
        tf.x[29] as usize,
        tf.x[30] as usize,
        tf.x[30] as usize,
    );

    #[cfg(not(target_arch = "aarch64"))]
    let bt = {
        panic!(
            "dump_cur_task_backtrace: unimplemented arch {}",
            core::env!("CARGO_CFG_TARGET_ARCH")
        );
    };
    dump_println(
        force,
        format_args!("cpu_id: {}, {:?}\n{bt}", cpu_id, current().inner()),
    );
}

/// Returns `true` when no suspicious long lock-waits are observed on this CPU.
/// Returns `false` when a task appears to have been waiting on a lock for too long.
///
/// Note: this is a *heuristic* watchdog check, not a full deadlock detector.
#[cfg(feature = "watchdog")]
pub fn check_mutex_deadlock(now: usize) -> bool {
    let mut ok = true;
    crate::global_task_queue::for_each_watchdog_task(axhal::percpu::this_cpu_id(), |weaktask| {
        if !ok {
            return;
        }
        if let Some(task) = weaktask.upgrade() {
            let Some((_lock, since)) = task.inner().waiting_snapshot() else {
                return;
            };

            let blocked = now.saturating_sub(since);
            if axhal::time::ticks_to_nanos(blocked as u64) > 20_000_000_000 {
                // suspect stall (20s)
                ok = false;
                return;
            }
        }
    });
    ok
}
