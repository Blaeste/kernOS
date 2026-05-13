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

/// Lance la tache userspace (ring3, code kernel identity-mappé)
pub fn spawn_user() {
    unsafe {
        let stack_top = addr_of_mut!(USER_STACK.0) as u64 + 8192;
        let entry = user_task as *const () as u64;
        // Utilise le CR3 courant (kernel) - user_task vit dans l'identity map
        // dont les huge pages ont le bit USER positionné par boot.asm
        let cr3 = crate::mem::paging::read_cr3();
        crate::task::scheduler::spawn_ring3(entry, stack_top, cr3);
    }
}

/// Charge et lance le binaire ELF embarqué user/hello en ring 3 avec CR3 isolé.
pub fn spawn_elf() {
    // ELF embarqué à la compilation — chemin relatif depuis src/
    static ELF: &[u8] = include_bytes!(
        "../user/hello/target/x86_64-unknown-none/debug/hello"
    );

    // 1. Crée un espace d'adressage isolé poru ce processus
    let cr3 = crate::mem::paging::new_cr3();

    // 2. Chqrge l'ELF dans cet espace (pas dans kernel CR3)
    let entry = match crate::elf::load(ELF, cr3) {
        Some(e) => e,
        None => {
            crate::println!("run: ELF invalide");
            return;
        }
    };

    // 3. Alloue et mappe une pile user dans le CR3 du processus
    // Pile user à 0x40200000 (PDPT[1] → PD[1]) — séparée de l'ELF (PD[0])
    let stack_frame = crate::mem::frame::alloc().expect("run: plus de frames");
    crate::mem::paging::map_user_at(cr3, 0x40200000, stack_frame);

    // 4. Lance en ring 3 avec le CR3 isolé
    crate::task::scheduler::spawn_ring3(entry, 0x40201000, cr3);
    crate::println!("run: ELF lance, entry=0x{:x}", entry);
}