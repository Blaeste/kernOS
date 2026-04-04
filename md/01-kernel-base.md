# 01 — Base du Kernel Rust

## Fichier : `src/main.rs`

### Attributs globaux
```rust
#![no_std]   // pas de bibliothèque standard (elle dépend d'un OS)
#![no_main]  // pas de fn main() — on gère notre propre point d'entrée
```

### Panic handler
```rust
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
```
- Obligatoire en `no_std`
- `-> !` = ne retourne jamais (type "never")

### Point d'entrée
```rust
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! { ... }
```
- `no_mangle` → garde le nom exact `_start` (le linker/bootloader le cherche)
- `extern "C"` → convention d'appel C
- `-> !` → le kernel ne retourne jamais

## Concepts clés
| Concept | Explication |
|---|---|
| `no_std` | Pas d'OS sous nous, donc pas de `std` |
| `core` | Mini-lib disponible sans OS (nombres, slices...) |
| `-> !` | Type "never" — fonction qui ne retourne pas |
| `unsafe` | Opérations dangereuses que Rust ne peut pas vérifier |
