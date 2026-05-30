# 32 — Syscalls (int 0x80)

## Objectif

Implémenter la frontière kernel/userspace via `int 0x80`.

## Convention d'appel

```
rax = numéro de syscall     → valeur de retour dans rax après iretq
rdi = arg1
rsi = arg2
rdx = arg3
```

## Syscalls implémentés

| Numéro | Nom | Signature | Description |
|--------|-----|-----------|-------------|
| 0 | `write` | `write(fd, buf, len) → len` | Écrit dans le terminal (fd=1/2) |
| 1 | `exit` | `exit(code)` | Termine la tâche courante |
| 2 | `getpid` | `getpid() → pid` | Index de la tâche courante |
| 3 | `yield` | `yield()` | Cède le CPU au prochain processus |
| 4 | `open` | `open(path, len) → fd` | Ouvre un fichier KFS (fd 3..7) |
| 5 | `close` | `close(fd)` | Libère un fd |
| 6 | `read` | `read(fd, buf, len) → n` | Lit depuis KFS à fd.pos |

`u64::MAX` = erreur (fichier introuvable, fd invalide, slots pleins).

## fd table

Chaque `Task` contient `fds: [FileDesc; 8]`.
- fd 0 = stdin (non implémenté)
- fd 1/2 = stdout/stderr → terminal
- fd 3..7 = fichiers KFS ouverts (`FileDesc { active, name[28], pos }`)

## Architecture

### syscall_stub (interrupt.rs)

Naked function, vecteur `0x80`, `DPL=3` (`flags=0xEE`). Réorganise les arguments :
```asm
mov rcx, rdx   // arg3
mov rdx, rsi   // arg2
mov rsi, rdi   // arg1
mov rdi, rax   // num
call syscall_inner
mov qword ptr [rsp + 112], rax   // préserve la valeur de retour dans le slot rax sauvegardé
```

**Fix important** : sans `mov [rsp+112], rax`, `pop rax` en fin de stub restaure le
numéro syscall original — la valeur de retour est perdue.

### syscall.rs

```rust
pub extern "C" fn syscall_handler(num, a1, a2, a3) -> u64 {
    unsafe { asm!("sti", ...); }   // ré-active les interruptions
    match num {
        SYS_WRITE  => sys_write(a1, a2, a3),
        SYS_EXIT   => sys_exit(a1),
        SYS_GETPID => sys_getpid(),
        SYS_YIELD  => sys_yield(),
        SYS_OPEN   => sys_open(a1, a2),
        SYS_CLOSE  => sys_close(a1),
        SYS_READ   => sys_read(a1, a2, a3),
        _          => u64::MAX,
    }
}
```

## Wrapper userspace (user/hello)

```rust
unsafe fn sys(num: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    let ret: u64;
    asm!("int 0x80",
        inlateout("rax") num => ret,
        in("rdi") a1, in("rsi") a2, in("rdx") a3,
        options(nostack));
    ret
}
```
