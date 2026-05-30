# 35 — Ring 3 : première tâche userspace

## Objectif
Exécuter du code en ring 3 (userspace) avec retour kernel via syscall.

## Milestone
**Premier `Hello ring 3!` affiché depuis du code exécuté en ring 3 !**

## Architecture

```
ring3 task                kernel
---------                --------
mov rax, 0               
mov rdi, 1               
int 0x80    ────────────► syscall_stub (IDT 0x80, DPL=3)
                          CPU lit TSS.rsp0 → pile kernel dédiée
                          syscall_inner → sys_write → println!
                         ◄──────────── iretq retour ring 3
mov rax, 1               
int 0x80    ────────────► sys_exit → task_exit()
                          scheduler → tâche 0 (shell)
```

## Composants

### Pages user (boot.asm)
Les huge pages doivent avoir le **bit 2 (U/S = User/Supervisor)** pour
être accessibles depuis ring 3. Sans ça : page fault immédiat.

```asm
or eax, 0b111       ; P4, P3 : present + writable + user
or eax, 0b10000111  ; P2 : present + writable + huge + user
```

### Pile d'interruption kernel dédiée (gdt.rs)
`TSS.rsp0` ne pointe plus vers `stack_top` (la pile kernel principale)
mais vers une pile dédiée `KERNEL_INT_STACK` (8 Ko statique).

Quand ring 3 fait `int 0x80`, le CPU :
1. Lit `TSS.rsp0` = sommet de `KERNEL_INT_STACK`
2. Switch la pile vers cette adresse fraîche
3. Pousse SS, RSP, RFLAGS, CS, RIP

Sans pile dédiée : le CPU écrase les frames kernel existants → GPF/corruption.

### spawn_ring3() (scheduler.rs)
Comme `spawn()` mais remplit le frame iretq avec les segments user :
- `ptr[15]` = RIP (entrée de la tâche)
- `ptr[16]` = USER_CODE (0x1B) — CS ring 3
- `ptr[17]` = 0x202 — RFLAGS (IF=1)
- `ptr[18]` = user_stack — RSP ring 3
- `ptr[19]` = USER_DATA (0x23) — SS ring 3

Quand le scheduler fait `iretq`, le CPU lit ces valeurs et switch en ring 3.

### userspace.rs (nouveau)
- `USER_STACK` : pile user 8 Ko statique
- `MSG` : message `[u8; 14]` en `.rodata` (pas une référence !)
- `user_task()` : appelle SYS_WRITE puis SYS_EXIT via `int 0x80`
- `spawn_user()` : appelle `spawn_ring3(entry, user_stack)`

### Syscall depuis ring 3
`int 0x80` fonctionne car l'IDT entry 0x80 a `flags=0xEE` (DPL=3).
Sans ça : GPF immédiat depuis ring 3.

## Erreurs rencontrées et fixes

| Erreur | Cause | Fix |
|--------|-------|-----|
| GPF error 0x20 | CS/SS inversés dans iretq frame | Échanger rdx/rcx dans push |
| Page Fault depuis ring 3 | Bit U/S absent dans page tables | `0b111` et `0b10000111` |
| GPF error 0x0, ptr=0x0 | `static &[u8]` = fat pointer, indirection nulle | `static [u8; 14]` + `sym MSG` |
| GPF dans sys_write | TSS.rsp0 pointe sur pile kernel active, corruption | Pile dédiée `KERNEL_INT_STACK` |

## Phase B — Isolation mémoire (CR3 par processus)

Chaque processus ELF obtient sa propre PML4 via `paging::new_cr3()` :
- `PDPT[0]` partagé = identity map kernel (0-1 Go, code/BSS/heap)
- `PDPT[1]` vide = zone user isolée (1-2 Go), remplie par `map_user_at`
- `Task.cr3` stocke l'adresse physique de la PML4 du processus

Le timer stub charge `NEXT_CR3` (écrit par `schedule()`) après `mov rsp, new_rsp` :
```asm
mov rax, [NEXT_CR3]
mov cr3, rax
```

**ELF relogé à 0x40000000** (1 Go, PDPT[1]) pour éviter les huge pages du kernel.
Pile user à 0x40200000.

## Phase C — ELF loader complet

`elf::load(bytes, cr3)` charge dans le CR3 du processus :
- `translate_in(cr3, vpage)` — vérifie si une page est déjà mappée (overlap PT_LOAD)
- `map_user_at(cr3, vpage, frame)` — mappe dans le CR3 cible sans changer le courant

`spawn_ring3(entry, user_stack, cr3)` écrit le frame iretq sur la **pile kernel** du slot
(pas sur user_stack, inaccessible depuis le kernel CR3 au moment du setup).

## Fichiers
- `src/elf.rs` — ELF64 loader (parse PT_LOAD, map_user_at, translate_in, BSS zero)
- `src/userspace.rs` — spawn_user / spawn_elf (new_cr3 + load + spawn_ring3)
- `src/task/scheduler.rs` — Task{cr3, fds}, NEXT_CR3, spawn_ring3(entry, stack, cr3)
- `src/mem/paging.rs` — new_cr3, map_user_at, translate_in, read_cr3 public
- `src/interrupt.rs` — irq_timer_stub charge CR3 depuis NEXT_CR3
- `src/gdt.rs` — KERNEL_INT_STACK, TSS.rsp0 dédié
- `src/main.rs` — TASKS[0].cr3 = NEXT_CR3 = boot_cr3 au démarrage
- `src/shell.rs` — commandes ring3 + run
- `user/hello/` — crate ELF userspace (syscall wrappers, test complet)
