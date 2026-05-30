# KernOS — Roadmap

## État actuel — Phases A→D complètes

Le kernel est un vrai OS userspace : ELF isolé en ring 3, syscalls complets, filesystem multi-blocs.

### Fait ✅

| Module | Détail |
|--------|--------|
| Boot ASM + Long mode 64-bit | Multiboot2, passage 32→64 |
| VGA + print!/println! | Affichage texte kernel |
| IDT + PIC + IRQ | Interruptions CPU complètes |
| Clavier PS/2 (AZERTY/QWERTY) + Souris | Input complet |
| Shell interactif | Parsing commandes |
| Free List allocator 256 Ko | alloc + free + fusion |
| Timer PIT 100 Hz | Base temps kernel |
| Frame allocator bitmap | Gestion physique de la RAM |
| Pagination 4 niveaux | map / unmap / translate / map_user_at |
| Multitâche préemptif | Round-robin, context switch, spawn |
| Framebuffer 1080p | Double buffering, dirty rect |
| Window manager | Drag & drop, z-order, focus |
| Taskbar + horloge | UI desktop basique |
| Driver ATA | Lecture/écriture disque |
| KFS v2 | 64 fichiers, FAT chaînée, 64 Ko max, read_at |
| SpinLock | Protection accès concurrent |
| GDT + TSS complète | Segments kernel + user |
| Debug amélioré | Port 0xE9, GPF/PF avec RIP/CR2 |
| ELF loader (Phase A) | parse PT_LOAD, map_user_at, copy + zero BSS |
| CR3 par processus (Phase B) | PDPT[1] isolé, switch dans irq_timer_stub |
| 7 syscalls (Phase C) | write/exit/getpid/yield/open/read/close + fd table |
| KFS v2 (Phase D) | FAT chaînée, 64 entrées, read_at positionnée |

---

## Phase A — ELF Loader ✅
> **Priorité absolue.** Sans ça, tout code userspace doit être linké dans le kernel.

- [x] Parser ELF64 — lire les headers `Elf64_Phdr`, identifier les segments `PT_LOAD`
- [x] Allouer des frames user — pour chaque segment : frame allocator → pages U/S mappées
- [x] Copier les segments — charger le code/data à leur adresse virtuelle cible
- [x] Lancer en ring 3 — `include_bytes!` + `elf::load()` + `spawn_ring3()` depuis le shell → **"Hello from ELF!" ✅**
- [ ] Stocker les ELF dans KFS — un fichier KFS = un binaire exécutable (Phase D)

**Milestone :** `run monprog` dans le shell charge et exécute un ELF depuis le disque en ring 3.

---

## Phase B — Isolation mémoire ✅
> Actuellement tous les processus partagent le même CR3 — catastrophique pour la stabilité.

- [x] CR3 par processus — `Task.cr3` + `paging::new_cr3()` — PDPT[0] partagé (kernel 0-1 Go), PDPT[1] isolé (user 1-2 Go)
- [x] Switch CR3 au context switch — `mov cr3, [{nc}]` dans `irq_timer_stub`, `NEXT_CR3` écrit par `schedule()`
- [x] ELF user relogé à 0x40000000 (1 Go) — hors de la zone kernel, isolation réelle
- [ ] Libération à task_exit — free pages user + PML4 (Phase D, nécessite walk PT)
- [x] Tester l'isolation — `run` charge l'ELF dans son CR3 propre, kernel CR3 intact ✅

**Milestone :** un process crashé ne corrompt pas les autres.

---

## Phase C — Syscalls complets ✅
> Aujourd'hui : 2 syscalls. Il en faut ~10 pour qu'un programme soit utile.

- [x] `getpid()` — retourne l'index de la tâche courante
- [x] `yield()` — cède le CPU, reprise correcte après reschedule
- [x] `open(path)` → fd — ouvre un fichier KFS, réserve slot fd 3..7
- [x] `close(fd)` — libère le slot fd
- [x] `read(fd, buf, len)` — lit depuis KFS avec position courante (fd.pos)
- [x] Fix syscall_stub — `mov [rsp+112], rax` préserve la valeur de retour
- [x] fd table dans Task — `[FileDesc; 8]` (name + pos par fichier ouvert)
- [x] user/hello teste tous les syscalls — write/pid/yield/open/read/close ✅
- [ ] `read(0, buf, len)` stdin — bloquant, nécessite Phase F (IPC)
- [ ] `exec(path)` — remplace le processus par un ELF (Phase D/E)
- [ ] `sleep(ms)` — attente temporisée

**Milestone :** un programme userspace peut lire/écrire des fichiers sans accès kernel direct.

---

## Phase D — KFS v2 ✅
> KFS v1 est trop limité pour accueillir des programmes (8 fichiers × 512 octets).

- [x] Taille variable — FAT chaînée (128 blocs × 512 o = 64 Ko max par fichier)
- [x] Plus de fichiers — 64 entrées × 32 octets (4 secteurs de table)
- [x] `read_at(name, offset, buf)` — lecture positionnée pour syscall read(fd.pos)
- [x] Auto-reformat au boot si magic/version invalide (migration v1→v2 transparente)
- [x] Testé : kfsWrite / kfsLs / kfsRead / run open+read depuis ELF ✅
- [ ] Sous-dossiers — entrées de type `DIR` (Phase ultérieure)
- [ ] Cache secteurs — éviter de relire ATA à chaque accès (Phase ultérieure)

**Milestone :** `kfsLs /bin` affiche les exécutables, fichiers > 512 octets lisibles.

---

## Phase E — User library (libkern) ✅
> Sans ça, chaque programme réimplémente les appels système.

- [x] `libkern` — crate `no_std` à `user/libkern/`, linkée via `path = "../libkern"`
- [x] Wrappers syscalls — `write`, `exit`, `getpid`, `yield_cpu`, `open`, `close`, `read` + `print_u64`
- [x] Bump allocator 16 Ko — `BumpAllocator` + `#[global_allocator]` dans `allocator.rs`
- [x] `Vec<u8>` en userspace — `extern crate alloc; use libkern::*;` → Vec depuis heap user ✅
- [x] `user/hello` simplifié — supprime les wrappers inline, utilise `use libkern::*`

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
✅ FAIT    →  A  ELF loader
✅ FAIT    →  B  Isolation mémoire (CR3 par processus)
✅ FAIT    →  C  Syscalls complets
✅ FAIT    →  D  KFS v2
Maintenant →  E  libkern (user library)
            →  F  IPC & Signaux
            →  G  Shell / UI
            →  H  Réseau  ← quand tout le reste est solide
```

**Le vrai déblocage A + B + C + D est accompli** : ELF isolé, syscalls complets, filesystem
multi-blocs. La prochaine étape (libkern) rend le développement userspace confortable.
