//! The core functionality of a monolithic kernel, including loading user
//! programs and managing processes.

#![no_std]
#![feature(likely_unlikely)]
#![warn(missing_docs)]

extern crate alloc;

#[macro_use]
extern crate axlog;

pub mod config;
pub mod futex;
pub mod mm;
pub mod resources;
pub mod shm;
pub mod task;
pub mod time;
pub mod vfs;

// Optional extension hooks (feature-only logic should live outside core)
pub mod hooks;