use axhal::{context::TrapFrame, percpu::this_cpu_id};
use axtask::{AxCpuMask, TaskInner};
use log::debug;

use crate::rendezvous as rv;

/// Stores the active trap frame for each CPU when a watchdog failure is detected.
static mut TRAP_FRAMES: [Option<&TrapFrame>; axconfig::plat::CPU_NUM] =
    [None; axconfig::plat::CPU_NUM];

/// Common watchdog initialization for both primary and secondary CPUs.
///
/// It sets up:
/// - soft lockup detection (timer + watchdog task)
/// - hard lockup detection (PMU/NMI based)
fn init_common() {
    init_softlockup_detection();

    // Register hard lockup detection task.
    crate::register_hardlockup_detection_task();

    // Register mutex deadlock check
    crate::register_watchdog_task(&crate::watchdog_task::MUTEX_DEADLOCK_CHECK);

    // Initialize and enable NMI source for hard lockup detection.
    axhal::nmi::init(axhal::time::timer_frequency() * 10 * 16);
    axhal::nmi::enable();

    // Register NMI handler
    axhal::nmi::register_nmi_handler(|| {
        // Every NMI checks whether watchdog tasks on THIS CPU are healthy.
        // If a failure is detected, THIS CPU becomes the cause CPU and
        // triggers a global rendezvous.
        let fail_name = crate::watchdog_task::check_watchdog_tasks();
        if fail_name.is_some() {
            rv::try_trigger();
        }

        // Once any CPU triggered, ALL CPUs must rendezvous here.
        if rv::is_triggered() {
            rv::mark_arrived();
            unsafe {
                TRAP_FRAMES[this_cpu_id()] = axhal::context::active_trap_frame();
            }
            let this_cpu = this_cpu_id();
            let is_cause = rv::cause_cpu() == Some(this_cpu);
            if is_cause {
                // Strong rendezvous: MUST wait until all CPUs are in NMI.
                rv::wait_all_arrived_strong();

                axplat::console_force_println!(
                    "[watchdog] failure detected on cpu {}, failed_task={:?}, arrived_mask={:#x}",
                    this_cpu,
                    fail_name,
                    rv::arrived_bitmap()
                );

                // Cause CPU dumps all tasks for all CPUs.
                for cpu in 0..axconfig::plat::CPU_NUM {
                    if let Some(tf) = unsafe { TRAP_FRAMES[cpu] } {
                        axtask::dump_cur_task_backtrace(cpu, tf, true);
                    }
                    axtask::dump_cpu_task_backtrace(cpu, true);
                }

                // Notify others that dump is done.
                rv::mark_dump_done();

                // Hard stop on the cause CPU.
                panic!("Watchdog task check failed (global dump)");
            } else {
                // Non-cause CPUs: spin until dump is done.
                while !rv::is_dump_done() {
                    core::hint::spin_loop();
                }
            }
        }
    });

    debug!("watchdog init success on cpu {}", this_cpu_id());
}

/// Initialize soft lockup detection.
///
/// A per-CPU watchdog task periodically updates a timestamp,
/// and timer callbacks check whether the timestamp is stale.
pub fn init_softlockup_detection() {
    // Timer callback used to detect soft lockup conditions.
    axtask::register_timer_callback(|_| {
        let now_ns = axhal::time::monotonic_time_nanos();
        crate::timer_tick();

        if crate::check_softlockup(now_ns) {
            if let Some(tf) = axhal::context::active_trap_frame() {
                axtask::dump_cur_task_backtrace(this_cpu_id(), tf, false);
            }
            axtask::dump_cpu_task_backtrace(this_cpu_id(), false);
        }
    });

    // Watchdog task that periodically "touches" the soft lockup timestamp.
    let watchdog_task = TaskInner::new(
        move || loop {
            crate::touch_softlockup(axhal::time::monotonic_time_nanos());
            axtask::yield_now();
        },
        "watchdog".into(),
        axconfig::TASK_STACK_SIZE,
    );

    // Bind watchdog task to the local CPU.
    watchdog_task.set_cpumask(AxCpuMask::one_shot(this_cpu_id()));
    axtask::spawn_task(watchdog_task);
}

pub fn init_primary() {
    init_common();
}

pub fn init_secondary() {
    init_common();
}
