# Étape 14 — Pagination x86_64

## Concept

La pagination crée une couche de traduction entre adresses **virtuelles** (ce que voit le code) et adresses **physiques** (la vraie RAM). Chaque programme peut avoir son propre espace d'adressage sans interférer avec les autres.

## Architecture 4 niveaux

```
Adresse virtuelle (48 bits) :
┌──────────┬──────────┬──────────┬──────────┬──────────────┐
│  PML4    │  PDPT    │   PD     │   PT     │   Offset     │
│ 9 bits   │ 9 bits   │ 9 bits   │ 9 bits   │  12 bits     │
│ [47:39]  │ [38:30]  │ [29:21]  │ [20:12]  │  [11:0]      │
└──────────┴──────────┴──────────┴──────────┴──────────────┘

CR3 → PML4[i4] → PDPT[i3] → PD[i2] → PT[i1] → page + offset
```

- Chaque table = 512 entrées × 8 octets = 4096 octets = 1 frame
- 9 bits = index 0 à 511

## Format d'une entrée (u64)

| Bits | Rôle |
|---|---|
| 0 | PRESENT — la page existe |
| 1 | WRITABLE — écriture autorisée |
| 7 | HUGE PAGE — 2 Mo (PD) ou 1 Go (PDPT) |
| 12..51 | Adresse physique (alignée 4 Ko) |

## Fichier : `src/mem/paging.rs`

### Fonctions

| Fonction | Rôle |
|---|---|
| `map(virt, phys)` | Mappe une page 4 Ko, crée les tables intermédiaires |
| `unmap(virt)` | Supprime un mapping (met l'entrée PT à 0) |
| `translate(virt)` | Retourne l'adresse physique ou None |

### Helpers internes

| Fonction | Rôle |
|---|---|
| `read_cr3()` | Lit le registre CR3 (adresse de la PML4) |
| `invlpg(addr)` | Invalide le cache TLB pour une adresse |
| `table_at(phys)` | Convertit une adresse physique en `&mut [u64; 512]` |
| `zero_frame(addr)` | Met un frame à zéro (nouvelles tables) |

## Identity map (boot.asm)

Le bootloader a déjà mappé le 1er Go en identity map (virt == phys) avec des huge pages 2 Mo. C'est crucial car `table_at()` accède aux tables par leur adresse physique directement comme adresse virtuelle.

`map()` refuse de toucher aux entrées PD avec le bit HUGE_PAGE pour ne pas casser le mapping du boot.

## Commande shell `pgtest`

Test complet du cycle : alloc frame → map → écriture → lecture → translate → unmap → free.

```
> pgtest
Frame alloue: 0x200000
Mappe: 0x40000000 -> 0x200000
Ecriture: *0x40000000 = 0xCAFE_BABE
Lecture:  *0x40000000 = 0xcafebabe
Translate: 0x40000000 -> 0x200000 OK
Unmap + free OK
```

## TLB — le piège classique

Le CPU cache les traductions virt→phys dans le TLB. Après avoir modifié une table de pages, il faut appeler `invlpg` sinon le CPU utilise l'ancien mapping en cache → bugs mystérieux.
