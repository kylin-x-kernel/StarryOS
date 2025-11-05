use std::env;
use std::process::Command;
use std::os::unix::process::CommandExt;
use syscall_numbers::aarch64::sys_call_name;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: strace <command>");
        return;
    }

    let command = &args[1];
    let command_args = &args[2..];

    eprintln!("strace: Starting trace of command: {} {:?}", command, command_args);

    match unsafe { libc::fork() } {
        -1 => panic!("fork failed"),
        0 => {
            // child process
            eprintln!("strace: Child process starting, enabling ptrace...");
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
            eprintln!("strace: Parent tracing child process with PID: {}", pid);
            let mut status = 0;
            unsafe {
                libc::waitpid(pid, &mut status, 0);
            }
            eprintln!("strace: Child process stopped, beginning syscall trace...");

            let mut syscall_count = 0;
            let mut is_entry = true; // Track if we're on entry (true) or exit (false)

            while libc::WIFSTOPPED(status) {
                let mut regs: libc::user_regs_struct = unsafe { std::mem::zeroed() };
                unsafe {
                    libc::ptrace(libc::PTRACE_GETREGS, pid, 0, &mut regs as *mut _ as *mut libc::c_void);
                }

                // Only print on syscall entry, not exit
                if is_entry {
                    let syscall_nr = regs.regs[8] as i64;
                    syscall_count += 1;

                    if let Some(syscall_name) = sys_call_name(syscall_nr) {
                        eprintln!("[{}] syscall #{}: {} (args: x0={:#x}, x1={:#x}, x2={:#x})",
                            pid, syscall_count, syscall_name, regs.regs[0], regs.regs[1], regs.regs[2]);
                    } else {
                        eprintln!("[{}] syscall #{}: unknown({}) (args: x0={:#x}, x1={:#x}, x2={:#x})",
                            pid, syscall_count, syscall_nr, regs.regs[0], regs.regs[1], regs.regs[2]);
                    }
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
                eprintln!("strace: Process {} exited with status: {}", pid, exit_code);
                eprintln!("strace: Total syscalls traced: {}", syscall_count);
            } else if libc::WIFSIGNALED(status) {
                let signal = libc::WTERMSIG(status);
                eprintln!("strace: Process {} terminated by signal: {}", pid, signal);
                eprintln!("strace: Total syscalls traced: {}", syscall_count);
            } else {
                eprintln!("strace: Process {} stopped unexpectedly (status: {})", pid, status);
            }
        }
    }
}
