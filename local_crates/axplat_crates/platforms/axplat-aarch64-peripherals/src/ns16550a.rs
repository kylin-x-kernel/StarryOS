//! NS16550A UART driver.
use uart_16550::{MmioSerialPort,WouldBlockError};
use axplat::mem::VirtAddr;
use kspin::SpinNoIrq;
use lazyinit::LazyInit;

static UART: LazyInit<SpinNoIrq<MmioSerialPort>> = LazyInit::new();

fn do_putchar(uart: &mut MmioSerialPort, c: u8) {
    match c {
        b'\n' => {
            uart.send(b'\r');
            uart.send(b'\n');
        }
        c => uart.send(c),
    }
}

pub fn write_bytes_force(uart_base: VirtAddr, bytes: &[u8]){
    let base_addr = uart_base.as_usize();
    let mut uart = unsafe { MmioSerialPort::new(base_addr) };
    uart.init();
    for c in bytes {
        do_putchar(&mut uart, *c);
    }
}

/// Writes a byte to the console.
pub fn putchar(c: u8) {
    do_putchar(&mut UART.lock(), c);
}

/// Reads a byte from the console, or returns [`None`] if no input is available.
pub fn getchar<E>() -> Result<u8,WouldBlockError> {
    UART.lock().try_receive()
}

/// Write a slice of bytes to the console.
pub fn write_bytes(bytes: &[u8]) {
    let mut uart = UART.lock();
    for c in bytes {
        do_putchar(&mut uart, *c);
    }
}

/// Reads bytes from the console into the given mutable slice.
/// Returns the number of bytes read.
pub fn read_bytes(bytes: &mut [u8]) -> usize {
    let mut read_len = 0;
    while read_len < bytes.len() {
        if let Ok(c) = getchar::<WouldBlockError>() {
            bytes[read_len] = c;
            read_len += 1;
        } else {
            break;
        }
    }
    read_len
}

/// Early stage initialization of the NS16550A UART driver.
pub fn init_early(uart_base: VirtAddr) {
    UART.init_once(SpinNoIrq::new({
        let base_addr = uart_base.as_usize();
        let mut uart = unsafe { MmioSerialPort::new(base_addr) };
        uart.init();
        uart
    }));
}

/// Default implementation of [`axplat::console::ConsoleIf`] using the
/// 16550a UART.
#[macro_export]
macro_rules! ns16550_console_if_impl {
    ($name:ident) => {
        struct $name;

        #[axplat::impl_plat_interface]
        impl axplat::console::ConsoleIf for $name {
            /// Writes given bytes to the console.
            fn write_bytes(bytes: &[u8]) {
                $crate::ns16550a::write_bytes(bytes);
            }

            fn write_bytes_force(bytes: &[u8]) {
                let mut uart_base = axplat::mem::phys_to_virt(axplat::mem::pa!(crate::config::devices::UART_PADDR));
                $crate::ns16550a::write_bytes_force(uart_16550, bytes);
            }

            /// Reads bytes from the console into the given mutable slice.
            ///
            /// Returns the number of bytes read.
            fn read_bytes(bytes: &mut [u8]) -> usize {
                $crate::ns16550a::read_bytes(bytes)
            }

            /// Returns the IRQ number for the console, if applicable.
            #[cfg(feature = "irq")]
            fn irq_num() -> Option<usize> {
                // Note that `crate` is not `$crate`!
                Some(crate::config::devices::UART_IRQ as _)
            }
        }
    };
}
