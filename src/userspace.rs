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

/// Charge et lance le binaire ELF embarqué user/hello en ring 3.
///
/// Étapes :
///   1. elf::load() — parse les segments PT_LOAD, alloue des frames, mappe en user
///   2. frame::alloc() + map_user() — une page de pile accessible depuis ring 3
///   3. spawn_ring3() — prépare le contexte iretq et active la tâche
pub fn spawn_elf() {
    // ELF embarqué à la compilation — chemin relatif depuis src/
    static ELF: &[u8] = include_bytes!(
        "../user/hello/target/x86_64-unknown-none/debug/hello"
    );

    let entry = match crate::elf::load(ELF) {
        Some(e) => e,
        None => {
            crate::println!("run: ELF invalide");
            return;
        }
    };

    // Alloue une page de pile user (4 Ko) mappé à 0x500000
    // le sommet de la pile = 0x501000 (la pile x86 grandit ver sle bas)
    let stack_frame = crate::mem::frame::alloc().expect("run: plsu de frames");
    crate::mem::paging::map_user(0x500000, stack_frame);

    crate::task::scheduler::spawn_ring3(entry, 0x501000);
    crate::println!("run: ELF lance, entry=0x{:x}", entry);
}