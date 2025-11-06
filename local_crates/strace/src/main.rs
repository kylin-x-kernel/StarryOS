use std::env;
use std::process::Command;
use std::os::unix::process::CommandExt;
use syscall_numbers::aarch64::sys_call_name;

// Helper function to read a word from tracee memory using PTRACE_PEEKDATA
unsafe fn peek_data(pid: i32, addr: u64) -> Result<u64, ()> {
    let result = libc::ptrace(libc::PTRACE_PEEKDATA, pid, addr as *mut libc::c_void, 0);
    if result == -1 {
        Err(())
    } else {
        Ok(result as u64)
    }
}

// Helper function to read a null-terminated string from tracee memory
unsafe fn read_string(pid: i32, addr: u64, max_len: usize) -> String {
    if addr == 0 {
        return "NULL".to_string();
    }

    let mut result = Vec::new();
    let mut current_addr = addr;

    for _ in 0..(max_len / 8 + 1) {
        match peek_data(pid, current_addr) {
            Ok(word) => {
                // Extract bytes from the word (little-endian)
                let bytes = word.to_le_bytes();
                for &byte in &bytes {
                    if byte == 0 {
                        // Found null terminator
                        if result.is_empty() {
                            return "\"\"".to_string();
                        }
                        return format!("\"{}\"", String::from_utf8_lossy(&result));
                    }
                    result.push(byte);
                    if result.len() >= max_len {
                        return format!("\"{}\"...", String::from_utf8_lossy(&result));
                    }
                }
                current_addr += 8;
            }
            Err(_) => {
                return format!("<invalid-ptr-{:#x}>", addr);
            }
        }
    }

    format!("\"{}\"...", String::from_utf8_lossy(&result))
}

// Helper function to escape special characters for display
fn escape_string(bytes: &[u8]) -> String {
    let mut result = String::new();
    for &byte in bytes {
        match byte {
            b'\n' => result.push_str("\\n"),
            b'\r' => result.push_str("\\r"),
            b'\t' => result.push_str("\\t"),
            b'\\' => result.push_str("\\\\"),
            b'"' => result.push_str("\\\""),
            b'\0' => result.push_str("\\0"),
            0x20..=0x7e => result.push(byte as char), // Printable ASCII
            _ => result.push_str(&format!("\\x{:02x}", byte)), // Non-printable as hex
        }
    }
    result
}

// Decode openat flags
fn decode_open_flags(flags: u64) -> String {
    let mut parts = Vec::new();
    let access_mode = flags & 0x3;

    match access_mode {
        0 => parts.push("O_RDONLY"),
        1 => parts.push("O_WRONLY"),
        2 => parts.push("O_RDWR"),
        _ => {}
    }

    if flags & 0x40 != 0 { parts.push("O_CREAT"); }
    if flags & 0x80 != 0 { parts.push("O_EXCL"); }
    if flags & 0x200 != 0 { parts.push("O_NOCTTY"); }
    if flags & 0x400 != 0 { parts.push("O_TRUNC"); }
    if flags & 0x800 != 0 { parts.push("O_APPEND"); }
    if flags & 0x1000 != 0 { parts.push("O_NONBLOCK"); }
    if flags & 0x8000 != 0 { parts.push("O_LARGEFILE"); }
    if flags & 0x10000 != 0 { parts.push("O_DIRECTORY"); }
    if flags & 0x20000 != 0 { parts.push("O_NOFOLLOW"); }
    if flags & 0x80000 != 0 { parts.push("O_CLOEXEC"); }

    if parts.is_empty() {
        format!("{:#x}", flags)
    } else {
        parts.join("|")
    }
}

// Decode dirfd value
fn decode_dirfd(dfd: u64) -> String {
    if dfd == 0xffffffffffffff9c || dfd as i32 == -100 {
        "AT_FDCWD".to_string()
    } else {
        format!("{}", dfd as i32)
    }
}

