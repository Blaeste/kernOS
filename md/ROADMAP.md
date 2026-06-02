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
- [x] Stocker + exécuter les ELF depuis KFS — `install <nom>` écrit l'ELF sur disque, `run <nom>` le relit et le charge ✅
- [x] ELF strippé (`strip = true`) — 980 Ko → 18 Ko, rentre dans KFS (64 Ko max)
- [x] Fix `elf::load` multi-pages — segments non alignés sur >1 page (le strip compacte → `done` mal compté → .rodata corrompue → GPF)

**Milestone :** ✅ `run <nom>` charge et exécute un ELF depuis le disque KFS en ring 3.

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

## Phase F — IPC & Signaux ✅
> Pour que les programmes communiquent.

- [x] Pipe — buffer circulaire 256 o dans le kernel, `src/ipc.rs`, jusqu'à 4 pipes
- [x] fd typés — `FileDesc.kind` (KFS / pipe_read / pipe_write), read/write/close routés
- [x] `SYS_PIPE` (9) → `(fd_r << 32) | fd_w` ; testé write→read dans user/hello ✅
- [x] `wait(pid)` (8) — `Task.waiting_for`, scheduler skip les tâches en attente
- [x] `kill(pid, 9)` (7) — SIGKILL marque la tâche inactive + réveille les waiters
- [x] **Fix critique frame allocator** — protège tout le kernel via `kernel_end`
      (le BSS fait ~8 Mo à cause de BACK_BUF ; avant, `alloc()` rendait des frames
      dans le BSS → triple fault au 2ème `run`)
- [ ] `SIGSTOP` / `SIGCONT` — pause/reprise (Phase ultérieure)
- [ ] `fork()` — clone CR3 + task, copy-on-write (Phase ultérieure)

---

## Phase G — Shell et UI
> Rendre l'OS utilisable au quotidien.

- [ ] Shell amélioré — pipes (`|`), redirections (`>`, `<`), variables `$VAR` (bloqué : besoin fork + stdin)
- [x] Terminal ANSI — `\e[…m` parsé dans `FbWriter` : 16 couleurs fg + bold, reset. Commande `colors` ✅
- [x] Fenêtres redimensionnables — drag du coin bas-droit (poignée visible), clamp mini/écran, re-flow du writer Terminal (max_cols/rows clampés au text_buf, curseur réaligné) ✅
- [x] Fixes UI au passage — clic en front montant (plus de spam launcher → crash), curseur sans fantôme en resize
- [x] Éditeur de texte intégré — `edit <fichier>` (fenêtre dédiée), insert/backspace/newline, curseur, **Ctrl+S** sauve sur KFS, **Escape/Ctrl+Q** ferme. Routage clavier vers la fenêtre focus (Ctrl tracké) ✅
- [x] Launcher — bouton `> hello` dans la taskbar → clic lance l'ELF en ring 3 (+ fix flicker : les clics taskbar ne forcent plus de redraw plein écran) ✅

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
✅ FAIT    →  A  ELF loader (+ exec depuis KFS)
✅ FAIT    →  B  Isolation mémoire (CR3 par processus)
✅ FAIT    →  C  Syscalls complets
✅ FAIT    →  D  KFS v2
✅ FAIT    →  E  libkern (user library)
✅ FAIT    →  F  IPC & Signaux
Maintenant →  G  Shell / UI
            →  H  Réseau  ← quand tout le reste est solide
```

**Le vrai déblocage A + B + C + D est accompli** : ELF isolé, syscalls complets, filesystem
multi-blocs. La prochaine étape (libkern) rend le développement userspace confortable.
