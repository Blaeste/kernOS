# 09 — Mini-Shell

## Architecture

Le shell intercepte les caractères du clavier et les accumule dans un buffer.
Quand l'utilisateur appuie sur Enter, la commande est comparée et exécutée.

```
Touche clavier
    → interrupt.rs (IRQ1 → scancode)
    → keyboard.rs  (scancode → ASCII)
    → shell.rs     (ASCII → buffer → commande)
```

---

## Buffer de commande

```rust
const MAX_CMD: usize = 78;  // 80 colonnes - 2 pour "> "
static mut BUFFER: [u8; MAX_CMD] = [0; MAX_CMD];
static mut LEN: usize = 0;
```

- Tableau fixe de 78 octets (pas d'allocateur → pas de `String`)
- `LEN` = nombre de caractères tapés
- Reset à 0 après chaque Enter

---

## Dispatch des caractères (`on_char`)

| Caractère | Action |
|-----------|--------|
| `\n` (Enter) | Exécute la commande, remet `LEN = 0`, réaffiche le prompt |
| `8` (Backspace) | Recule `LEN`, efface le caractère à l'écran via `backspace()` |
| autre | Ajoute au buffer si pas plein, affiche le caractère |

---

## Commandes

| Commande | Action |
|----------|--------|
| `help` | Affiche la liste des commandes |
| `clear` | Efface l'écran via `Writer::clear()` |
| `reboot` | Redémarre la machine via le contrôleur clavier PS/2 |
| *(vide)* | Ne fait rien |
| *(autre)* | Affiche "commande inconnue: ..." |

### Reboot
```rust
asm!("out dx, al", in("dx") 0x64u16, in("al") 0xFEu8);
```
Envoie la commande `0xFE` au port `0x64` (contrôleur PS/2) → le CPU redémarre.

---

## Comparaison de chaînes

Sans `std`, pas de `==` sur `&[u8]`. On compare manuellement :

```rust
fn eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] { return false; }
        i += 1;
    }
    true
}
```

---

## Piège Rust : `true;` vs `true`

```rust
// ❌ Retourne () — le ; transforme l'expression en statement
let ok = unsafe { true; };

// ✅ Retourne bool — sans ; c'est la valeur de retour du bloc
let ok = unsafe { true };
```

En Rust, la **dernière expression sans `;`** est la valeur de retour du bloc.
Avec `;` ça devient un statement qui retourne `()` (unit / rien).

---

## Piège Rust : `unsafe` imbriqué

```rust
// ❌ Warning : unsafe inutile car déjà dans un bloc unsafe
unsafe {
    crate::print!("...");  // print! contient déjà son propre unsafe
}

// ✅ Extraire la logique unsafe, puis appeler print! en dehors
let ok = unsafe { ... };
if ok {
    crate::print!("...");
}
```

---

## Structure du projet
```
src/
├── main.rs        → point d'entrée, macros, modules
├── vga.rs         → affichage VGA (clear, scroll, backspace)
├── interrupt.rs   → IDT, handlers exceptions + IRQ stubs
├── pic.rs         → PIC 8259 init + EOI
├── keyboard.rs    → scancode → ASCII, gestion Shift
├── mouse.rs       → souris PS/2, paquets 3 octets
└── shell.rs       → mini-shell (buffer, commandes)
```
