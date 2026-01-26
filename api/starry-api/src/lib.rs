#![no_std]
#![feature(likely_unlikely)]
#![feature(bstr)]
#![allow(missing_docs)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

#[macro_use]
extern crate axlog;

extern crate alloc;

pub mod file;
pub mod io;
pub mod mm;
pub mod signal;
pub mod socket;
pub mod syscall;
pub mod task;
#[cfg(feature = "tee")]
pub mod tee;
pub mod terminal;
pub mod time;
pub mod vfs;

/// Initialize.
pub fn init() {
    info!("Initialize VFS...");
    vfs::mount_all().expect("Failed to mount vfs");

    info!("Initialize /proc/interrupts...");
    axtask::register_timer_callback(|_| {
        time::inc_irq_cnt();
    });

    info!("Initialize alarm...");
    starry_core::time::spawn_alarm_task();

    #[cfg(feature = "tee_test")]
    {
        use crate::tee::test::{test_examples::tee_test_example, test_unit_test::tee_test_unit};

        info!("Running TEE tests...");
        tee_test_example();
        tee_test_unit();
    }
}
