#![no_std]
#![doc = include_str!("../README.md")]

#[macro_use]
extern crate log;

pub mod generic_timer;
#[cfg(feature = "irq")]
pub mod gic;
pub mod pl011;
pub mod ns16550a;
pub mod pl031;
pub mod psci;
#[cfg(any(feature = "nmi-pmu", feature = "nmi-sdei"))]
pub mod nmi;
#[cfg(feature = "pmu")]
pub mod pmu;
