# 08 — Clavier ASCII, Backspace & Scroll

## Scancodes → ASCII

Le clavier PS/2 envoie des **scancodes** (numéros de touches), pas des caractères.
Il faut une table de conversion : scancode → ASCII.

### Table scancode Set 1
```
Scancode  Touche      ASCII
0x02      1           '1'
0x10      q           'q'
0x1E      a           'a'
0x2C      z           'z'
0x39      Space       ' '
...
```

Deux tables : `NORMAL` (minuscules) et `SHIFT` (majuscules/symboles).
Index = scancode, valeur = caractère ASCII. `0` = touche spéciale ignorée.

### Gestion Shift
```
Scancode 0x2A = Left Shift appuyé   → SHIFT_HELD = true
Scancode 0xAA = Left Shift relâché  → SHIFT_HELD = false
Scancode 0x36 = Right Shift appuyé  → SHIFT_HELD = true
Scancode 0xB6 = Right Shift relâché → SHIFT_HELD = false
```

Si `SHIFT_HELD` → utilise la table `SHIFT`, sinon `NORMAL`.

### Touches spéciales
| Code ASCII | Touche | Action |
|-----------|--------|--------|
| `8` | Backspace | Efface le dernier caractère |
| `27` | Escape | Ignoré |
| `9` | Tab | Ignoré (pour l'instant) |
| `\n` | Enter | Nouvelle ligne |
| `0` | Spéciale | Ignoré (Ctrl, Alt, etc.) |

---

## Backspace

Méthode `backspace()` dans `Writer` :
1. Recule `col` de 1
2. Écrit un espace à la position précédente (efface visuellement)
3. Ne remonte pas à la ligne précédente (le shell n'en a pas besoin)

```rust
pub fn backspace(&mut self) {
    if self.col > 0 {
        self.col -= 1;
        let offset = (self.row * WIDTH + self.col) * 2;
        unsafe {
            *VGA.add(offset)     = b' ';
            *VGA.add(offset + 1) = self.color;
        }
    }
}
```

---

## Scroll

Avant : quand `row >= 25` → `clear()` efface tout l'écran.
Maintenant : `scroll()` décale toutes les lignes vers le haut.

### Algorithme
```
1. Copier ligne 1 → ligne 0
2. Copier ligne 2 → ligne 1
3. ...
4. Copier ligne 24 → ligne 23
5. Effacer ligne 24 (dernière)
6. Curseur reste sur ligne 24, colonne 0
```

Chaque case VGA = 2 octets (caractère + couleur), donc on copie par paires.

```rust
pub fn scroll(&mut self) {
    for row in 1..HEIGHT {
        for col in 0..WIDTH {
            let from = (row * WIDTH + col) * 2;
            let to = ((row - 1) * WIDTH + col) * 2;
            unsafe {
                *VGA.add(to)     = *VGA.add(from);
                *VGA.add(to + 1) = *VGA.add(from + 1);
            }
        }
    }
    // Efface la dernière ligne
    for col in 0..WIDTH {
        let offset = ((HEIGHT - 1) * WIDTH + col) * 2;
        unsafe {
            *VGA.add(offset)     = b' ';
            *VGA.add(offset + 1) = self.color;
        }
    }
    self.row = HEIGHT - 1;
    self.col = 0;
}
```

---

## Structure du code

```
keyboard.rs
├── NORMAL[58]      table scancode → ASCII minuscule
├── SHIFT[58]       table scancode → ASCII majuscule
├── SHIFT_HELD      état du shift (static mut bool)
└── on_key()        dispatch : shift/backspace/escape/caractère

vga.rs (Writer)
├── clear()         efface tout l'écran
├── scroll()        décale les lignes vers le haut
├── backspace()     efface le caractère précédent
├── write_byte()    écrit un caractère (appelle scroll si nécessaire)
└── write_str_bytes() écrit une chaîne
```

---

## Concepts clés
| Concept | Explication |
|---------|------------|
| Scancode Set 1 | Standard PS/2, chaque touche = un numéro fixe |
| `>= 0x80` | Relâchement = scancode + 0x80 |
| VGA 2 octets | Chaque case = [caractère, couleur] |
| Scroll vs Clear | Scroll garde l'historique visible, clear efface tout |
| `static mut` | Variable globale mutable — nécessite `unsafe` (pas de mutex en no_std) |
