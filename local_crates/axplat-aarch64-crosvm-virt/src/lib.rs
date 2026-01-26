// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 WeiKang Guo <guoweikang.kernel@gmail.com
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.

#![no_std]

#[macro_use]
extern crate axplat;

mod boot;
mod init;
mod mem;
mod power;
pub mod fdt;
mod serial;
mod gicv3;
pub mod psci;

pub mod config {
    //! Platform configuration module.
    //!
    //! If the `AX_CONFIG_PATH` environment variable is set, it will load the configuration from the specified path.
    //! Otherwise, it will fall back to the `axconfig.toml` file in the current directory and generate the default configuration.
    //!
    //! If the `PACKAGE` field in the configuration does not match the package name, it will panic with an error message.
    axconfig_macros::include_configs!(path_env = "AX_CONFIG_PATH", fallback = "axconfig.toml");
    assert_str_eq!(
        PACKAGE,
        env!("CARGO_PKG_NAME"),
        "`PACKAGE` field in the configuration does not match the Package name. Please check your configuration file."
    );
}

axplat_aarch64_peripherals::ns16550_console_if_impl!(ConsoleIfImpl);
axplat_aarch64_peripherals::time_if_impl!(TimeIfImpl);


#[cfg(feature = "irq")]
irq_if_impl!(IrqIfImpl);
