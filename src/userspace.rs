/// Pile user - 8 ko static poru l'instant
static mut USER_STACK:      [u8; 8192] = [0; 8192];

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
        let stack_top = core::ptr::addr_of_mut!(USER_STACK).cast::<u8>().add(8192) as u64;
        let entry = user_task as *const () as u64;
        crate::task::scheduler::spawn_ring3(entry, stack_top);
    }
}