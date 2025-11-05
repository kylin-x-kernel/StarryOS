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

    match unsafe { libc::fork() } {
        -1 => panic!("fork failed"),
        0 => {
            // child process
            unsafe {
                let _ = Command::new(command)
                    .args(command_args)
                    .pre_exec(|| {
                        libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0);
                        Ok(())
                    })
                    .exec();
            }
        }
        pid => {
            // parent process
            let mut status = 0;
            unsafe {
                libc::waitpid(pid, &mut status, 0);
            }
            while libc::WIFSTOPPED(status) {
                let mut regs: libc::user_regs_struct = unsafe { std::mem::zeroed() };
                unsafe {
                    libc::ptrace(libc::PTRACE_GETREGS, pid, 0, &mut regs as *mut _ as *mut libc::c_void);
                }
                let syscall_nr = regs.regs[8] as i64;
                if let Some(syscall_name) = sys_call_name(syscall_nr) {
                    println!("[{}] syscall: {}", pid, syscall_name);
                } else {
                    println!("[{}] syscall: {}", pid, syscall_nr);
                }
                unsafe {
                    libc::ptrace(libc::PTRACE_SYSCALL, pid, 0, 0);
                    libc::waitpid(pid, &mut status, 0);
                }
            }
        }
    }
}
