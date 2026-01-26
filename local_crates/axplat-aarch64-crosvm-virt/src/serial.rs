// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 WeiKang Guo <guoweikang.kernel@gmail.com
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSE for license details.

#[unsafe(no_mangle)]
pub extern "C" fn _boot_print_usize(num: usize) {
    let mut msg: [u8; 16] = [0; 16];
    let mut num = num;
    let mut cnt = 0;

    boot_print_str("0x");
    if num == 0 {
        boot_serial_send('0' as u8);
    } else {
        loop {
            if num == 0 {
                break;
            }
            msg[cnt] = match (num & 0xf) as u8 {
                n if n < 10 => n + '0' as u8,
                n => n - 10 + 'a' as u8,
            };
            cnt += 1;
            num >>= 4;
        }
        for i in 0..cnt {
            boot_serial_send(msg[cnt - i - 1]);
        }
    }
    boot_print_str("\r\n");
}

/// 打印字符串
#[unsafe(no_mangle)]
pub fn boot_print_str(data: &str) {
    for byte in data.bytes() {
        boot_serial_send(byte);
    }
}

/// 打印整形
#[allow(dead_code)]
pub fn boot_print_usize(num: usize) {
    _boot_print_usize(num);
}

#[derive(Copy, Clone, Debug)]
/// Struct representing a NS16550A UART peripheral
pub struct Uart {
	/// Base address of the peripheral
	base_address: usize,
}

impl Uart {
	/// Creates a new instance of `Uart` with the given base address.
	pub const fn new(base_address: usize) -> Self {
		Self { base_address }
	}

	/// If the transmitter holding register is empty, writes `c` in the transmitter holding register, and returns `c`. Otherwise returns `None`.
	pub fn put(&self, c: u8) -> Option<u8> {
		let ptr = self.base_address as *mut u8;
		unsafe {
			ptr.write_volatile(c);
		}
		Some(c)
	}
}

static BOOT_SERIAL: Uart = Uart::new(0x3f8);

#[allow(dead_code)]
/// 打印EL1寄存器
pub fn print_el1_reg(switch: bool) {
    if !switch {
        return;
    }
    crate::boot_print_reg!("SCTLR_EL1");
    crate::boot_print_reg!("SPSR_EL1");
    crate::boot_print_reg!("TCR_EL1");
    crate::boot_print_reg!("VBAR_EL1");
    crate::boot_print_reg!("MAIR_EL1");
    crate::boot_print_reg!("MPIDR_EL1");
    crate::boot_print_reg!("TTBR0_EL1");
    crate::boot_print_reg!("TTBR1_EL1");
    crate::boot_print_reg!("ID_AA64AFR0_EL1");
    crate::boot_print_reg!("ID_AA64AFR1_EL1");
    crate::boot_print_reg!("ID_AA64DFR0_EL1");
    crate::boot_print_reg!("ID_AA64DFR1_EL1");
    crate::boot_print_reg!("ID_AA64ISAR0_EL1");
    crate::boot_print_reg!("ID_AA64ISAR1_EL1");
    crate::boot_print_reg!("ID_AA64ISAR2_EL1");
    crate::boot_print_reg!("ID_AA64MMFR0_EL1");
    crate::boot_print_reg!("ID_AA64MMFR1_EL1");
    crate::boot_print_reg!("ID_AA64MMFR2_EL1");
    crate::boot_print_reg!("ID_AA64PFR0_EL1");
    crate::boot_print_reg!("ID_AA64PFR1_EL1");

    crate::boot_print_reg!("ICC_AP0R0_EL1");
    crate::boot_print_reg!("ICC_AP1R0_EL1");
    crate::boot_print_reg!("ICC_BPR0_EL1");
    crate::boot_print_reg!("ICC_BPR1_EL1");
    crate::boot_print_reg!("ICC_CTLR_EL1");
    crate::boot_print_reg!("ICC_HPPIR0_EL1");
    crate::boot_print_reg!("ICC_HPPIR1_EL1");
    crate::boot_print_reg!("ICC_IAR0_EL1");
    crate::boot_print_reg!("ICC_IAR1_EL1");
    crate::boot_print_reg!("ICC_IGRPEN0_EL1");
    crate::boot_print_reg!("ICC_IGRPEN1_EL1");
    crate::boot_print_reg!("ICC_PMR_EL1");
    crate::boot_print_reg!("ICC_RPR_EL1");
    crate::boot_print_reg!("ICC_SRE_EL1");
} 

/// BOOT阶段打印寄存器，用于调试
#[macro_export]
macro_rules! boot_print_reg {
    ($reg_name:tt) => {
        boot_print_str($reg_name);
        boot_print_str(": ");
        let reg;
        unsafe { core::arch::asm!(concat!("mrs {}, ", $reg_name), out(reg) reg) };
        boot_print_usize(reg);
    };
}

/// 打印字节
#[allow(unused)]
pub fn boot_serial_send(data: u8) {
    unsafe { BOOT_SERIAL.put(data) };
}
