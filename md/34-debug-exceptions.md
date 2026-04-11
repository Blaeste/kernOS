# 34 — Debug amélioré : exceptions et port 0xE9

## Objectif
Obtenir des informations utiles quand le kernel crash au lieu d'un freeze
silencieux.

## Problèmes résolus

### 1. Exceptions sans info
Les handlers d'exception (Page Fault, GPF) affichaient juste un message
puis `loop {}`. Aucune info sur l'adresse du crash ou la cause.

### 2. Deadlock dans les handlers
Si un crash arrive pendant un `println!` (PRINT_LOCK tenu), le handler
essaie aussi `println!` → deadlock (spin infini sur le lock).

### 3. Panic silencieux
Le panic handler faisait `loop {}` sans aucune sortie. Impossible de
savoir si c'est une exception ou un panic Rust.

### 4. Caractères non-ASCII → panic
`draw_char()` accédait à `FONT[c as usize]` avec une font de 128 entrées.
Un caractère UTF-8 > 127 (comme `é` = 0xC3 0xA9) causait un index out of
bounds → panic.

## Solutions

### Port debug QEMU (0xE9)
`src/klib/debug.rs` — écrit directement sur le port 0xE9 :
- `dbg_str(s)` : affiche une string
- `dbg_ln(s)` : string + newline  
- `dbg_hex(val)` : affiche un u64 en hexadécimal

Sortie visible dans le terminal avec `-debugcon stdio` dans QEMU.
Aucune dépendance au framebuffer ni au PRINT_LOCK.

### Stubs naked pour GPF et Page Fault
Les exceptions avec error code (GPF=13, PF=14) poussent un code d'erreur
sur la pile. Il faut un stub naked qui le pop avant d'appeler le handler :

```
page_fault_stub:
    pop rdi          ; error code (poussé par le CPU)
    mov rsi, [rsp]   ; RIP (frame iretq)
    mov rdx, cr2     ; adresse fautive
    call page_fault_inner
```

`page_fault_inner` affiche : RIP, CR2, error code décodé (present/write/user).
`gpf_inner` affiche : RIP, error code.

### force_unlock()
`SpinLock::force_unlock()` force la libération du lock sans vérifier
qu'il est tenu. Utilisé par les handlers d'exception avant de `println!`.

### SpinLock avec cli/sti
`lock()` fait `cli` (désactive interruptions) avant d'acquérir.
`unlock()` fait `sti` après libération. Empêche les interruptions
d'interrompre une section critique.

### Panic handler amélioré
Affiche le fichier et la ligne du panic via le port debug :
```
!!! PANIC !!!
  at src/io/framebuffer.rs:0xB4
```

### Clamp draw_char
Si `c >= 128`, affiche `?` au lieu de crash :
```rust
let idx = if c < 128 { c as usize } else { b'?' as usize };
```

### Clamp swap_buffers
Dirty rect clampé aux dimensions de l'écran pour éviter les écritures
hors framebuffer.

## Fichiers modifiés
- `src/klib/debug.rs` (nouveau) — sortie port 0xE9
- `src/klib/mod.rs` — `pub mod debug`
- `src/klib/sync.rs` — `force_unlock()`, cli/sti dans lock/unlock
- `src/interrupt.rs` — page_fault_stub/inner, gpf_stub/inner avec debug
- `src/main.rs` — panic handler amélioré
- `src/io/framebuffer.rs` — clamp draw_char + clamp swap_buffers
- `src/shell.rs` — supprimé accent dans help (`état` → `etat`)
- `run.ps1` — ajouté `-debugcon stdio`
