# 33 — GDT + TSS pour Ring 3

## Objectif
Remplacer la GDT minimale de boot.asm par une GDT complète avec TSS,
nécessaire pour exécuter du code en ring 3 (userspace).

## Concepts

### GDT (Global Descriptor Table)
Table de segments lue par le CPU. Chaque entrée décrit un segment mémoire :
- **Sélecteur** = index × 8 + RPL (Ring Privilege Level)
- Le CPU utilise CS (Code Segment) et SS (Stack Segment) pour vérifier les privilèges

### Structure de notre GDT (7 entrées)
| Index | Sélecteur | Description |
|-------|-----------|-------------|
| 0     | 0x00      | NULL (obligatoire) |
| 1     | 0x08      | Code kernel (DPL=0) |
| 2     | 0x10      | Data kernel (DPL=0) |
| 3     | 0x1B      | Code user (DPL=3, RPL=3) |
| 4     | 0x23      | Data user (DPL=3, RPL=3) |
| 5-6   | 0x28      | TSS (16 octets, 2 entrées) |

### Bits importants des descripteurs de segment
- Bit 41 : Writable (data) / Readable (code)
- Bit 43 : Executable (1 = code, 0 = data)
- Bit 44 : Descriptor type (1 = code/data)
- Bits 45-46 : DPL (0 = kernel, 3 = user)
- Bit 47 : Present
- Bit 53 : Long mode (64-bit, code seulement)

### TSS (Task State Segment)
Structure de 104 octets. Le CPU y lit RSP0 = l'adresse de la pile kernel.
Quand une interruption arrive depuis ring 3, le CPU :
1. Lit TSS.RSP0
2. Switch la pile vers cette adresse
3. Pousse SS, RSP, RFLAGS, CS, RIP sur la pile kernel
4. Saute au handler

Sans TSS → le CPU ne sait pas quelle pile utiliser → triple fault.

### Descripteur TSS dans la GDT
Le TSS en 64-bit occupe **16 octets** (2 entrées de 8) :
- tss_low : taille + adresse bits 0-23 + type 0x89 + adresse bits 24-31
- tss_high : adresse bits 32-63

## Fichiers

### `src/gdt.rs` (nouveau)
- `Tss` : struct repr(C, packed), 104 octets
- `Gdt` : struct avec 5 segments + TSS (2 entrées)
- Constantes : KERNEL_CODE=0x08, KERNEL_DATA=0x10, USER_CODE=0x1B, USER_DATA=0x23, TSS_SEL=0x28
- `init(kernel_stack_top)` :
  1. Configure TSS.rsp0
  2. Remplit les segments (kernel + user + TSS descriptor)
  3. `lgdt` charge la nouvelle GDT
  4. `ltr` charge le TSS
  5. Recharge DS/ES/FS/GS/SS avec KERNEL_DATA
  6. Far return (`retfq`) pour recharger CS avec KERNEL_CODE

### `src/main.rs`
- `mod gdt;` ajouté
- `extern "C" { static stack_top: u8; }` pour récupérer l'adresse de la pile depuis boot.asm
- `gdt::init(kstack)` appelé au boot, avant les interruptions

### `src/boot.asm`
- `global stack_top` ajouté pour exporter le symbole

### `src/interrupt.rs`
- `selector: 0x08` remplacé par `crate::gdt::KERNEL_CODE`

### `src/task/scheduler.rs`
- `0x08` dans spawn() remplacé par `crate::gdt::KERNEL_CODE as u64`

## Instructions asm clés
- `lgdt [ptr]` — charge la GDT (limite + base)
- `ltr reg` — charge le sélecteur TSS dans le Task Register
- `mov ds, reg` — recharge un segment de données
- `push cs; push addr; retfq` — far return pour recharger CS
