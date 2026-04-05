# 13 — Frame Allocator (Pages Physiques)

## Concept

La RAM est découpée en **frames** de 4 Ko (4096 octets).
Le frame allocator gère quels frames sont libres ou occupés.

```
RAM physique:
[ frame 0 ][ frame 1 ][ frame 2 ][ frame 3 ] ...
  4 Ko       4 Ko       4 Ko       4 Ko
  occupé     occupé     libre      libre     ...
```

---

## Bitmap

Chaque bit = un frame. `0` = libre, `1` = occupé.

```
128 Mo de RAM / 4 Ko = 32 768 frames
32 768 bits / 8 = 4 096 octets de bitmap
```

```rust
static mut BITMAP: [u8; 4096] = [0xFF; 4096]; // tout occupé par défaut
```

Tout occupé par défaut = sécurité. On ne libère que les zones RAM détectées.

### Opérations sur les bits
```rust
// frame 13 → byte 1, bit 5
let byte = 13 / 8;   // = 1
let bit  = 13 % 8;   // = 5

// Mettre libre (bit à 0)
bitmap[byte] &= !(1 << bit);

// Mettre occupé (bit à 1)
bitmap[byte] |= 1 << bit;

// Tester si libre
bitmap[byte] & (1 << bit) == 0
```

---

## Initialisation

1. Parser la memory map Multiboot2
2. Pour chaque zone **type 1 (libre)** → marquer les frames comme libres
3. Protéger les **2 premiers Mo** (kernel, tables de pages, stack) → les re-marquer occupés

```rust
pub fn init(multiboot_addr: u64) {
    // Libère les frames des zones RAM
    parse_memory_map(..., |entry| {
        if entry.is_available() {
            set_free(start_frame..end_frame);
        }
    });
    // Protège le kernel (frames 0-511 = 0x0 à 0x200000)
    for f in 0..512 { set_used(f); }
}
```

---

## API

| Fonction | Description |
|----------|------------|
| `init(addr)` | Initialise le bitmap depuis la memory map |
| `alloc()` | Retourne l'adresse d'un frame libre (ou `None`) |
| `free(addr)` | Libère un frame |
| `free_count()` | Nombre de frames libres |

### Résultat typique (QEMU 128 Mo)
```
Frames libres: 32224 (128896 Ko)
= 128 Mo total - 2 Mo kernel = ~126 Mo disponibles
```

---

## Différence avec le bump allocator

| | Bump (`allocator.rs`) | Frame (`frame.rs`) |
|---|---|---|
| Granularité | N'importe quelle taille | 4 Ko fixe |
| Libération | ❌ Non | ✅ Oui |
| Usage | Heap Rust (`String`, `Vec`) | Pages physiques pour l'OS |
| Taille totale | 64 Ko fixe | Toute la RAM libre |
| Équivalent | `malloc` simplifié | Gestionnaire de pages |

---

## Concepts clés
| Concept | Explication |
|---------|------------|
| Frame | Page physique de 4 Ko |
| Bitmap | 1 bit par frame — compact et rapide |
| Identity map | Adresse physique = adresse virtuelle (pour l'instant) |
| Protection kernel | Frames 0-511 (2 Mo) toujours marqués occupés |
| `0xFF` par défaut | Sécurité — tout est occupé tant qu'on n'a pas détecté la RAM |
