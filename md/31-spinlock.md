# 31 — Spinlock (klib/sync)

## Objectif

Protéger les ressources partagées entre tâches maintenant que le multitâche préemptif est actif.

## SpinLock

Basé sur `AtomicBool` — 100% safe Rust, no_std compatible :

```rust
pub struct SpinLock {
    locked: AtomicBool,
}
```

### API

```rust
SpinLock::new()          // constante, utilisable en static
lock()                   // spin jusqu'à obtenir le verrou
unlock()                 // libère le verrou
with(|| { ... })         // lock + exécute + unlock automatique
```

### Ordres mémoire

- `lock()` : `compare_exchange(..., Ordering::Acquire, ...)` — garantit que les lectures/écritures après le lock voient les effets avant le unlock précédent
- `unlock()` : `store(false, Ordering::Release)` — garantit que toutes les écritures avant le unlock sont visibles

## Protection de println!

`PRINT_LOCK` global dans `framebuffer.rs` :

```rust
pub static PRINT_LOCK: SpinLock = SpinLock::new();
```

La macro `print!` l'utilise :

```rust
crate::io::framebuffer::PRINT_LOCK.with(|| {
    unsafe { ... }
});
```

→ Deux tâches ne peuvent plus écrire simultanément dans le terminal.

## Utilisation future

```rust
use crate::klib::sync::SpinLock;

static MY_LOCK: SpinLock = SpinLock::new();

MY_LOCK.with(|| {
    // accès exclusif à une ressource partagée
});
```
