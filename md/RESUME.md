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

1. **Bootloader UEFI** — Écrire un bootloader qui démarre la machine
2. **Mode 64-bit + affichage texte** — Passer en long mode et afficher à l'écran
3. **Gestion mémoire** — Pagination, allocateur
4. **Interruptions** — IDT, gestion clavier/timer
5. **Multitâche** — Scheduler de processus
6. **Système de fichiers** — Lire/écrire sur disque
7. **Drivers** — Clavier, écran, disque
8. **Shell texte** — Interface utilisateur en ligne de commande
9. **Stack graphique** — Framebuffer, window manager, GUI

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
