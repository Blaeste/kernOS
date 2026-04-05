# 10 — Allocateur Mémoire (Heap)

## Pourquoi un allocateur ?

Sans allocateur, on ne peut utiliser que des tableaux de taille fixe (`[u8; 78]`).
Avec un allocateur, on débloque la crate `alloc` de Rust :
- `String` — chaînes de taille variable
- `Vec<T>` — tableaux dynamiques
- `Box<T>` — allocation sur le heap

---

## Bump Allocator

Le plus simple des allocateurs : un pointeur qui avance à chaque allocation.

```
Heap: [████████████████████░░░░░░░░░░░░]
       ^                  ^             ^
       début              offset        fin (64 Ko)
```

- Chaque `alloc()` avance `offset` de `layout.size()` octets
- `dealloc()` ne fait rien — la mémoire n'est jamais libérée
- Simple mais suffisant pour un kernel qui ne libère pas beaucoup

### Alignement
Le CPU exige que certains types soient alignés (u32 sur 4 octets, u64 sur 8...).
L'allocateur arrondit l'offset au prochain multiple de `layout.align()` :

```rust
let aligned = (offset + align - 1) & !(align - 1);
```

`& !(align - 1)` masque les bits bas → arrondi au multiple inférieur.

---

## Code (`allocator.rs`)

```rust
const HEAP_SIZE: usize = 64 * 1024;          // 64 Ko
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];  // dans .bss
static mut OFFSET: usize = 0;                // pointeur courant

pub struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Aligner, avancer, retourner le pointeur
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Ne fait rien (bump)
    }
}
```

---

## Intégration dans `main.rs`

```rust
extern crate alloc;         // active la crate alloc (String, Vec, Box)
mod allocator;

#[global_allocator]         // dit à Rust : utilise notre allocateur
static ALLOCATOR: allocator::BumpAllocator = allocator::BumpAllocator;
```

---

## Piège Rust 2024 : `static mut` et références

```rust
// ❌ Interdit en edition 2024 — crée une référence vers static mut
unsafe { HEAP.as_mut_ptr().add(aligned) }

// ✅ addr_of_mut! donne un pointeur brut sans référence
unsafe { core::ptr::addr_of_mut!(HEAP).cast::<u8>().add(aligned) }
```

---

## Traits importants
| Trait/Type | Rôle |
|-----------|------|
| `GlobalAlloc` | Trait que doit implémenter tout allocateur global |
| `Layout` | Décrit une allocation : taille + alignement |
| `#[global_allocator]` | Attribut qui désigne l'allocateur du programme |
| `extern crate alloc` | Active la crate `alloc` (types heap sans `std`) |

---

## Limitations actuelles
- Pas de libération mémoire (`dealloc` ne fait rien)
- Heap fixe de 64 Ko (dans le .bss)
- Pas thread-safe (pas de mutex)
- Futur : free-list allocator ou buddy allocator pour pouvoir libérer
