# 12 — Multiboot2 Memory Map

## Multiboot2 Info

GRUB passe au kernel un pointeur vers une structure **Multiboot2 info**.
Cette structure contient des **tags** décrivant le hardware détecté au boot.

### Passage du pointeur
```
GRUB → ebx = adresse Multiboot2 info
boot.asm → mov edi, ebx (sauvegarde avant setup_page_tables)
_start64 → call _start (rdi = 1er argument en System V ABI)
main.rs → fn _start(multiboot_addr: u64)
```

### Structure en mémoire
```
[total_size: u32][reserved: u32]     ← 8 octets header
[tag1][padding][tag2][padding]...    ← tags alignés sur 8 octets
[tag_end]                            ← type=0, size=8
```

---

## Tags

Chaque tag commence par un header de 8 octets :
```rust
struct Tag {
    tag_type: u32,   // identifiant
    size: u32,       // taille totale
}
```

| Type | Nom | Description |
|------|-----|------------|
| 0 | End | Fin de la liste |
| 6 | Memory Map | Carte des zones mémoire |
| 8 | Framebuffer | Info écran graphique |
| ... | ... | Autres tags (non utilisés) |

---

## Memory Map (tag type 6)

Le tag memory map liste les zones de mémoire physique :

```rust
struct MemoryEntry {
    base_addr: u64,   // adresse de début
    length: u64,      // taille en octets
    entry_type: u32,  // 1=libre, 2=réservé, 3=ACPI...
    _reserved: u32,
}
```

### Types de zones
| Type | Signification |
|------|--------------|
| 1 | RAM libre — utilisable par le kernel |
| 2 | Réservé — BIOS, MMIO, hardware |
| 3 | ACPI reclaimable |
| 4 | ACPI NVS (non volatile) |
| 5 | Bad RAM |

### Résultat typique (QEMU 128 Mo)
```
0x00000000 -    639 Ko - LIBRE    ← mémoire basse
0x0009fc00 -      1 Ko - RESERVE  ← BDA (BIOS Data Area)
0x000f0000 -     64 Ko - RESERVE  ← ROM BIOS
0x00100000 - 129920 Ko - LIBRE    ← RAM principale (~127 Mo)
0x07fe0000 -    128 Ko - RESERVE  ← ACPI
0xfffc0000 -    256 Ko - RESERVE  ← ROM haute
```

---

## Parsing

```rust
pub fn parse_memory_map(multiboot_addr: u64, callback: impl FnMut(&MemoryEntry)) {
    // 1. Sauter le header (8 octets)
    // 2. Boucle : lire chaque tag
    //    - Aligner sur 8 octets
    //    - Si type == 0 → fin
    //    - Si type == 6 → parcourir les entrées
    //    - Sinon → sauter (offset += size)
}
```

L'alignement `(offset + 7) & !7` arrondit au prochain multiple de 8.

---

## Concepts clés
| Concept | Explication |
|---------|------------|
| Multiboot2 | Standard de boot — GRUB passe des infos au kernel |
| `ebx` → `rdi` | GRUB met le pointeur dans ebx, on le passe comme argument à _start |
| System V ABI | Convention x86_64 : 1er argument = rdi |
| Identity map | Adresse virtuelle = adresse physique (configuré dans boot.asm) |
| `#[repr(C)]` | Layout mémoire C — nécessaire pour lire les structs du hardware |
| Tag type 6 | Memory map — liste des zones RAM du système |
