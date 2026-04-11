/*
    ══════════════════════════════════════════════════════════
    SHELL — Interface en ligne de commande du kernel
    ══════════════════════════════════════════════════════════

    RÔLE
    ────
    Le shell est la couche interactive du kernel : il reçoit les frappes
    clavier caractère par caractère, les accumule dans un buffer, puis
    exécute la commande quand l'utilisateur appuie sur Entrée.

    FLOT D'EXÉCUTION
    ────────────────
    Interruption clavier (IRQ1)
      └─→ drivers/keyboard.rs     décode le scancode → caractère ASCII
            └─→ shell::on_char()  accumule dans BUFFER
                  └─→ shell::execute()  parse + dispatch si '\n'

    PARSING
    ───────
    La ligne saisie est découpée en deux parties dès le début d'execute() :

        "kfsWrite monFichier contenu"
         └──────┘ └──────────────────┘
           name           args

    - name  = tout ce qui précède le premier espace (ou toute la ligne si
              aucun espace) → identifie la commande de façon exacte avec eq()
    - args  = tout ce qui suit le premier espace → paramètres de la commande

    Grâce à cette approche, "kfsDel" ne matche jamais "kfsDelete" :
    eq() compare octet par octet avec longueur stricte, pas de préfixe.

    BUFFER
    ──────
    BUFFER est un tableau statique de MAX_CMD octets (unsafe, partagé via
    interruption). LEN indique combien d'octets sont remplis.
    MAX_CMD = 78 pour rester dans les 80 colonnes VGA (2 chars pour "> ").

    SÉCURITÉ
    ────────
    L'accès à BUFFER/LEN est unsafe car modifié depuis le handler d'IRQ.
    En mono-tâche/single-core (ce qu'est KernOS), aucune course critique
    n'est possible : l'IRQ ne peut pas interrompre execute() qui tourne
    déjà en contexte noyau avec les interruptions désactivées pendant le
    traitement PIC.
*/

use core::arch::asm;
use core::ptr::{addr_of, addr_of_mut};

use crate::klib::string::{eq, to_str, to_str_or, find_space};
use crate::{print, println};

/// Taille max d'une commande : 78 octets = 80 colonnes VGA - 2 (le prompt "> ")
const MAX_CMD: usize = 78;

/// Buffer de saisie - accumule les octets tapés au clavier jusqu'au '\n'
/// Modifié depuis le handler IRQ1, donc accès uniquement via unsafe.
static mut BUFFER: [u8; MAX_CMD] = [0; MAX_CMD];

/// Nombre d'octets valids dans BUFFER (0 =  ligne vide)
static mut LEN: usize = 0;

/// Affiche le prompt "> " au d2but d'une nouvelle ligne de saisie.
/// Appelée au boot et après chaque commande exécutée.
pub fn prompt() {
    print!("> ");
}

/// Point d'entrée depuis le driver clavier (appelé à chaque IRQ1).
/// Accumule les caractères dans BUFFER, gère le backspace (code 8),
/// et lance execute() quand '\n' est reçu.
pub fn on_char(ch: u8) {
    match ch {
        b'\n' => {
            println!();
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
                    if *addr_of!(crate::io::framebuffer::FB_ACTIVE) {
                        (*addr_of_mut!(crate::io::framebuffer::FB_WRITER)).backspace();
                        crate::io::framebuffer::swap_buffers();
                    } else {
                        (*addr_of_mut!(crate::io::vga::WRITER)).backspace();
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
                print!("{}", ch as char);
            }
        }
    }
}

