# 29 — Allocateur Free List

## Objectif

Remplacer le BumpAllocator (pas de free) par un Free List allocator avec alloc + free + fusion.

## Architecture

### Heap

Zone mémoire statique de 256 Ko (avant : 64 Ko) :

```rust
static mut HEAP: [u8; 256 * 1024] = [0; 256 * 1024];
```

### FreeBlock (header)

Chaque bloc libre contient un en-tête de 16 octets (2 × usize sur x86_64) :

```rust
#[repr(C)]
struct FreeBlock {
    size: usize,          // taille totale (header inclus)
    next: *mut FreeBlock,  // prochain bloc libre
}
```

### Liste chaînée

`FREE_HEAD` pointe vers le premier bloc libre. Au `init()`, un seul bloc couvre tout le heap.

### alloc (first-fit)

1. Parcourt la free list
2. Trouve le premier bloc >= taille demandée
3. Si le reste est assez grand → découpe en 2 blocs
4. Sinon → prend tout le bloc
5. Retourne `ptr + MIN_BLOCK` (saute le header)

### dealloc

1. Recule de `MIN_BLOCK` pour retrouver le header
2. Insère le bloc en tête de liste
3. Appelle `merge_free_blocks()` → tri par adresse + fusion des blocs adjacents

### Fusion (coalescence)

1. `sort_free_list()` — tri bulle par adresse croissante
2. `merge_free_blocks()` — si `bloc.addr + bloc.size == next.addr` → fusionne

## Commande shell

```
> heapInfo
Heap: 256 Ko total, 256 Ko libre, 1 blocks libres
```

## Changements

- `src/mem/allocator.rs` — réécrit : FreeListAllocator remplace BumpAllocator
- `src/main.rs` — `#[global_allocator]` → FreeListAllocator, appel `mem::allocator::init()`
- `src/shell.rs` — commande `heapInfo`
