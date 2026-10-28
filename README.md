# kernOS

Un système d'exploitation desktop écrit from scratch en Rust, en solo : du bootloader au
gestionnaire de fenêtres, en passant par la mémoire virtuelle, le multitâche et l'espace
utilisateur. Le nom vient de l'alsacien : *Kern*, le noyau.

Noyau monolithique x86_64, Rust `no_std` avec un minimum d'assembleur, testé dans QEMU.

## Ce qui fonctionne

- **Démarrage** : bootloader Multiboot2 en assembleur, passage du mode 32 bits au mode long
  64 bits, GDT et TSS.
- **Interruptions et matériel** : IDT complète avec les exceptions CPU, PIC 8259, timer PIT
  à 100 Hz, clavier PS/2 (AZERTY et QWERTY), souris PS/2, driver disque ATA PIO.
- **Mémoire** : lecture de la memory map Multiboot2, allocateur de frames physiques
  (bitmap), pagination à 4 niveaux, allocateur de tas free-list avec fusion des blocs.
- **Multitâche** : ordonnanceur round-robin préemptif, changement de contexte, un espace
  d'adressage (CR3) par processus, spinlocks.
- **Espace utilisateur** : processus isolés en ring 3, chargeur ELF64, 10 syscalls (`write`,
  `exit`, `getpid`, `yield`, `open`, `read`, `close`, `kill`, `wait`, `pipe`), pipes entre
  processus, et `libkern`, une petite bibliothèque `no_std` pour les programmes utilisateur.
- **Système de fichiers** : KFS v2, un format maison sur disque (FAT chaînée, 64 fichiers).
- **Interface graphique** : framebuffer 1080p avec double buffering et dirty rectangles,
  gestionnaire de fenêtres (déplacement, redimensionnement, z-order, focus), barre des tâches
  avec horloge et lanceur, explorateur de fichiers, terminal ANSI 16 couleurs, éditeur de
  texte intégré, shell de plus de 25 commandes.

Prochaine étape : le réseau (VirtIO net, UDP, TCP, HTTP).

## Organisation du code

```
src/
├── boot.asm        bootloader Multiboot2, passage 32 → 64 bits
├── main.rs         point d'entrée du noyau
├── interrupt.rs    IDT, exceptions, IRQ
├── gdt.rs          GDT et TSS
├── syscall.rs      les 10 syscalls
├── elf.rs          chargeur ELF64
├── userspace.rs    lancement des processus en ring 3
├── ipc.rs          pipes
├── mem/            allocateurs (frames, tas) et pagination
├── task/           ordonnanceur
├── drivers/        PIC, clavier, souris, ATA, timer
├── fs/             KFS et système de fichiers en RAM
├── io/             VGA, framebuffer, police, stdin
├── ui/             fenêtres, barre des tâches, explorateur, éditeur
└── klib/           utilitaires no_std
user/
├── hello/          programme utilisateur qui teste tous les syscalls
└── libkern/        bibliothèque pour les programmes utilisateur
md/                 37 notes de conception, une par étape
```

## Lancer kernOS

Prérequis (Debian / Ubuntu) :

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install qemu-system-x86 nasm grub-pc-bin xorriso
```

La toolchain nightly, la cible `x86_64-unknown-none` et `rust-src` sont fixées par
`rust-toolchain.toml` : rustup les installe au premier build.

```bash
./run.sh        # compile le programme utilisateur, le noyau, l'ISO, puis lance QEMU
```

Sous Windows : `run.ps1`.

## Documentation

Le dossier [`md/`](md/) retrace la construction étape par étape, du noyau de base
(`01-kernel-base.md`) à l'espace utilisateur (`35-ring3-userspace.md`). La vue d'ensemble
et les décisions sont dans [`md/RESUME.md`](md/RESUME.md).

## Licence

Tous droits réservés : le code est public pour être consulté, pas réutilisé.
Voir [LICENSE](LICENSE).
