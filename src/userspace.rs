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

/// ELF hello embarqué (strippé ~18 Ko) - sert au 'run' sans argument et à 'install'.
pub static HELLO_ELF: &[u8] = include_bytes!(
    "../user/hello/target/x86_64-unknown-none/debug/hello"
);

/// Charge et lance le binaire ELF hello embarqué user/hello en ring 3 avec CR3 isolé.
pub fn spawn_elf() {
    
    // 1. Crée un espace d'adressage isolé poru ce processus
    let cr3 = crate::mem::paging::new_cr3();

    // 2. Chqrge l'ELF dans cet espace (pas dans kernel CR3)
    let entry = match crate::elf::load(HELLO_ELF, cr3) {
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

/// Lit un ELF depuis KFS par son nom, le charge dans un CR3 isolé et le lance.
pub fn spawn_elf_from_kfs(name: &[u8]) {

    // Buffer de chargement 64 Ko (= taille max d'un fichier KFS).
    // En static mut (BSS) et non sur la pile : la pile kernel ne fait que 16 Ko.
    static mut LOAD_BUF: [u8; 65536] = [0; 65536];
    let buf = unsafe { &mut *core::ptr::addr_of_mut!(LOAD_BUF) };

    let n = crate::fs::kfs::read(name, buf);
    if n == 0 {

        crate::println!("run: fichier introuvable");
        return;
    }

    // 1. Crée un espace d'adressage isolé poru ce processus
    let cr3 = crate::mem::paging::new_cr3();

    // 2. Chqrge l'ELF dans cet espace (pas dans kernel CR3)
    let entry = match crate::elf::load(&buf[..n], cr3) {
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
    crate::println!("run: '{}' lance ({} o), entry=0x{:x}", crate::klib::string::to_str(name), n, entry);
}

/// Copie l'ELF hello embarqué dans KFS sous le nom donné.
pub fn install_hello(name: &[u8]) {

    if crate::fs::kfs::create(name, HELLO_ELF) {

        crate::println!("install: '{}' ecrit ({} octets)", crate::klib::string::to_str(name), HELLO_ELF.len());
    } else {

        crate::println!("install: echec (disque plein ?)");
    }
}