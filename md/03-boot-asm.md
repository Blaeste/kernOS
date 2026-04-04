# 03 — Bootloader ASM (`src/boot.asm`)

## Pourquoi de l'assembleur ?
GRUB démarre le CPU en **mode 32-bit**. Notre kernel Rust est 64-bit.
Il faut un pont en ASM pour passer du 32-bit au 64-bit.

## Étapes du démarrage

### 1. Header Multiboot2
```asm
section .multiboot
align 8
header_start:
    dd 0xE85250D6                    ; magic Multiboot2
    dd 0                             ; architecture i386
    dd header_end - header_start     ; taille
    dd 0x100000000 - (...)           ; checksum
    dw 0  dw 0  dd 8                 ; end tag
header_end:
```
GRUB scanne les 8 premiers Ko du binaire pour trouver ce magic.

### 2. Passage en mode 64-bit (5 étapes obligatoires)
```
1. setup_page_tables  → prépare la pagination (identity map 1 Go)
2. cr4 bit 5 (PAE)    → active Physical Address Extension
3. cr3 = p4_table     → pointe vers la table de pages racine
4. EFER MSR bit 8     → active Long Mode Enable
5. cr0 bit 31 (PG)    → active la pagination → Long Mode activé !
```

### 3. GDT + far jump
```asm
lgdt [gdt64.pointer]       ; charge la Global Descriptor Table
jmp gdt64.code:_start64    ; far jump → active vraiment le 64-bit
```

### 4. `_start64` — on est en 64-bit
```asm
bits 64
_start64:
    mov ax, 0        ; segments à 0 (inutiles en 64-bit)
    ...
    call _start      ; appelle le kernel Rust !
    hlt
```

## Concepts clés
| Concept | Explication |
|---|---|
| `bits 32` / `bits 64` | Indique à NASM le mode du code qui suit |
| `global` | Rend un symbole visible pour le linker (`pub` en Rust) |
| `extern` | Déclare un symbole défini dans un autre fichier |
| `resb N` | Réserve N octets non-initialisés |
| `dd` / `dw` / `dq` | Define 32 / 16 / 64 bits |
| Identity map | Adresse virtuelle = adresse physique (plus simple pour démarrer) |
| GDT | Table obligatoire en 64-bit, décrit les segments mémoire |