// Decode mmap protection flags
fn decode_prot_flags(prot: u64) -> String {
    let mut parts = Vec::new();
    if prot & 0x1 != 0 { parts.push("PROT_READ"); }
    if prot & 0x2 != 0 { parts.push("PROT_WRITE"); }
    if prot & 0x4 != 0 { parts.push("PROT_EXEC"); }
    if prot == 0 { return "PROT_NONE".to_string(); }
    if parts.is_empty() { format!("{:#x}", prot) } else { parts.join("|") }
}

// Decode mmap flags
fn decode_mmap_flags(flags: u64) -> String {
    let mut parts = Vec::new();
    if flags & 0x01 != 0 { parts.push("MAP_SHARED"); }
    if flags & 0x02 != 0 { parts.push("MAP_PRIVATE"); }
    if flags & 0x10 != 0 { parts.push("MAP_FIXED"); }
    if flags & 0x20 != 0 { parts.push("MAP_ANONYMOUS"); }
    if parts.is_empty() { format!("{:#x}", flags) } else { parts.join("|") }
}

// Helper function to read fixed-length buffer from tracee memory
unsafe fn read_buffer(pid: i32, addr: u64, len: usize) -> String {
    if addr == 0 {
        return "NULL".to_string();
    }

    let mut result = Vec::new();
    let mut current_addr = addr;
    let max_display = len.min(64); // Limit display to 64 bytes

    for _ in 0..((max_display + 7) / 8) {
        match peek_data(pid, current_addr) {
            Ok(word) => {
                let bytes = word.to_le_bytes();
                for &byte in &bytes {
                    if result.len() >= max_display {
                        break;
                    }
                    result.push(byte);
                }
                current_addr += 8;
            }
            Err(_) => {
                return format!("<invalid-ptr-{:#x}>", addr);
            }
        }
    }

    let escaped = escape_string(&result);
    if len > max_display {
        format!("\"{}\"... ({} bytes)", escaped, len)
    } else {
        format!("\"{}\"", escaped)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: strace [-v] <command> [args...]");
        return;
    }

    // Parse verbose flag
    let mut verbose = false;
    let mut cmd_start = 1;

    if args[1] == "-v" || args[1] == "--verbose" {
        verbose = true;
        cmd_start = 2;
        if args.len() < 3 {
            eprintln!("Usage: strace [-v] <command> [args...]");
            return;
        }
    }

    let command = &args[cmd_start];
    let command_args = &args[cmd_start + 1..];

    match unsafe { libc::fork() } {
        -1 => panic!("fork failed"),
        0 => {
            // child process
            if verbose {
                eprintln!("strace: Child process starting, enabling ptrace...");
            }
            unsafe {
                let _ = Command::new(command)
                    .args(command_args)
                    .pre_exec(|| {
                        libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0);
                        Ok(())
                    })
                    .exec();
            }
            eprintln!("strace: exec failed!");
        }
        pid => {
            // parent process
            if verbose {
                eprintln!("strace: Starting trace of command: {} {:?}", command, command_args);
                eprintln!("strace: Parent tracing child process with PID: {}", pid);
            }
            let mut status = 0;
            unsafe {
                libc::waitpid(pid, &mut status, 0);
            }
            if verbose {
                eprintln!("strace: Child process stopped, beginning syscall trace...");
            }

            let mut syscall_count = 0;
            let mut is_entry = true; // Track if we're on entry (true) or exit (false)
            let mut pending_output = String::new(); // Store syscall entry output
            let mut current_syscall = String::new(); // Track current syscall name
            let mut current_syscall_nr: i64 = -1; // Track current syscall number
            let mut read_params: (u64, u64, u64) = (0, 0, 0); // Store read(fd, buf, count)

            while libc::WIFSTOPPED(status) {
                let mut regs: libc::user_regs_struct = unsafe { std::mem::zeroed() };
                unsafe {
                    libc::ptrace(libc::PTRACE_GETREGS, pid, 0, &mut regs as *mut _ as *mut libc::c_void);
                }

                let syscall_nr = regs.regs[8] as i64;

                // Detect if we're out of sync (syscall number changed when we expected exit)
                if !is_entry && syscall_nr != current_syscall_nr {
                    // Print the pending syscall without return value (unfinished)
                    if !pending_output.is_empty() {
                        eprintln!("{} = <unfinished ...>", pending_output);
                    }
                    // We're out of sync - treat this as a new entry
                    is_entry = true;
                    pending_output.clear();
                    current_syscall.clear();
                }

                if is_entry {
                    // Syscall entry - format but don't print yet
                    syscall_count += 1;
                    current_syscall_nr = syscall_nr;

                    if let Some(syscall_name) = sys_call_name(syscall_nr) {
                        current_syscall = syscall_name.to_string();
                        // Format syscall with decoded flags (Alpine strace style)
                        pending_output = match syscall_name {
                            "write" => {
                                let fd = regs.regs[0];
                                let buf_addr = regs.regs[1];
                                let count = regs.regs[2] as usize;
                                let buf_content = unsafe { read_buffer(pid, buf_addr, count) };
                                format!("write({}, {}, {})", fd, buf_content, count)
                            },
                            "writev" => {
                                let fd = regs.regs[0];
                                let iovec_addr = regs.regs[1];
                                let iovcnt = regs.regs[2];
                                // Try to read iovec structures
                                let mut iovec_display = Vec::new();
                                for i in 0..iovcnt.min(3) {
                                    let iov_addr = iovec_addr + i * 16; // iovec is 16 bytes on 64-bit
                                    if let Ok(iov_base) = unsafe { peek_data(pid, iov_addr) } {
                                        if let Ok(iov_len) = unsafe { peek_data(pid, iov_addr + 8) } {
                                            let content = unsafe { read_buffer(pid, iov_base, iov_len as usize) };
                                            iovec_display.push(format!("{{iov_base={}, iov_len={}}}", content, iov_len));
                                        }
                                    }
                                }
                                if !iovec_display.is_empty() {
                                    format!("writev({}, [{}], {})", fd, iovec_display.join(", "), iovcnt)
                                } else {
                                    format!("writev({}, {:#x}, {})", fd, iovec_addr, iovcnt)
                                }
                            },
                            "execve" => {
                                let filename = unsafe { read_string(pid, regs.regs[0], 256) };
                                format!("execve({}, {:#x}, {:#x})", filename, regs.regs[1], regs.regs[2])
                            },
                            "openat" => {
                                let dfd = decode_dirfd(regs.regs[0]);
                                let filename = unsafe { read_string(pid, regs.regs[1], 256) };
                                let flags = decode_open_flags(regs.regs[2]);
                                format!("openat({}, {}, {})", dfd, filename, flags)
                            },
                            "newfstatat" => {
                                let dfd = decode_dirfd(regs.regs[0]);
                                let pathname = unsafe { read_string(pid, regs.regs[1], 256) };
                                format!("newfstatat({}, {}, {:#x}, {})", dfd, pathname, regs.regs[2], regs.regs[3])
                            },
                            "read" => {
                                // Store read parameters for exit processing
                                read_params = (regs.regs[0], regs.regs[1], regs.regs[2]);
                                format!("read({}", regs.regs[0])
                            },
                            "getdents64" => {
                                format!("getdents64({}, {:#x}, {})", regs.regs[0], regs.regs[1], regs.regs[2])
                            },
                            "ioctl" => {
                                let fd = regs.regs[0];
                                let request = regs.regs[1];
                                let request_name = match request {
                                    0x5413 => "TIOCGWINSZ",
                                    0x5401 => "TCGETS",
                                    0x5402 => "TCSETS",
                                    _ => "",
                                };
                                if !request_name.is_empty() {
                                    format!("ioctl({}, {}, {:#x})", fd, request_name, regs.regs[2])
                                } else {
                                    format!("ioctl({}, {:#x}, {:#x})", fd, request, regs.regs[2])
                                }
                            },
                            "fcntl" => {
                                let fd = regs.regs[0];
                                let cmd = regs.regs[1];
                                let cmd_name = match cmd {
                                    0 => "F_DUPFD",
                                    1 => "F_GETFD",
                                    2 => "F_SETFD",
                                    3 => "F_GETFL",
                                    4 => "F_SETFL",
                                    _ => "",
                                };
                                let arg_name = if cmd == 2 && regs.regs[2] == 1 {
                                    "FD_CLOEXEC".to_string()
                                } else {
                                    format!("{:#x}", regs.regs[2])
                                };
                                if !cmd_name.is_empty() {
                                    format!("fcntl({}, {}, {})", fd, cmd_name, arg_name)
                                } else {
                                    format!("fcntl({}, {}, {})", fd, cmd, arg_name)
                                }
                            },
                            "close" => format!("close({})", regs.regs[0]),
                            "lseek" => {
                                let fd = regs.regs[0];
                                let offset = regs.regs[1] as i64;
                                let whence = regs.regs[2];
                                let whence_name = match whence {
                                    0 => "SEEK_SET",
                                    1 => "SEEK_CUR",
                                    2 => "SEEK_END",
                                    _ => "SEEK_UNKNOWN",
                                };
                                format!("lseek({}, {}, {})", fd, offset, whence_name)
                            },
                            "mmap" => {
                                let addr = if regs.regs[0] == 0 { "NULL".to_string() } else { format!("{:#x}", regs.regs[0]) };
                                let prot = decode_prot_flags(regs.regs[2]);
                                let flags = decode_mmap_flags(regs.regs[3]);
                                format!("mmap({}, {}, {}, {}, {}, {})",
                                    addr, regs.regs[1], prot, flags, regs.regs[4] as i32, regs.regs[5])
                            },
                            "mprotect" => {
                                let prot = decode_prot_flags(regs.regs[2]);
                                format!("mprotect({:#x}, {}, {})", regs.regs[0], regs.regs[1], prot)
                            },
                            "munmap" => format!("munmap({:#x}, {})", regs.regs[0], regs.regs[1]),
                            "brk" => {
                                let addr = if regs.regs[0] == 0 { "NULL".to_string() } else { format!("{:#x}", regs.regs[0]) };
                                format!("brk({})", addr)
                            },
                            _ => format!("{}({:#x}, {:#x}, {:#x})", syscall_name, regs.regs[0], regs.regs[1], regs.regs[2])
                        };
                    } else {
                        pending_output = format!("syscall_{}({:#x}, {:#x}, {:#x})",
                            syscall_nr, regs.regs[0], regs.regs[1], regs.regs[2]);
                    }
                } else {
                    // Syscall exit - print with return value
                    let retval = regs.regs[0] as i64;

                    // Special handling for execve: it has extra ptrace stops from stop_current_and_wait
                    if current_syscall == "execve" {
                        // Detect exec-stop: if retval looks like a pointer (large positive value), skip it
                        if retval > 0x10000 {
                            // This is the exec-stop, not the real exit. Continue without printing or toggling
                            unsafe {
                                libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, 0);
                                libc::waitpid(pid, &mut status, 0);
                            }
                            continue;
                        }
                        // Print with return value
                        eprintln!("{} = {}", pending_output, retval);
                    } else if current_syscall == "read" {
                        // Special handling for read: show buffer content if successful
                        let (_fd, buf_addr, count) = read_params;
                        if retval > 0 {
                            let bytes_read = retval as usize;
                            let buf_content = unsafe { read_buffer(pid, buf_addr, bytes_read) };
                            eprintln!("{}, {}, {}) = {}", pending_output, buf_content, count, retval);
                        } else {
                            eprintln!("{}, {:#x}, {}) = {}", pending_output, buf_addr, count, retval);
                        }
                    } else {
                        eprintln!("{} = {}", pending_output, retval);
                    }

                    pending_output.clear();
                    current_syscall.clear();
                }

                // Toggle between entry and exit
                is_entry = !is_entry;

                unsafe {
                    libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, 0);
                    libc::waitpid(pid, &mut status, 0);
                }
            }

            if libc::WIFEXITED(status) {
                let exit_code = libc::WEXITSTATUS(status);
                eprintln!("+++ exited with {} +++", exit_code);
                if verbose {
                    eprintln!("strace: Total syscalls traced: {}", syscall_count);
                }
            } else if libc::WIFSIGNALED(status) {
                let signal = libc::WTERMSIG(status);
                eprintln!("+++ killed by signal {} +++", signal);
                if verbose {
                    eprintln!("strace: Total syscalls traced: {}", syscall_count);
                }
            }
        }
    }
}
