# KernOS — Roadmap

## État actuel — Ring 3 fonctionnel

Le kernel est mature. Le premier code userspace tourne en ring 3 via `iretq` + `int 0x80`.
Tout ce qui suit construit *au-dessus* de cette frontière kernel/user.

### Fait ✅

| Module | Détail |
|--------|--------|
| Boot ASM + Long mode 64-bit | Multiboot2, passage 32→64 |
| VGA + print!/println! | Affichage texte kernel |
| IDT + PIC + IRQ | Interruptions CPU complètes |
| Clavier PS/2 (AZERTY/QWERTY) + Souris | Input complet |
| Shell interactif | Parsing commandes, historique |
| Free List allocator 256 Ko | alloc + free + fusion |
| Timer PIT 100 Hz | Base temps kernel |
| Frame allocator bitmap | Gestion physique de la RAM |
| Pagination 4 niveaux | map / unmap / translate |
| Multitâche préemptif | Round-robin, context switch, spawn |
| Framebuffer 800×600 | Double buffering, dirty rect |
| Window manager | Drag & drop, z-order, focus, fermeture |
| Taskbar + horloge | UI desktop basique |
| Driver ATA | Lecture/écriture disque |
| KFS v1 | 8 fichiers × 512 octets, persisté sur disque |
| Scroll terminal | PgUp/PgDn, historique |
| SpinLock | Protection accès concurrent |
| Syscalls (int 0x80) | write + exit |
| GDT + TSS complète | Segments kernel + user |
| Debug amélioré | Port 0xE9, exceptions GPF/PF avec RIP/CR2, panic handler |
| Ring 3 userspace | spawn_ring3, iretq, "Hello ring 3!" |

---

## Phase A — ELF Loader
> **Priorité absolue.** Sans ça, tout code userspace doit être linké dans le kernel.

- [ ] Parser ELF64 — lire les headers `Elf64_Phdr`, identifier les segments `PT_LOAD`
- [ ] Allouer des frames user — pour chaque segment : frame allocator → pages U/S mappées
- [ ] Copier les segments — charger le code/data à leur adresse virtuelle cible
- [ ] Lancer en ring 3 — `spawn_ring3(elf_entry, user_stack)` avec le bon espace d'adressage
- [ ] Stocker les ELF dans KFS — un fichier KFS = un binaire exécutable

**Milestone :** `run monprog` dans le shell charge et exécute un ELF depuis le disque en ring 3.

---

## Phase B — Isolation mémoire
> Actuellement tous les processus partagent le même CR3 — catastrophique pour la stabilité.

- [ ] CR3 par processus — chaque `Task` a sa propre `PML4` (clone du kernel mapping + pages user)
- [ ] Switch CR3 au context switch — `mov cr3, rax` dans `irq_timer_stub`
- [ ] Libération à task_exit — free pages user + PML4 au décès du processus
- [ ] Tester l'isolation — deux processus avec mêmes adresses virtuelles, données différentes

**Milestone :** un process crashé ne corrompt pas les autres.

---

## Phase C — Syscalls complets
> Aujourd'hui : 2 syscalls. Il en faut ~10 pour qu'un programme soit utile.

- [ ] `read(fd, buf, len)` — lire stdin (clavier)
- [ ] `open(path)` → fd — ouvrir un fichier KFS
- [ ] `close(fd)` — fermer un fd
- [ ] `read_file(fd, buf, len)` — lire depuis KFS
- [ ] `write_file(fd, buf, len)` — écrire dans KFS
- [ ] `exec(path)` — remplacer le processus par un ELF
- [ ] `getpid()` — identifiant de processus
- [ ] `yield()` — céder le CPU volontairement
- [ ] `sleep(ms)` — attente temporisée

**Milestone :** un programme userspace peut lire/écrire des fichiers sans accès kernel direct.

---

## Phase D — KFS v2
> KFS v1 est trop limité pour accueillir des programmes (8 fichiers × 512 octets).

- [ ] Taille variable — fichiers multi-secteurs avec liste chaînée de blocs
- [ ] Plus de fichiers — passer à 64+ entrées (index bitmap)
- [ ] Sous-dossiers — entrées de type `DIR` avec listing récursif
- [ ] Métadonnées — timestamp, permissions rwx basiques
- [ ] Cache secteurs — éviter de relire ATA à chaque accès

**Milestone :** `kfsLs /bin` affiche les exécutables, fichiers > 512 octets lisibles.

---

## Phase E — User library (libkern)
> Sans ça, chaque programme réimplémente les appels système.

- [ ] `libkern` — crate `no_std` linkée statiquement dans les user programs
- [ ] Wrappers syscalls — `fn write(s: &str)`, `fn open(path) -> Fd`, etc.
- [ ] Allocateur user — bump ou free list pour les programmes userspace
- [ ] String / Vec en userspace — avec l'allocateur user

**Milestone :** un programme Rust userspace peut `use libkern::*;` et faire des I/O.

---

## Phase F — IPC & Signaux
> Pour que les programmes communiquent.

- [ ] Pipe — buffer circulaire kernel partagé entre 2 fd (un writer, un reader)
- [ ] Signaux basiques — `SIGKILL` (termine), `SIGSTOP` / `SIGCONT`
- [ ] `wait(pid)` — syscall pour attendre la fin d'un enfant
- [ ] `fork()` simplifié — clone CR3 + task (copy-on-write optionnel)

---

## Phase G — Shell et UI
> Rendre l'OS utilisable au quotidien.

- [ ] Shell amélioré — pipes (`|`), redirections (`>`, `<`), variables `$VAR`
- [ ] Terminal ANSI — couleurs 16, bold, reset (`\e[31m` etc.)
- [ ] Fenêtres redimensionnables — coin drag resize
- [ ] Éditeur de texte intégré — ouvre un fichier KFS, édite, `Ctrl+S` sauvegarde
- [ ] Launcher — clic sur icône → `exec` d'un ELF

---

## Phase H — Réseau
> Optionnel mais impressionnant.

- [ ] VirtIO net — driver réseau QEMU (rapide, moderne)
- [ ] Stack UDP — IP + UDP + checksum
- [ ] Stack TCP — 3-way handshake, retransmission
- [ ] Serveur HTTP minimal — `curl http://kernos:8080/` depuis l'hôte

---

## Ordre recommandé

```
Maintenant  →  A  ELF loader
            →  B  Isolation mémoire (CR3 par processus)
            →  C  Syscalls complets
            →  D  KFS v2
            →  E  libkern (user library)
            →  F  IPC & Signaux
            →  G  Shell / UI
            →  H  Réseau  ← quand tout le reste est solide
```

**Le vrai déblocage est A + B + C** : une fois qu'on peut charger un ELF isolé avec des syscalls
corrects, l'OS devient un vrai OS. Tout le reste est de l'enrichissement.
