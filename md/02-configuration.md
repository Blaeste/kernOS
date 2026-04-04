# 02 — Configuration de compilation

## Fichier : `.cargo/config.toml`
```toml
[build]
target = "x86_64-unknown-none"  # cible bare-metal, pas d'OS

[target.x86_64-unknown-none]
rustflags = [
    "-C", "link-arg=-Tlinker.ld",      # utilise notre linker script
    "-C", "relocation-model=static"    # adresses fixes (pas de PIE)
]
```

## Fichier : `linker.ld`
```ld
ENTRY(_start32)       /* point d'entrée : notre asm 32-bit */

SECTIONS {
    . = 0x100000;     /* charge le kernel à 1 MB (zone réservée en dessous) */

    .multiboot : { KEEP(*(.multiboot)) }  /* header Multiboot2 — KEEP = ne pas supprimer */
    .text      : { *(.text*)   }          /* code exécutable */
    .rodata    : { *(.rodata*) }          /* données read-only */
    .data      : { *(.data*)   }          /* variables globales initialisées */
    .bss       : { *(.bss*)    }          /* variables globales non-initialisées */
}
```

## Concepts clés
| Concept | Explication |
|---|---|
| `x86_64-unknown-none` | Cible bare-metal : x86_64, pas d'OS |
| `relocation-model=static` | Adresse fixe — on sait qu'on sera à `0x100000` |
| `KEEP` | Force le linker à garder la section (sinon il l'efface) |
| `0x100000` | 1 MB — convention standard, zone en dessous réservée au BIOS |
