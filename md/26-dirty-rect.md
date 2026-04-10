# Étape 26 — Dirty Rectangles (optimisation framebuffer)

## Problème

`swap_buffers()` copiait **tout** le back buffer vers le framebuffer à chaque frame :

```
1920 × 1080 × 4 octets = ~8 Mo par swap
```

Même si un seul pixel change, on copie 8 Mo. Pendant un drag de fenêtre ou un simple clignotement de curseur, c'est du gaspillage massif.

## Solution : Dirty Rectangle

On traque la **zone modifiée** depuis le dernier swap. Au lieu de copier tout l'écran, on ne copie que le rectangle englobant toutes les modifications.

### Variables globales

```rust
static mut DIRTY_X1: u32 = 0;
static mut DIRTY_Y1: u32 = 0;
static mut DIRTY_X2: u32 = 0;
static mut DIRTY_Y2: u32 = 0;
```

`(DIRTY_X1, DIRTY_Y1)` = coin haut-gauche, `(DIRTY_X2, DIRTY_Y2)` = coin bas-droit.
Si `DIRTY_X2 == 0 && DIRTY_Y2 == 0` → rien n'a changé → `swap_buffers()` retourne immédiatement.

### `mark_dirty(x, y, w, h)`

Agrandit le dirty rect pour englober la nouvelle zone :

```rust
pub fn mark_dirty(x: u32, y: u32, w: u32, h: u32) {
    unsafe {
        if DIRTY_X2 == 0 && DIRTY_Y2 == 0 {
            DIRTY_X1 = x; DIRTY_Y1 = y;
            DIRTY_X2 = x + w; DIRTY_Y2 = y + h;
        } else {
            if x < DIRTY_X1 { DIRTY_X1 = x; }
            if y < DIRTY_Y1 { DIRTY_Y1 = y; }
            if x + w > DIRTY_X2 { DIRTY_X2 = x + w; }
            if y + h > DIRTY_Y2 { DIRTY_Y2 = y + h; }
        }
    }
}
```

### `swap_buffers()` optimisé

Ne copie que les lignes du dirty rect, et seulement les colonnes concernées :

```rust
pub fn swap_buffers() {
    unsafe {
        if DIRTY_X2 == 0 && DIRTY_Y2 == 0 { return; }

        let x1 = DIRTY_X1 as usize;
        let y1 = DIRTY_Y1 as usize;
        let x2 = DIRTY_X2 as usize;
        let y2 = DIRTY_Y2 as usize;
        let copy_w = x2 - x1;

        let mut y = y1;
        while y < y2 {
            let src = BACK_BUF[y * w + x1..];
            let dst = framebuffer[y * pitch/4 + x1..];
            copy_nonoverlapping(src, dst, copy_w);
            y += 1;
        }

        // Reset
        DIRTY_X1 = 0; DIRTY_Y1 = 0;
        DIRTY_X2 = 0; DIRTY_Y2 = 0;
    }
}
```

## Qui appelle `mark_dirty` ?

L'appel est fait au **niveau le plus haut possible** pour éviter des milliers d'appels par pixel :

| Fonction | Appel `mark_dirty` | Pourquoi |
|---|---|---|
| `put_pixel(x, y, color)` | **Non** | Trop fréquent (appelé par draw_char, fill_rect…) |
| `draw_char(x, y, c, fg, bg)` | `mark_dirty(x, y, 8, 16)` | 1 appel pour 128 pixels |
| `fill_rect(x, y, w, h, color)` | `mark_dirty(x, y, w, h)` | 1 appel pour tout le rectangle |
| `clear(color)` | `mark_dirty(0, 0, w, h)` | 1 appel pour tout l'écran |
| `scroll()` | `mark_dirty(x_origin, y_origin, zone_w, zone_h)` | 1 appel pour toute la zone texte |
| `mouse.rs` — effacement curseur | `mark_dirty(ox, oy, 12, 16)` | 1 appel pour la zone du sprite |
| `mouse.rs` — dessin curseur | `mark_dirty(nx, ny, 12, 16)` | 1 appel pour la zone du sprite |

### Pourquoi pas dans `put_pixel` ?

`put_pixel` est appelé en boucle par `draw_char` (128×), `fill_rect` (w×h fois), le curseur souris (192×)… Mettre `mark_dirty` dedans = 4 comparaisons × chaque pixel = inutile. Marquer une seule fois au niveau de la fonction englobante est bien plus efficace.

## Gain

| Scénario | Avant | Après |
|---|---|---|
| Rien ne bouge | Copie 8 Mo | Copie 0 octet (early return) |
| Un caractère tapé | Copie 8 Mo | Copie 8×16 = 128 pixels = 512 octets |
| Curseur souris bouge | Copie 8 Mo | Copie ~12×16 × 2 zones ≈ 1.5 Ko |
| Drag fenêtre | Copie 8 Mo | Copie le rect englobant la fenêtre |

## Fichiers modifiés

| Fichier | Modification |
|---|---|
| `src/io/framebuffer.rs` | Ajout `DIRTY_X1/Y1/X2/Y2`, `mark_dirty()`, `swap_buffers()` optimisé, `mark_dirty` dans `draw_char`, `fill_rect`, `clear`, `scroll` |
| `src/drivers/mouse.rs` | Ajout `mark_dirty` pour effacement et dessin du curseur |
