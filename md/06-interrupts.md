# 06 — Interruptions (IDT)

## C'est quoi une interruption ?
Quand le CPU rencontre un événement imprévu (erreur mémoire, division par zéro, touche clavier...),
il **interrompt** ce qu'il fait et saute vers une fonction prédéfinie — un **handler**.
La table qui liste tous ces handlers s'appelle l'**IDT** (Interrupt Descriptor Table).

Sans IDT → le CPU ne sait pas où aller → **triple fault** → reboot silencieux.

---

## Structure d'une entrée IDT (16 octets)
```
[ offset_low  : u16 ]  bits  0-15  de l'adresse du handler
[ selector    : u16 ]  sélecteur de segment dans la GDT (0x08)
[ ist         : u8  ]  Interrupt Stack Table (0 = stack courante)
[ flags       : u8  ]  0x8E = present + interrupt gate 64-bit
[ offset_mid  : u16 ]  bits 16-31  de l'adresse du handler
[ offset_high : u32 ]  bits 32-63  de l'adresse du handler
[ _reserved   : u32 ]  doit être 0
```

L'adresse est découpée en 3 morceaux — héritage du x86 16-bit, jamais changé.

**Flags 0x8E = `1000_1110`**
```
bit 7    = 1  → présent (entrée valide)
bits 5-6 = 00 → DPL 0 (ring 0, niveau kernel)
bits 0-3 = 1110 → interrupt gate 64-bit
```

---

## Numéros d'exceptions CPU (définis par Intel)
| N° | Nom | Cause |
|---|---|---|
| 0  | Divide Error | Division par zéro |
| 1  | Debug | Point d'arrêt debug |
| 2  | NMI | Erreur matérielle grave |
| 3  | Breakpoint | Instruction `int3` |
| 4  | Overflow | Dépassement arithmétique |
| 5  | Bound Range | Accès hors bornes |
| 6  | Invalid Opcode | Instruction inconnue |
| 8  | Double Fault | Deux exceptions simultanées — fatal |
| 13 | General Protection | Violation de protection mémoire |
| 14 | Page Fault | Accès mémoire invalide |

---

## Structure du code

### `IdtEntry::new(handler)`
```rust
let addr = handler as u64;  // adresse 64-bit de la fonction
IdtEntry {
    offset_low:  (addr & 0xFFFF) as u16,
    selector:    0x08,                            // segment GDT 64-bit
    flags:       0x8E,
    offset_mid:  ((addr >> 16) & 0xFFFF) as u16,
    offset_high: ((addr >> 32) & 0xFFFFFFFF) as u32,
    ...
}
```

### `Idt::load()`
```rust
let pointer = IdtPointer {
    limit: (256 * size_of::<IdtEntry>() - 1) as u16,  // 4095
    base:  self as *const _ as u64,
};
asm!("lidt [{}]", in(reg) &pointer);  // charge dans le registre IDTR du CPU
```

### Handler
```rust
pub unsafe extern "C" fn handler_page_fault() {
    println!("EXCEPTION: Page Fault !");
    loop {}  // bloque le kernel (pour l'instant)
}
```

### Init
```rust
pub fn init() {
    unsafe {
        let idt = core::ptr::addr_of_mut!(IDT);
        (*idt).set_handler(PAGE_FAULT, handler_page_fault);
        (*idt).load();
    }
}
```

---

## Concepts clés
| Concept | Explication |
|---|---|
| `#[repr(C)]` | Ordre des champs garanti — le CPU lit la struct directement |
| `#[repr(C, packed)]` | Pas de padding — `IdtPointer` doit faire exactement 10 octets |
| `unsafe extern "C" fn` | Le CPU appelle ces fonctions en ASM, pas via Rust |
| `lidt` | Instruction ASM qui charge l'IDT dans le registre IDTR |
| `$crate::` | Dans une macro, référence la crate où la macro est définie |
| Triple fault | Si pas d'IDT → reboot silencieux du CPU |
