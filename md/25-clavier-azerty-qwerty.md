# 25 — Support clavier AZERTY / QWERTY

## Objectif

Pouvoir taper en AZERTY ou QWERTY selon le clavier physique, et switcher à chaud depuis le shell.

## Implémentation

### keyboard.rs

4 tables de scancodes Set 1 :

| Table          | Usage                        |
|----------------|------------------------------|
| `QWERTY_NORMAL`| QWERTY sans Shift            |
| `QWERTY_SHIFT` | QWERTY avec Shift            |
| `AZERTY_NORMAL`| AZERTY sans Shift            |
| `AZERTY_SHIFT` | AZERTY avec Shift            |

Un enum + flag global :
```rust
pub enum Layout { Qwerty, Azerty }
pub static mut LAYOUT: Layout = Layout::Qwerty;
```

Dans `on_key`, on sélectionne la bonne table selon `LAYOUT` et `SHIFT_HELD`.

## Commande shell

```
layout azerty   → passe en AZERTY
layout qwerty   → passe en QWERTY
```

Le changement est immédiat, pas de reboot nécessaire.

## Fichiers modifiés

- `src/drivers/keyboard.rs` — 4 tables, enum Layout, LAYOUT flag, on_key mis à jour
- `src/shell.rs` — commande `layout` + help mis à jour
