# 20 — Focus clavier et fenêtre active

## Concept

Le **focus** détermine quelle fenêtre reçoit les frappes clavier.
Une seule fenêtre peut être focusée à la fois. Si aucune n'est focusée → le clavier est silencieux.

## Comportement

| Action | Résultat |
|---|---|
| Clic sur barre de titre | Fenêtre focusée + brought to front |
| Clic dans le vide | Focus perdu |
| Bouton fermer | Focus perdu si c'était la fenêtre active |
| Taskbar : hide | Focus perdu |
| Taskbar : show | Fenêtre focusée automatiquement |
| Boot | Terminal focusé par défaut |

## Indicateur visuel

La barre de titre change de couleur selon le focus :

```
Fenêtre inactive  → WIN_TITLE_BG         (0x336699, bleu moyen)
Fenêtre active    → WIN_TITLE_BG_FOCUSED (0x4488CC, bleu clair)
```

## Implémentation

### `window.rs`

```rust
static mut FOCUSED_WIN: usize = usize::MAX;  // usize::MAX = aucun focus

pub fn get_focused_win() -> usize { unsafe { FOCUSED_WIN } }
pub fn set_focused_win(idx: usize) { unsafe { FOCUSED_WIN = idx; } }
```

`draw_one(win, idx)` choisit la couleur du titre selon `FOCUSED_WIN == idx`.

`on_mouse_down` :
- Clic barre de titre → `FOCUSED_WIN = idx`
- Clic bouton fermer → `FOCUSED_WIN = usize::MAX` si c'était la fenêtre active
- Clic dans le vide (fin de boucle) → `FOCUSED_WIN = usize::MAX`

### `taskbar.rs`

`on_click` :
- Hide → `set_focused_win(usize::MAX)` si c'était la fenêtre active
- Show → `set_focused_win(i)` pour focus automatique

### `keyboard.rs`

```rust
let focused = crate::ui::window::get_focused_win();
let bound = unsafe { (*core::ptr::addr_of!(crate::io::framebuffer::FB_WRITER)).bound_win };
if focused == bound {
    crate::shell::on_char(ch);
}
```

Le clavier route vers le shell **seulement** si la fenêtre Terminal est focusée.
`usize::MAX` (aucun focus) → aucune fenêtre ne reçoit les touches.

### `main.rs`

```rust
let term_idx = ui::window::create_window(...);
ui::window::set_focused_win(term_idx);  // focus par défaut au boot
```

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/ui/theme.rs` | `WIN_TITLE_BG_FOCUSED = 0x4488CC` |
| `src/ui/window.rs` | `FOCUSED_WIN`, `get/set_focused_win`, couleur titre dynamique, perte focus |
| `src/ui/taskbar.rs` | focus sur show, perte focus sur hide |
| `src/drivers/keyboard.rs` | route clavier seulement si fenêtre liée est focusée |
| `src/main.rs` | `set_focused_win(term_idx)` au boot |
