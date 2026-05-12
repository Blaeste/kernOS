use core::ptr::addr_of_mut;

// Pile ring 3 — même contrainte que KERNEL_INT_STACK : align(16) requis.
#[repr(align(16))]
struct UserStack([u8; 8192]);
static mut USER_STACK: UserStack = UserStack([0; 8192]);

static MSG: [u8; 14] = *b"Hello ring 3!\n";

/// Tache userspace - s'execute en ring 3
/// Appelle int 0x80 pour ecrire, puis exit
unsafe extern "C" fn user_task() {
    unsafe {
        core::arch::asm!(
            "mov rax, 0",
            "mov rdi, 1",
            "lea rsi, [{msg}]",
            "mov rdx, 14",
            "int 0x80",
            "mov rax, 1",
            "xor rdi, rdi",
            "int 0x80",
            msg = sym MSG,
            options(nostack)
        );
    }
}

/// Lance la tache userspace
pub fn spawn_user() {
    unsafe {
        let stack_top = addr_of_mut!(USER_STACK.0) as u64 + 8192;
        let entry = user_task as *const () as u64;
        crate::task::scheduler::spawn_ring3(entry, stack_top);
    }
}