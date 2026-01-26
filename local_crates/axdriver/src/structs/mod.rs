// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// Copyright (C) 2025 Yuekai Jia <equation618@gmail.com>
// Copyright (C) 2025 ChengXiang Qi <kuangjux@outlook.com>
// See LICENSE for license details.
//
// This file has been modified by KylinSoft on 2025.

#![allow(unused_imports)]

use core::ops::{Deref, DerefMut};

use axdriver_base::{BaseDriverOps, DeviceType};
use smallvec::SmallVec;

#[path = "static.rs"]
mod imp;

pub use imp::*;

/// A unified enum that represents different categories of devices.
#[allow(clippy::large_enum_variant)]
pub enum AxDeviceEnum {
    /// Network card device.
    #[cfg(feature = "net")]
    Net(AxNetDevice),
    /// Block storage device.
    #[cfg(feature = "block")]
    Block(AxBlockDevice),
    /// Graphic display device.
    #[cfg(feature = "display")]
    Display(AxDisplayDevice),
    /// Graphic input device.
    #[cfg(feature = "input")]
    Input(AxInputDevice),
    /// Vsock device.
    #[cfg(feature = "vsock")]
    Vsock(AxVsockDevice),
}

impl BaseDriverOps for AxDeviceEnum {
    #[inline]
    #[allow(unreachable_patterns)]
    fn device_type(&self) -> DeviceType {
        match self {
            #[cfg(feature = "net")]
            Self::Net(_) => DeviceType::Net,
            #[cfg(feature = "block")]
            Self::Block(_) => DeviceType::Block,
            #[cfg(feature = "display")]
            Self::Display(_) => DeviceType::Display,
            #[cfg(feature = "input")]
            Self::Input(_) => DeviceType::Input,
            #[cfg(feature = "vsock")]
            Self::Vsock(_) => DeviceType::Vsock,
            _ => unreachable!(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    fn device_name(&self) -> &str {
        match self {
            #[cfg(feature = "net")]
            Self::Net(dev) => dev.device_name(),
            #[cfg(feature = "block")]
            Self::Block(dev) => dev.device_name(),
            #[cfg(feature = "display")]
            Self::Display(dev) => dev.device_name(),
            #[cfg(feature = "input")]
            Self::Input(dev) => dev.device_name(),
            #[cfg(feature = "vsock")]
            Self::Vsock(dev) => dev.device_name(),
            _ => unreachable!(),
        }
    }
}

/// A structure that contains all device drivers of a certain category.
pub struct AxDeviceContainer<D>(SmallVec<[D; 1]>);

impl<D> AxDeviceContainer<D> {
    /// Constructs the container from one device.
    pub fn from_one(dev: D) -> Self {
        Self(SmallVec::from_buf([dev]))
    }

    /// Takes one device out of the container (will remove it from the
    /// container).
    pub fn take_one(&mut self) -> Option<D> {
        self.0.pop()
    }

    /// Takes `nth` devices out of the container (will remove them from the
    /// container). Returns `None` if there are not enough devices.
    #[allow(dead_code)]
    pub fn take_nth(&mut self, n: usize) -> Option<D> {
        if self.len() >= n {
            Some(self.0.remove(n))
        } else {
            None
        }
    }

    /// Adds one device into the container.
    #[allow(dead_code)]
    pub(crate) fn push(&mut self, dev: D) {
        self.0.push(dev);
    }
}

impl<D> Deref for AxDeviceContainer<D> {
    type Target = SmallVec<[D; 1]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D> DerefMut for AxDeviceContainer<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<D> Default for AxDeviceContainer<D> {
    fn default() -> Self {
        Self(Default::default())
    }
}
