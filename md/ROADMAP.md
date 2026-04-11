# KernOS — Roadmap

## Phase 1 — Fondations kernel
- [x] **Syscall** — int 0x80, frontière kernel/user ✅
  - syscall write (affiche depuis userspace)
  - syscall exit (termine un processus)
  - table des syscalls extensible
- [x] **GDT + TSS** — nouvelle GDT complète, TSS avec RSP0, segments user ✅
- [x] **Debug amélioré** — port 0xE9, exceptions avec RIP/CR2, panic handler ✅
- [x] **Première tâche ring 3** — iretq vers userspace, syscall depuis ring 3 ✅
- [ ] **ELF loader** — parse un binaire ELF depuis KFS, mappe en mémoire, exécute en ring 3

## Phase 2 — Isolation et communication
- [ ] **Processus isolés** — chaque processus a son propre espace d'adressage (paging)
- [ ] **IPC** — pipe, signaux entre processus
- [ ] **KFS v2** — dossiers, permissions, métadonnées

## Phase 3 — Drivers et réseau
- [ ] **VirtIO disk** — remplace ATA, bien plus rapide sous QEMU
- [ ] **Stack réseau** — UDP d'abord, puis TCP/IP
- [ ] **VirtIO net** — driver réseau QEMU

## Phase 4 — UI et confort
- [ ] **Éditeur de texte** — ouvre KFS, édite, Ctrl+S sauvegarde
- [ ] **Terminal amélioré** — couleurs ANSI, scroll molette
- [ ] **Fenêtres redimensionnables**
- [ ] **Shell amélioré** — pipes, redirections, variables

## Fait ✅
- [x] Boot ASM + long mode (64-bit)
- [x] VGA textuelle
- [x] IDT + PIC + IRQ
- [x] Framebuffer + double buffering + dirty rect
- [x] Police bitmap 8x16
- [x] Window manager (drag, z-order, focus, fermeture)
- [x] Taskbar + horloge
- [x] Driver clavier (AZERTY/QWERTY switchable)
- [x] Driver souris + curseur
- [x] Driver ATA (lecture/écriture disque)
- [x] KFS filesystem persisté sur disque
- [x] Shell interactif (parsing name/args)
- [x] Scroll terminal avec historique (PgUp/PgDn)
- [x] Fenêtre Fichiers cliquable (clic → contenu KFS dans terminal)
- [x] Free List allocator 256 Ko (alloc + free + fusion)
- [x] Multitâche préemptif (context switch, round-robin, spawn, task_exit)
- [x] SpinLock (klib/sync, protection println!)
- [x] Syscalls int 0x80 (write + exit)
- [x] GDT + TSS (5 segments + TSS, selecteurs propagés)
- [x] Debug ameliore (port 0xE9, exceptions GPF/PF avec RIP/CR2, panic handler, clamp font/swap)
- [x] Ring 3 userspace (spawn_ring3, pile dédiée TSS, pages U/S, Hello ring 3!)
