# 29 — Allocateur Free List

## Objectif

Remplacer le BumpAllocator (pas de free) par un Free List allocator avec alloc + free + fusion.

## Architecture

### Heap

Zone mémoire statique de 256 Ko, alignée à 16 octets pour garantir l'alignement de `FreeBlock` :

```rust
#[repr(align(16))]
struct Heap([u8; 256 * 1024]);
static mut HEAP: Heap = Heap([0; 256 * 1024]);
```

> **Avant :** `static mut HEAP: [u8; N]` sans alignement explicite → alignement 1 octet → `FreeBlock` potentiellement misaligné.

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
3. Si le reste est assez grand → découpe en 2 blocs, remplace le bloc courant par le résidu dans la liste
4. Sinon → **retire le bloc entier de la free list** avant de le retourner
5. Retourne `ptr + MIN_BLOCK` (saute le header)

> **Bug corrigé :** l'ancienne version oubliait de retirer le bloc de la free list dans le cas 4, causant une double-allocation et une corruption silencieuse du heap.

### dealloc

1. Recule de `MIN_BLOCK` pour retrouver le header
2. **Insère le bloc en ordre croissant d'adresse** (parcourt la liste jusqu'à trouver le bon emplacement)
3. Fusionne immédiatement avec le bloc suivant si adjacent
4. Fusionne immédiatement avec le bloc précédent si adjacent

> **Bug corrigé :** l'ancienne version insérait en tête de liste (ordre aléatoire) puis appelait `sort_free_list()` — un tri bulle sur linked list dont l'implémentation ne reconstruisait pas le lien `prev`, corrompant la liste à chaque swap. Supprimé et remplacé par l'insertion triée en O(n).

### Fusion (coalescence)

Gérée directement dans `dealloc` en O(1) après insertion triée — aucune passe séparée nécessaire.

```
avant free :  [bloc A] → [bloc C]
après free B : [bloc A] → [bloc B] → [bloc C]
si A+B adjacents : [bloc AB] → [bloc C]
si AB+C adjacents : [bloc ABC]
```

## Commande shell

```
> heapInfo
Heap: 256 Ko total, 256 Ko libre, 1 blocks libres
```

## Changements

- `src/mem/allocator.rs` — réécrit : FreeListAllocator remplace BumpAllocator
- `src/mem/allocator.rs` — v2 : correction double-allocation + suppression sort corrompu + HEAP aligné 16
- `src/main.rs` — `#[global_allocator]` → FreeListAllocator, appel `mem::allocator::init()`
- `src/shell.rs` — commande `heapInfo`
- `run.sh` — compilation `user/hello` avant le kernel (requis par `include_bytes!`)
