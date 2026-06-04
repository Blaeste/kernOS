# KernOS — Résumé du Projet

## Le Projet

**KernOS** — Un OS desktop Linux-like écrit entièrement en Rust, from scratch.
Le nom vient de l'alsacien : "Kern" = noyau.

## Décisions Prises

| Choix | Décision |
|---|---|
| **Langage** | Rust (full, avec un minimum d'ASM x86_64) |
| **Architecture** | x86_64 (PC 64 bits) |
| **Type de noyau** | Monolithique (comme Linux) |
| **Bootloader** | Custom, écrit from scratch |
| **Cible** | Desktop avec interface graphique |
| **Approche** | Progressive, étape par étape |
| **Équipe** | Solo (potentiellement 2) |

## Environnement de Développement

| Outil | Usage |
|---|---|
| **Rust nightly** | Compilateur — nightly requis pour les features OS dev (`#![no_std]`, inline ASM...) |
| **rust-src** | Sources de la lib standard, nécessaires pour recompiler `core`/`alloc` pour notre cible bare-metal |
| **x86_64-unknown-none** | Cible de compilation : machine nue, sans OS |
| **NASM** | Assembleur x86_64 (déjà installé) |
| **QEMU** | Émulateur de PC — permet de tester KernOS dans une fenêtre sans redémarrer la machine |
| **Cargo** | Build system de Rust |

## Feuille de Route

1. ~~**Bootloader**~~ ✅ — Boot ASM Multiboot2, passage 32→64 bit
2. ~~**Affichage VGA**~~ ✅ — print!/println! avec formatage
3. ~~**Interruptions**~~ ✅ — IDT complète, exceptions CPU
4. ~~**PIC + Clavier + Souris**~~ ✅ — PIC 8259, IRQ, scancodes, souris PS/2
5. ~~**Scancodes → ASCII + Shell**~~ ✅ — Clavier ASCII, mini-shell complet
6. ~~**Allocateur + Timer**~~ ✅ — Free-list allocator 256Ko, PIT 100Hz
7. ~~**Mémoire physique**~~ ✅ — Memory map Multiboot2, frame allocator bitmap
8. ~~**Pagination**~~ ✅ — Page tables 4 niveaux, map/unmap/translate/map_user_at/new_cr3
9. ~~**Multitâche**~~ ✅ — Scheduler round-robin, context switch, CR3 par processus
10. ~~**Système de fichiers**~~ ✅ — KFS v2 : FAT chaînée, 64 fichiers, 64 Ko max
11. ~~**Stack graphique**~~ ✅ — Framebuffer 1080p, double buffering, fenêtres, drag & drop
12. ~~**Userspace ring 3**~~ ✅ — Phase A : ELF loader, Phase B : isolation CR3, Phase C : 7 syscalls
13. ~~**libkern**~~ ✅ — crate no_std userspace, bump allocator 16 Ko, Vec/String/Box
14. ~~**IPC & Signaux**~~ ✅ — pipes circulaires, wait(pid), kill(pid,9), 10 syscalls au total
15. ~~**Shell amélioré + UI**~~ ✅ (partiel) — terminal ANSI 16 couleurs, fenêtres redimensionnables, éditeur intégré, launcher taskbar. Reste : pipes shell `|`, stdin bloquant
16. **Réseau** — VirtIO net, UDP, TCP, HTTP

## Architecture du Code

```
src/
├── main.rs            point d'entrée, macros print!/println!
├── boot.asm           bootloader ASM 32→64 bit, Multiboot2
├── interrupt.rs       IDT, handlers exceptions + IRQ stubs naked
├── multiboot.rs       parsing tags Multiboot2 (memory map, framebuffer)
├── shell.rs           mini-shell (buffer, commandes, 25+ commandes)
├── syscall.rs         10 syscalls : write/exit/getpid/yield/open/read/close/kill/wait/pipe (read(0) bloquant)
├── ipc.rs             pipes circulaires 256 o (jusqu'à 4), alloc_pipe/read/write
├── gdt.rs             GDT + TSS (segments kernel/user, rsp0)
├── elf.rs             ELF64 loader (PT_LOAD, map_user_at, BSS zero)
├── userspace.rs       spawn_user / spawn_elf / spawn_elf_from_kfs (ring 3)
├── mem/
│   ├── allocator.rs   free-list allocator 256Ko
│   ├── frame.rs       frame allocator bitmap (pages 4Ko)
│   └── paging.rs      tables 4 niveaux, new_cr3, map_user_at, translate_in
├── task/
│   └── scheduler.rs   round-robin, Task{cr3, fds}, NEXT_CR3, spawn_ring3
├── drivers/
│   ├── pic.rs         PIC 8259 init + EOI
│   ├── keyboard.rs    scancode → ASCII, AZERTY/QWERTY
│   ├── mouse.rs       souris PS/2, curseur graphique, drag
│   ├── ata.rs         driver ATA PIO (read/write secteur)
│   └── timer.rs       PIT 100Hz, compteur ticks
├── fs/
│   ├── kfs.rs         KFS v2 : FAT chaînée, 64 fichiers, 64 Ko max
│   └── ramfs.rs       filesystem RAM temporaire
├── io/
│   ├── vga.rs         affichage VGA texte
│   ├── framebuffer.rs framebuffer 1080p, double buffering, PRINT_LOCK
│   ├── font.rs        police bitmap 8×16
│   └── stdin.rs       ring buffer 256 o line-buffered, wake sur '\n'
├── ui/
│   ├── window.rs      window manager (z-order, drag, resize, focus)
│   ├── taskbar.rs     barre des tâches + horloge + launcher
│   ├── filewin.rs     fenêtre fichiers KFS cliquable
│   └── editor.rs      éditeur de texte (edit <f>, Ctrl+S, Escape)
└── klib/              utilitaires no_std (string, mem, cmp, debug)

user/
└── hello/             crate ELF userspace — teste tous les syscalls
```

## Commandes d'Installation

```bash
# Linux (Debian/Ubuntu)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install qemu-system-x86 nasm grub-pc-bin xorriso

# Configurer Rust pour l'OS dev
# (rust-toolchain.toml à la racine force déjà nightly automatiquement)
rustup component add rust-src    # Sources pour recompiler les libs de base
rustup target add x86_64-unknown-none  # Cible bare-metal sans OS
```

## Ressources

- [Writing an OS in Rust](https://os.phil-opp.com/) — Tutoriel de référence
- [OSDev Wiki](https://wiki.osdev.org/) — Documentation technique
- [Redox OS](https://www.redox-os.org/) — OS complet en Rust, source d'inspiration
