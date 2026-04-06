use core::arch::asm;

const MAX_CMD: usize = 78;  // 80 colonnes - 2 pour "> "

static mut BUFFER: [u8; MAX_CMD] = [0; MAX_CMD];
static mut LEN: usize = 0;

pub fn prompt() {
    crate::print!("> ");
}

pub fn on_char(ch: u8) {
    match ch {
        b'\n' => {
            crate::println!();
            let len = unsafe { LEN };
            let cmd = unsafe { &BUFFER[..len] };
            execute(cmd);
            unsafe { LEN = 0; }
            prompt();
        }
        8 => {
            // Backspace
            unsafe {
                if LEN > 0 {
                    LEN -= 1;
                    (*core::ptr::addr_of_mut!(crate::io::vga::WRITER)).backspace();
                }
            }
        }
        _ => {
            let ok = unsafe {
                if LEN < MAX_CMD {
                    BUFFER[LEN] = ch;
                    LEN += 1;
                    true
                } else {
                    false
                }
            };
            if ok {
                crate::print!("{}", ch as char);
            }
        }
    }
}

fn execute(cmd: &[u8]) {
    if eq(cmd, b"help") {
        crate::println!("Commandes disponibles :");
        crate::println!("  help   - affiche cette aide");
        crate::println!("  clear  - efface l'ecran");
        crate::println!("  reboot - redemarre la machine");
        crate::println!("  uptime - temps depuis le demarrage");
        crate::println!("  memmap - affiche la carte memoire");
        crate::println!("  frames - affiche les frames libres");
        crate::println!("  pgtest - teste la pagination");
        crate::println!("  tasks  - affiche les taches actives");
    } else if eq(cmd, b"clear") {
        unsafe { (*core::ptr::addr_of_mut!(crate::io::vga::WRITER)).clear(); }
    } else if eq(cmd, b"reboot") {
        unsafe { asm!("out dx, al", in("dx") 0x64u16, in("al") 0xFEu8, options(nostack)); }
    } else if eq(cmd, b"uptime") {
        let ticks = crate::drivers::timer::get_ticks();
        let seconds = ticks / 100;
        crate::println!("Uptime: {} secondes ({} ticks)", seconds, ticks);
    } else if eq(cmd, b"memmap") {
        // Affiche la carte mémoire fournie par GRUB (zones RAM libres/réservées)
        let addr = unsafe { *core::ptr::addr_of!(crate::MULTIBOOT_ADDR) };
        crate::println!("== Memory Map ==");
        crate::multiboot::parse_memory_map(addr, |entry| {
            let kind = if entry.is_available() { "LIBRE" } else { "RESERVE" };
            let size_kb = entry.length / 1024;
            crate::println!("  {:#010x} - {} Ko - {}", entry.base_addr, size_kb, kind);
        });
    } else if eq(cmd, b"frames") {
        let free = crate::mem::frame::free_count();
        let free_kb = free * 4;
        crate::println!("Frames libres: {} ({} Ko)", free, free_kb);
    } else if eq(cmd, b"pgtest") {
        // 1. Alloue un frame physique libre
        let phys = crate::mem::frame::alloc().expect("pgtest: plus de frames");
        crate::println!("Frame alloue: {:#x}", phys);

        // 2. Mappe ce frame vers une adresse virtuelle au-dela du 1er Go
        //    0x4000_0000 = 1 Go, donc hors de l'identity map du boot
        let virt: u64 = 0x4000_0000;
        crate::mem::paging::map(virt, phys);
        crate::println!("Mappe: {:#x} -> {:#x}", virt, phys);

        // 3. Ecrit une valeur magique à l'adresse virtuelle
        unsafe { *(virt as *mut u64) = 0xCAFE_BABE; }
        crate::println!("Ecriture: *{:#x} = 0xCAFE_BABE", virt);

        // 4. Relit pour vérifier
        let val = unsafe { *(virt as *const u64) };
        crate::println!("Lecture:  *{:#x} = {:#x}", virt, val);

        // 5. Traduit l'adresse pour vérifier le mapping
        if let Some(p) = crate::mem::paging::translate(virt) {
            crate::println!("Translate: {:#x} -> {:#x} OK", virt, p);
        }

        // 6. Unmap + libère le frame
        crate::mem::paging::unmap(virt);
        crate::mem::frame::free(phys);
        crate::println!("Unmap + free OK");

    } else if eq(cmd, b"tasks") {
        crate::println!("== Taches ==");
        for i in 0..crate::task::MAX_TASKS {
            let active = unsafe { (*core::ptr::addr_of!(crate::task::TASKS))[i].active };
            if active {
                let current = unsafe { *core::ptr::addr_of!(crate::task::CURRENT) };
                let marker = if i == current { " <-" } else { "" };
                crate::println!(" [{}] active{}", i, marker);
            }
        }
    } else if cmd.is_empty() {
        // Entrer sans rien faire -> ne rien faire
    } else {
        crate::println!("commande inconnue: {}", core::str::from_utf8(cmd).unwrap_or("???"));
    }
}

// Compare deux slices d'octets (pas de == sur [u8] sans std)
fn eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] { return false; }
        i += 1;
    }
    true
}