#![no_std]
pub mod init;
pub mod lockup_detection;
pub mod rendezvous;
pub mod watchdog_task;
pub use crate::{
    init::{init_primary, init_secondary},
    lockup_detection::{
        check_softlockup, register_hardlockup_detection_task, timer_tick, touch_softlockup,
    },
    watchdog_task::register_watchdog_task,
};
