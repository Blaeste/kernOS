/// Table des appels système kernel

use crate::klib::string::to_str_or;

/// Numéros de syscall
pub const SYS_WRITE:        u64 = 0;
pub const SYS_EXIT:         u64 = 1;

/// Handler principal - appelé depuis le stub int 0x80
/// rax = numéro, rdi = arg1, rsi = arg2, rdx = arg3
pub extern "C" fn syscall_handler(num: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    match num {
        SYS_WRITE   => sys_write(a1, a2, a3),
        SYS_EXIT    => sys_exit(a1),
        _           => u64::MAX,                // syscall inconnu
    }
}

/// syscall 0 : write(fd, buf_ptr, len)
/// fd 1 = stdout (terminal)
fn sys_write(fd: u64, buf_ptr: u64, len: u64) -> u64 {

    crate::klib::debug::dbg_str("[SYS_WRITE] fd=");
    crate::klib::debug::dbg_hex(fd);
    crate::klib::debug::dbg_str(" ptr=");
    crate::klib::debug::dbg_hex(buf_ptr);
    crate::klib::debug::dbg_str(" len=");
    crate::klib::debug::dbg_hex(len);
    crate::klib::debug::dbg_ln("");

    if fd != 1 { return u64::MAX ;}

    let slice = unsafe {
        core::slice::from_raw_parts(buf_ptr as *const u8, len as usize)
    };
    let s = to_str_or(slice, "");
    if !s.is_empty() {
        crate::print!("{}", s);
    }
    len
}

/// syscall 1 : exit(code)
fn sys_exit(_code: u64) -> u64 {
    crate::task::scheduler::task_exit();
    0
}