//! Wait status encoding according to POSIX/Linux format
//!
//! This module implements the status value encoding used by waitpid() and
//! related system calls. The status is a 32-bit integer with different
//! encodings depending on how the process terminated or stopped.
//!
//! Reference: https://man7.org/linux/man-pages/man2/waitpid.2.html

// Status encoding constants based on POSIX/Linux format

/// Mask for the low 7 bits (signal number)
const WSIGMASK: i32 = 0x7F;

/// Status code indicating process was stopped
const WSTOPPED: i32 = 0x7F;

/// Bit indicating core dump was produced
const WCOREFLAG: i32 = 0x80;

/// Status value for continued process
const WCONTINUED_STATUS: i32 = 0xFFFF;

/// Shift for exit code or stop signal
const WEXITSHIFT: i32 = 8;

/// Mask for extracting exit code or stop signal
const WEXITMASK: i32 = 0xFF;

/// Wait status value with proper encoding
///
/// This type ensures that status values are correctly encoded according to
/// POSIX standards before being returned to userspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitStatus(i32);

impl WaitStatus {
    /// Create status for process that exited normally
    ///
    /// Encoding: `(exit_code & 0xFF) << 8`
    ///
    /// # Example
    /// ```
    /// use wait_status::WaitStatus;
    /// let status = WaitStatus::exited(42);
    /// assert_eq!(status.as_raw(), 42 << 8);
    /// ```
    pub fn exited(code: i32) -> Self {
        Self((code & WEXITMASK) << WEXITSHIFT)
    }

    /// Create status for process terminated by signal
    ///
    /// Encoding: `(signal & 0x7F) | (core_dump ? 0x80 : 0)`
    ///
    /// # Arguments
    /// * `sig` - Signal number that terminated the process
    /// * `core_dumped` - Whether a core dump was produced
    pub fn signaled(sig: i32, core_dumped: bool) -> Self {
        let mut status = sig & WSIGMASK;
        if core_dumped {
            status |= WCOREFLAG;
        }
        Self(status)
    }

    /// Create status for process stopped by signal
    ///
    /// Encoding: `((signal & 0xFF) << 8) | 0x7F`
    ///
    /// # Arguments
    /// * `sig` - Signal number that stopped the process
    pub fn stopped(sig: i32) -> Self {
        Self(((sig & WEXITMASK) << WEXITSHIFT) | WSTOPPED)
    }

    /// Create status for process continued from stopped state
    ///
    /// Encoding: `0xFFFF`
    pub fn continued() -> Self {
        Self(WCONTINUED_STATUS)
    }

    /// Get the raw status value to return to userspace
    pub fn as_raw(&self) -> i32 {
        self.0
    }
}

// Status inspection helper functions (matching POSIX macros)
// These mirror the C macros: WIFEXITED, WEXITSTATUS, etc.

/// Returns true if the process terminated normally
///
/// Equivalent to POSIX `WIFEXITED(status)` macro.
#[inline]
#[allow(dead_code)]
pub fn wifexited(status: i32) -> bool {
    (status & WSIGMASK) == 0
}

/// Extract the exit status from a normal termination
///
/// Equivalent to POSIX `WEXITSTATUS(status)` macro.
/// Only valid if `wifexited(status)` returns true.
#[inline]
#[allow(dead_code)]
pub fn wexitstatus(status: i32) -> i32 {
    (status >> WEXITSHIFT) & WEXITMASK
}

/// Returns true if the process was terminated by a signal
///
/// Equivalent to POSIX `WIFSIGNALED(status)` macro.
#[inline]
#[allow(dead_code)]
pub fn wifsignaled(status: i32) -> bool {
    ((status & WSIGMASK) + 1) as i8 >= 2
}

/// Extract the signal number that terminated the process
///
/// Equivalent to POSIX `WTERMSIG(status)` macro.
/// Only valid if `wifsignaled(status)` returns true.
#[inline]
#[allow(dead_code)]
pub fn wtermsig(status: i32) -> i32 {
    status & WSIGMASK
}

/// Returns true if a core dump was produced
///
/// Equivalent to POSIX `WCOREDUMP(status)` macro (non-standard extension).
/// Only valid if `wifsignaled(status)` returns true.
#[inline]
#[allow(dead_code)]
pub fn wcoredump(status: i32) -> bool {
    (status & WCOREFLAG) != 0
}

/// Returns true if the process is currently stopped
///
/// Equivalent to POSIX `WIFSTOPPED(status)` macro.
#[inline]
#[allow(dead_code)]
pub fn wifstopped(status: i32) -> bool {
    (status & WEXITMASK) == WSTOPPED
}

/// Extract the signal number that stopped the process
///
/// Equivalent to POSIX `WSTOPSIG(status)` macro.
/// Only valid if `wifstopped(status)` returns true.
#[inline]
#[allow(dead_code)]
pub fn wstopsig(status: i32) -> i32 {
    (status >> WEXITSHIFT) & WEXITMASK
}

/// Returns true if the process was continued
///
/// Equivalent to POSIX `WIFCONTINUED(status)` macro.
#[inline]
#[allow(dead_code)]
pub fn wifcontinued(status: i32) -> bool {
    status == WCONTINUED_STATUS
}
