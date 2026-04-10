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
5. ~~**Scancodes → ASCII + Shell**~~ ✅ — Clavier ASCII, mini-shell (help, clear, reboot, uptime, memmap, frames)
6. ~~**Allocateur + Timer**~~ ✅ — Bump allocator 64Ko, PIT 100Hz
7. ~~**Mémoire physique**~~ ✅ — Memory map Multiboot2, frame allocator bitmap
8. ~~**Pagination**~~ ✅ — Page tables 4 niveaux, map/unmap/translate
9. ~~**Multitâche**~~ ✅ — Scheduler round-robin, context switch, spawn
10. **Système de fichiers** — Lire/écrire sur disque
11. ~~**Stack graphique**~~ ✅ — Framebuffer 800×600, police bitmap, FbWriter, double buffering, curseur souris, fenêtres, drag & drop, dirty rectangles

## Architecture du Code

```
src/
├── main.rs            point d'entrée, macros print!/println!
├── boot.asm           bootloader ASM 32→64 bit, Multiboot2
├── interrupt.rs       IDT, handlers exceptions + IRQ stubs
├── multiboot.rs       parsing tags Multiboot2 (memory map)
├── shell.rs           mini-shell (buffer, commandes)
├── mem/
│   ├── allocator.rs   bump allocator (heap 64Ko pour String, Vec)
│   └── frame.rs       frame allocator bitmap (pages 4Ko)
├── drivers/
│   ├── pic.rs         PIC 8259 init + EOI
│   ├── keyboard.rs    scancode → ASCII, gestion Shift
│   ├── mouse.rs       souris PS/2, curseur graphique, drag
│   └── timer.rs       PIT 100Hz, compteur ticks
├── io/
│   ├── vga.rs         affichage VGA (clear, scroll, backspace)
│   ├── framebuffer.rs framebuffer graphique, double buffering, FbWriter
│   └── font.rs        police bitmap VGA 8×16
└── ui/
    └── window.rs      fenêtres graphiques, drag & drop
```

## Commandes d'Installation

```bash
# Installer Rust
winget install Rustlang.Rustup

# Installer QEMU
winget install SoftwareFreedomConservancy.QEMU

# Configurer Rust pour l'OS dev
rustup default nightly           # Version expérimentale (features OS dev)
rustup component add rust-src    # Sources pour recompiler les libs de base
rustup target add x86_64-unknown-none  # Cible bare-metal sans OS
```

## Ressources

- [Writing an OS in Rust](https://os.phil-opp.com/) — Tutoriel de référence
- [OSDev Wiki](https://wiki.osdev.org/) — Documentation technique
- [Redox OS](https://www.redox-os.org/) — OS complet en Rust, source d'inspiration