/// Parse et dispatch une commande.
/// Découpe cmd en (name, args) sur le premier espace,
/// puis matche name exactement — aucun risque de collision de préfixe.
fn execute(cmd: &[u8]) {
    let (name, args) = match find_space(cmd) {
        Some(pos)   => (&cmd[..pos], &cmd[pos+1..]),
        None        => (cmd, &b""[..]),
    };

    if eq(name, b"help") {
        println!(concat!(
            "Commandes disponibles :\n",
            "  help     - affiche cette aide\n",
            "  clear    - efface l'ecran\n",
            "  reboot   - redemarre la machine\n",
            "  uptime   - temps depuis le demarrage\n",
            "  memmap   - affiche la carte memoire\n",
            "  frames   - affiche les frames libres\n",
            "  pgtest   - teste la pagination\n",
            "  tasks    - affiche les taches actives\n",
            "  ls       - liste les fichiers\n",
            "  cat      - affiche un fichier (cat nom)\n",
            "  write    - cree un fichier (write nom contenu)\n",
            "  rm       - supprime un fichier (rm nom)\n",
            "  ataRead  - lit le secteur 0\n",
            "  ataWrite - ecrit dans le secteur 0\n",
            "  kfsLs    - liste fichiers disque\n",
            "  kfsWrite - ecrit sur disque (kfsWrite nom contenu)\n",
            "  kfsRead  - lit depuis disque (kfsRead nom)\n",
            "  kfsDel   - supprime un fichier disque (kfsDel nom)",
            "  layout   - change le layout clavier (layout azerty|qwerty)",
            "  heapInfo - affiche l'état actuel de la heap",
        ));
    }

    else if eq(name, b"clear") {
        unsafe {
            if *addr_of!(crate::io::framebuffer::FB_ACTIVE) {
                (*addr_of_mut!(crate::io::framebuffer::FB_WRITER)).clear_screen();
                crate::ui::window::draw_all();
                crate::ui::taskbar::draw();
                crate::io::framebuffer::swap_buffers();
            } else {
                (*addr_of_mut!(crate::io::vga::WRITER)).clear();
            }
        }
    }

    else if eq(name, b"reboot") {
        unsafe { asm!("out dx, al", in("dx") 0x64u16, in("al") 0xFEu8, options(nostack)); }
    }

    else if eq(name, b"uptime") {
        let ticks = crate::drivers::timer::get_ticks();
        let seconds = ticks / 100;
        println!("Uptime: {} secondes ({} ticks)", seconds, ticks);
    }

    else if eq(name, b"memmap") {
        // Affiche la carte mémoire fournie par GRUB (zones RAM libres/réservées)
        let addr = unsafe { *addr_of!(crate::MULTIBOOT_ADDR) };
        println!("== Memory Map ==");
        crate::multiboot::parse_memory_map(addr, |entry| {
            let kind = if entry.is_available() { "LIBRE" } else { "RESERVE" };
            let size_kb = entry.length / 1024;
            println!("  {:#010x} - {} Ko - {}", entry.base_addr, size_kb, kind);
        });
    }

    else if eq(name, b"frames") {
        let free = crate::mem::frame::free_count();
        let free_kb = free * 4;
        println!("Frames libres: {} ({} Ko)", free, free_kb);
    }

    else if eq(name, b"pgtest") {
        // 1. Alloue un frame physique libre
        let phys = crate::mem::frame::alloc().expect("pgtest: plus de frames");
        println!("Frame alloue: {:#x}", phys);

        // 2. Mappe ce frame vers une adresse virtuelle au-dela du 1er Go
        //    0x4000_0000 = 1 Go, donc hors de l'identity map du boot
        let virt: u64 = 0x4000_0000;
        crate::mem::paging::map(virt, phys);
        println!("Mappe: {:#x} -> {:#x}", virt, phys);

        // 3. Ecrit une valeur magique à l'adresse virtuelle
        unsafe { *(virt as *mut u64) = 0xCAFE_BABE; }
        println!("Ecriture: *{:#x} = 0xCAFE_BABE", virt);

        // 4. Relit pour vérifier
        let val = unsafe { *(virt as *const u64) };
        println!("Lecture:  *{:#x} = {:#x}", virt, val);

        // 5. Traduit l'adresse pour vérifier le mapping
        if let Some(p) = crate::mem::paging::translate(virt) {
            println!("Translate: {:#x} -> {:#x} OK", virt, p);
        }

        // 6. Unmap + libère le frame
        crate::mem::paging::unmap(virt);
        crate::mem::frame::free(phys);
        println!("Unmap + free OK");

    }

    else if eq(name, b"tasks") {
        println!("== Taches ==");
        for i in 0..crate::task::scheduler::MAX_TASKS {
            let active = unsafe { (*addr_of!(crate::task::scheduler::TASKS))[i].active };
            if active {
                let current = unsafe { *addr_of!(crate::task::scheduler::CURRENT) };
                let marker = if i == current { " <-" } else { "" };
                println!(" [{}] active{}", i, marker);
            }
        }
    }

    else if eq(name, b"ls") {
        crate::fs::ramfs::ls();
    }

    else if eq(name, b"cat") {
        crate::fs::ramfs::cat(args);
    }

    else if eq(name, b"write") {
        if let Some(pos) = find_space(args) {
            crate::fs::ramfs::create(&args[..pos], &args[pos+1..]);
        } else {
            println!("usage: write <nom> <contenu>");
        }
    }

    else if eq(name, b"rm") {
        crate::fs::ramfs::rm(args);
    }

    else if eq(name, b"ataRead") {
        let mut buf = [0u16; 256];
        crate::drivers::ata::read_sector(0, &mut buf);
        println!("Secteur 0 (16 premier octets):");
        let mut i = 0;
        while i < 8 {
            let word = buf[i];
            println!("  [{:02}] {:02x} {:02x}", i*2, word & 0xFF, (word >> 8) & 0xFF);
            i += 1;
        }
    }

    else if eq(name, b"ataWrite") {
        let mut buf = [0u16; 256];
        // Ecrit "KernOS" dans les premiers octets
        buf[0] = 0x654B;
        buf[1] = 0x6E72;
        buf[2] = 0x534F;
        crate::drivers::ata::write_sector(0, &buf);
        println!("Secteur 0 ecrit OK");
    }

    else if eq(name, b"kfsLs") {
        let mut count = 0u32;
        crate::fs::kfs::list(|name, size| {
            println!("   {} ({} o)", to_str(name), size);
            count += 1;
        });
        if count == 0 {
            println!("(vide)");
        }
    }

    else if eq(name, b"kfsWrite") {
        if let Some(pos) = find_space(args) {
            let ok = crate::fs::kfs::create(&args[..pos], &args[pos+1..]);
            if ok { println!("OK"); } else { println!("Erreur (plein?)"); }
        } else {
            println!("usage kfsWrite <nom> <contenu>");
        }
    }

    else if eq(name, b"kfsRead") {
        let mut buf = [0u8; 512];
        let sz = crate::fs::kfs::read(args, &mut buf);
        if sz == 0 {
            println!("fichier introuvable");
        } else {
            println!("{}", to_str_or(&buf[..sz], "(binaire)"));
        }
    }

    else if eq(name, b"kfsDel") {
        if crate::fs::kfs::delete(args) {
            println!("OK");
        } else {
            println!("fichier introuvable");
        }
    }

    else if eq(name, b"layout") {
        if eq(args, b"azerty") {
            unsafe { crate::drivers::keyboard::LAYOUT = crate::drivers::keyboard::Layout::Azerty; }
            println!("Layout: AZERTY");
        } else if eq(args, b"qwerty") {
            unsafe { crate::drivers::keyboard::LAYOUT = crate::drivers::keyboard::Layout::Qwerty; }
            println!("Layout: QWERTY");
        } else {
            println!("usage: layout azerty|qwerty");
        }
    }

    else if eq(name, b"heapInfo") {
        let (total, free, blocks) = crate::mem::allocator::heap_info();
        println!("Heap: {} Ko total, {} Ko libre, {} blocks libres", total / 1024, free / 1024, blocks);
    }

    else if name.is_empty() {
        // Ne rien faire
    }

    else {
        println!("commande inconnue: {}", to_str(name));
    }
}
