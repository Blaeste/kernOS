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

## Fichiers
- `src/userspace.rs` (nouveau) — tâche ring 3 + spawn_user
- `src/task/scheduler.rs` — spawn_ring3()
- `src/gdt.rs` — KERNEL_INT_STACK, TSS.rsp0 dédié
- `src/boot.asm` — bit U/S sur toutes les pages
- `src/main.rs` — mod userspace
- `src/shell.rs` — commande ring3
