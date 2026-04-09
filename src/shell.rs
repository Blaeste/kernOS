use core::arch::asm;

use crate::klib::string::{eq, to_str, to_str_or, starts_with, find_space};

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
                    if *core::ptr::addr_of!(crate::io::framebuffer::FB_ACTIVE) {
                        (*core::ptr::addr_of_mut!(crate::io::framebuffer::FB_WRITER)).backspace();
                        crate::io::framebuffer::swap_buffers();
                    } else {
                        (*core::ptr::addr_of_mut!(crate::io::vga::WRITER)).backspace();
                    }
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
        crate::println!("  ls     - liste les fichiers");
        crate::println!("  cqt    - affiche un fichier (cat nom)");
        crate::println!("  write  - cree un fichier (write nom contenu)");
        crate::println!("  rm     - supprime un fichier (rm nom)");
        crate::println!("  ataRead  - lit le secteur 0");
        crate::println!("  ataWrite - ecrit dans le secteur 0");
    } else if eq(cmd, b"clear") {
        unsafe {
            if *core::ptr::addr_of!(crate::io::framebuffer::FB_ACTIVE) {
                (*core::ptr::addr_of_mut!(crate::io::framebuffer::FB_WRITER)).clear_screen();
                crate::ui::window::draw_all();
                crate::ui::taskbar::draw();
                crate::io::framebuffer::swap_buffers();
            } else {
                (*core::ptr::addr_of_mut!(crate::io::vga::WRITER)).clear();
            }
        }
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
        for i in 0..crate::task::scheduler::MAX_TASKS {
            let active = unsafe { (*core::ptr::addr_of!(crate::task::scheduler::TASKS))[i].active };
            if active {
                let current = unsafe { *core::ptr::addr_of!(crate::task::scheduler::CURRENT) };
                let marker = if i == current { " <-" } else { "" };
                crate::println!(" [{}] active{}", i, marker);
            }
        }
    } else if eq(cmd, b"ls") {
        crate::fs::ramfs::ls();
    } else if starts_with(cmd, b"cat") {
        crate::fs::ramfs::cat(&cmd[4..]);
    } else if starts_with(cmd, b"write") {
        let rest = &cmd[6..];
        if let Some(pos) = find_space(rest) {
            crate::fs::ramfs::create(&rest[..pos], &rest[pos+1..]);
        } else {
            crate::println!("usage: write <nom> <contenu>");
        }
    } else if starts_with(cmd, b"rm") {
        crate::fs::ramfs::rm(&cmd[3..]);
    } else if eq(cmd, b"ataRead") {
        let mut buf = [0u16; 256];
        crate::drivers::ata::read_sector(0, &mut buf);
        crate::println!("Secteur 0 (16 premier octets):");
        let mut i = 0;
        while i < 8 {
            let word = buf[i];
            crate::println!("  [{:02}] {:02x} {:02x}", i*2, word & 0xFF, (word >> 8) & 0xFF);
            i += 1;
        }
    } else if eq(cmd, b"ataWrite") {
        let mut buf = [0u16; 256];
        // Ecrit "KernOS" dans les premiers octets
        buf[0] = 0x654B;
        buf[1] = 0x6E72;
        buf[2] = 0x534F;
        crate::drivers::ata::write_sector(0, &buf);
        crate::println!("Secteur 0 ecrit OK");
    } else if eq(cmd, b"kfsLs") {
        crate::fs::kfs::list(|name, size| {
            crate::println!("   {} ({} o)", to_str(name), size);
        });
    } else if starts_with(cmd, b"kfsWrite") {
        let rest = &cmd[9..];
        if let Some(pos) = find_space(rest) {
            let ok = crate::fs::kfs::create(&rest[..pos], &rest[pos+1..]);
            if ok { crate::println!("OK"); } else { crate::println!("Erreur (plein?)"); }
        } else {
            crate::println!("usage kfsWrite <nom> <contenu>");
        }
    } else if starts_with(cmd, b"kfsRead") {
        let name = &cmd[8..];
        let mut buf = [0u8; 512];
        let sz = crate::fs::kfs::read(name, &mut buf);
        if sz == 0 {
            crate::println!("fichier introuvable");
        } else {
            crate::println!("{}", to_str_or(&buf[..sz], "(binaire)"));
        }
    } else if cmd.is_empty() {
        // Entrer sans rien faire -> ne rien faire
    } else {
        crate::println!("commande inconnue: {}", to_str(cmd));
    }
}