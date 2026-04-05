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
5. ~~**Scancodes → ASCII + Shell**~~ ✅ — Clavier ASCII, mini-shell (help, clear, reboot, uptime)
6. ~~**Allocateur + Timer**~~ ✅ — Bump allocator 64Ko, PIT 100Hz
7. **Multitâche** — Scheduler de processus
8. **Système de fichiers** — Lire/écrire sur disque
9. **Shell texte** — Interface utilisateur en ligne de commande
10. **Stack graphique** — Framebuffer, window manager, GUI

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
