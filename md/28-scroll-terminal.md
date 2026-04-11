# 28 — Scroll terminal avec historique

## Objectif

Permettre de remonter dans l'historique du terminal avec PgUp/PgDn, sans perdre le contenu affiché.

## Architecture

### Buffer historique

`TEXT_MAX_ROWS` passe de 50 à 200 — le buffer garde les 200 dernières lignes.

Les lignes **actives** (visibles) occupent toujours les `max_rows` dernières entrées du buffer :

```
text_buf[0..150]          ← historique (hors écran)
text_buf[150..199]        ← lignes visibles (max_rows ≈ 50)
```

L'index d'écriture est donc :
```rust
let buf_row = (TEXT_MAX_ROWS - max_rows) + row;
```

### view_offset

Nouveau champ `view_offset: u32` dans FbWriter :
- `0` = vue live (bas du buffer)
- `N` = N lignes remontées dans l'historique

Toute frappe clavier remet `view_offset = 0` automatiquement.

### scroll()

Décale maintenant aussi le `text_buf` d'une ligne vers le haut avant de décaler les pixels :

```rust
let mut i = 0;
while i < TEXT_MAX_ROWS - 1 {
    self.text_buf[i] = self.text_buf[i + 1];
    i += 1;
}
self.text_buf[TEXT_MAX_ROWS - 1] = [b' '; TEXT_MAX_COLS];
```

### redraw_from_buf()

Redessine les `max_rows` lignes visibles depuis le buffer selon `view_offset` :

```rust
let base = (TEXT_MAX_ROWS - max_rows).saturating_sub(view_offset);
// dessine text_buf[base..base+max_rows]
```

### scroll_up() / scroll_down()

```rust
pub fn scroll_up(&mut self)   { view_offset += 1; redraw_from_buf(); }
pub fn scroll_down(&mut self) { view_offset -= 1; redraw_from_buf(); }
```

### Touches PgUp / PgDn (keyboard.rs)

Les touches étendues ont un préfixe `0xE0` → flag `E0_PENDING` :

```
0xE0 reçu → E0_PENDING = true
0x49 reçu + E0_PENDING → scroll_up()
0x51 reçu + E0_PENDING → scroll_down()
```

### redraw_text() simplifié

`redraw_text()` appelle désormais `redraw_from_buf()` — évite le bug où le contenu disparaissait après un clic (NEEDS_REDRAW effaçait le buffer).

## Utilisation

- **PgUp** : remonte dans l'historique
- **PgDn** : redescend vers le live
- Toute frappe : retour automatique au live
