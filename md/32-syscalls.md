# 32 — Syscalls (int 0x80)

## Objectif

Implémenter la frontière kernel/userspace via `int 0x80`.

## Convention d'appel

```
rax = numéro de syscall
rdi = arg1
rsi = arg2
rdx = arg3
rax = valeur de retour
```

## Syscalls implémentés

| Numéro | Nom | Description |
|--------|-----|-------------|
| 0 | `write` | Écrit dans le terminal (fd=1) |
| 1 | `exit` | Termine la tâche courante |

## Architecture

### syscall_stub (interrupt.rs)

Naked function enregistrée sur le vecteur `0x80` avec `DPL=3` (`flags=0xEE`) — accessible depuis ring 3.

Réorganise les arguments avant l'appel (convention SysV AMD64) :
```asm
mov rcx, rdx   // arg3
mov rdx, rsi   // arg2
mov rsi, rdi   // arg1
mov rdi, rax   // num
call syscall_inner
```

### set_handler_user (interrupt.rs)

Nouvelle méthode sur `Idt` — comme `set_handler` mais avec `flags=0xEE` (DPL=3).
Les autres handlers restent à `0x8E` (DPL=0, kernel seulement).

### syscall.rs

Table des syscalls + dispatch :
```rust
pub extern "C" fn syscall_handler(num, a1, a2, a3) -> u64 {
    match num {
        SYS_WRITE => sys_write(a1, a2, a3),
        SYS_EXIT  => sys_exit(a1),
        _         => u64::MAX,
    }
}
```

`sys_write` utilise `klib::string::to_str_or` pour valider l'UTF-8.

## Test depuis le shell

```rust
core::arch::asm!(
    "int 0x80",
    in("rax") 0u64,              // SYS_WRITE
    in("rdi") 1u64,              // fd=1 stdout
    in("rsi") msg.as_ptr() as u64,
    in("rdx") msg.len() as u64,
);
```

```
> syscalltest
[syscall write] ca marche !
```
