# 19 — Window manager : shell lié à sa fenêtre, z-order et redraw

## Problèmes résolus

### 1. Texte du terminal toujours par-dessus toutes les fenêtres

`redraw_text()` était appelé **après** `draw_all()` dans `mouse.rs`.
Résultat : le texte du terminal s'affichait au-dessus de la fenêtre Fichiers même quand elle était devant.

**Fix** : `draw_all()` appelle maintenant `redraw_writer_if_visible()` au bon moment dans le z-order — juste après avoir dessiné la fenêtre liée au writer.

```rust
// window.rs — draw_all()
draw_one(&WINDOWS[idx]);
if idx == bound {
    redraw_writer_if_visible();  // texte dessiné au bon z-order
}
```

### 2. `clear` effaçait tout l'écran

`clear_screen()` appelait `clear(self.bg)` qui remplit **tout le back buffer**.
Résultat : fond gris sur tout l'écran.

**Fix** : `clear_screen()` efface seulement la zone contenu de la fenêtre.

```rust
fill_rect(self.x_origin, self.y_origin, self.max_cols * 8, self.max_rows * 16, self.bg);
```

### 3. Fenêtres fantômes (bug précédent)

Quand une fenêtre était fermée ou cachée, sa zone restait affichée.

**Fix** : `clear(DESKTOP_BG)` ajouté avant `draw_all()` dans `mouse.rs` pour effacer tout le fond avant de redessiner.

---

## Nouveau : FbWriter lié à sa fenêtre

### Champ `bound_win`

`FbWriter` mémorise l'index de sa fenêtre (`usize::MAX` = aucune).

```rust
pub bound_win: usize,
```

### `bind_writer_to_window(x, y, w, h, win_idx)`

Configure le writer sur la zone contenu d'une fenêtre + mémorise l'index.

### `redraw_writer_if_visible()`

- Vérifie que la fenêtre liée est `visible`
- Sync `x_origin/y_origin` sur la position **actuelle** de la fenêtre (drag inclus)
- Appelle `redraw_text()` pour redessiner le texte

```
fenêtre cachée  → skip (texte pas redessiné)
fenêtre visible → sync position + redraw texte au bon z-order
```

### `FbWriter.bg`

Couleur de fond dynamique :
- `DESKTOP_BG` par défaut (plein écran)
- `WIN_CONTENT` quand bindé à une fenêtre

Utilisé dans `write_byte`, `backspace`, `scroll`, `clear_screen`, `redraw_text`.

---

## Résumé des fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/io/framebuffer.rs` | `bound_win`, `bg`, `clear_screen` zone locale, `redraw_text` dans impl |
| `src/ui/window.rs` | `draw_all` intègre `redraw_writer_if_visible` au z-order |
| `src/drivers/mouse.rs` | `clear(DESKTOP_BG)` avant `draw_all`, supprime redraw_text externe |
| `src/main.rs` | `bind_writer_to_window` avec `term_idx` |
