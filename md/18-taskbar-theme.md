# Étape 18 — Thème, Bouton Fermer & Taskbar

## Concept

Cette étape introduit un **système de thème centralisé**, un **bouton de fermeture** pour chaque fenêtre, une **optimisation du rendu des bordures**, et une **taskbar** permettant de masquer et restaurer les fenêtres.

---

## 1. Système de thème (`ui/theme.rs`)

### Palette de base

La palette de base regroupe toutes les couleurs brutes de l'interface. Elle ne doit jamais être utilisée directement dans le code de rendu — elle sert uniquement de source pour les couleurs sémantiques.

| Constante     | Valeur (ARGB) | Description            |
|---------------|---------------|------------------------|
| `BLACK`       | `0xFF_00_00_00` | Noir pur             |
| `WHITE`       | `0xFF_FF_FF_FF` | Blanc pur            |
| `GRAY_DARK`   | `0xFF_1E_1E_1E` | Gris très foncé      |
| `GRAY_MID`    | `0xFF_3C_3C_3C` | Gris moyen           |
| `GRAY_LIGHT`  | `0xFF_6E_6E_6E` | Gris clair           |
| `GRAY_SILVER` | `0xFF_C0_C0_C0` | Argent               |
| `BLUE_DARK`   | `0xFF_00_33_66` | Bleu foncé (titres)  |
| `BLUE_MID`    | `0xFF_00_55_AA` | Bleu moyen (boutons) |
| `RED`         | `0xFF_CC_00_00` | Rouge (fermeture)    |

### Couleurs sémantiques

Les couleurs sémantiques référencent les couleurs de base et définissent **à quoi** sert chaque couleur dans l'interface.

| Constante        | Référence      | Rôle                                      |
|------------------|----------------|-------------------------------------------|
| `DESKTOP_BG`     | `GRAY_DARK`    | Fond du bureau                            |
| `TASKBAR_BG`     | `GRAY_MID`     | Fond de la taskbar                        |
| `WIN_BORDER`     | `GRAY_LIGHT`   | Bordure des fenêtres                      |
| `WIN_TITLE_BG`   | `BLUE_DARK`    | Fond de la barre de titre                 |
| `WIN_TITLE_FG`   | `WHITE`        | Texte de la barre de titre                |
| `WIN_CONTENT`    | `GRAY_SILVER`  | Fond du contenu des fenêtres              |
| `WIN_CLOSE`      | `RED`          | Bouton de fermeture                       |
| `BTN_VISIBLE`    | `BLUE_MID`     | Bouton taskbar — fenêtre visible          |
| `BTN_HIDDEN`     | `GRAY_LIGHT`   | Bouton taskbar — fenêtre masquée          |
| `BTN_FG`         | `WHITE`        | Texte des boutons taskbar                 |
| `TEXT_FG`        | `BLACK`        | Texte général                             |
| `TEXT_BG`        | `WHITE`        | Fond texte général                        |

### Pourquoi deux niveaux ?

Séparer palette et sémantique permet de **changer une couleur de base une seule fois** pour qu'elle se répercute partout où elle est utilisée. Par exemple, remplacer `BLUE_DARK` dans la palette change automatiquement `WIN_TITLE_BG` sans toucher au code de rendu.

```
Palette de base          Couleurs sémantiques        Rendu
──────────────           ────────────────────        ──────
BLUE_DARK ──────────────▶ WIN_TITLE_BG ────────────▶ draw_one()
                        ▶ (autres usages potentiels)
```

---

## 2. Bouton Fermer (`ui/window.rs`)

### Constante

```rust
const CLOSE_BTN_SIZE: u32 = 14; // taille du carré en pixels
```

### Position

Le bouton est placé dans le coin haut-droit de la barre de titre :

```
btn_x = win.x + win.w - BORDER - CLOSE_BTN_SIZE - 2
btn_y = win.y + BORDER + 2
```

| Variable | Valeur                                      | Description                    |
|----------|---------------------------------------------|--------------------------------|
| `btn_x`  | `win.x + win.w - BORDER - CLOSE_BTN_SIZE - 2` | Bord droit, avec marge       |
| `btn_y`  | `win.y + BORDER + 2`                        | Sous la bordure supérieure     |

### Comportement au clic

```
1. Détecter si (mx, my) est dans [btn_x .. btn_x + CLOSE_BTN_SIZE] × [btn_y .. btn_y + CLOSE_BTN_SIZE]
2. Si oui → window.visible = false
          → NEEDS_REDRAW = true
          → retourner (ne pas démarrer un drag)
```

### Flag `NEEDS_REDRAW`

| Fonction           | Rôle                                                        |
|--------------------|-------------------------------------------------------------|
| `needs_redraw()`   | Retourne `true` si le flag est levé, puis le **remet à false** (consomme le flag) |
| `set_needs_redraw()` | Lève le flag depuis l'extérieur (utilisé par la taskbar) |

Ce flag permet de déclencher un redraw complet sans qu'un drag soit en cours — par exemple après avoir fermé une fenêtre ou cliqué sur la taskbar.

---

## 3. Optimisation du rendu des bordures (`draw_one`)

### Problème : overdraw

L'ancienne implémentation remplissait tout le rectangle de la fenêtre avec `BORDER_COLOR`, puis redessinait la barre de titre et le contenu par-dessus. Résultat : certains pixels étaient écrits **3 fois**.

```
Avant :
  fill_rect(win, BORDER_COLOR)     → toute la fenêtre en couleur de bordure
  fill_rect(title_area, TITLE_BG)  → écrase la zone titre
  fill_rect(content_area, CONTENT) → écrase la zone contenu
  → pixels de bordure : écrits 1 fois ✓
  → pixels de titre   : écrits 2 fois ✗
  → pixels de contenu : écrits 2 fois ✗
```

### Solution : 4 `fill_rect` pour les 4 bords

```
Après :
  fill_rect(bord haut)    → 1 pixel de hauteur, toute la largeur
  fill_rect(bord bas)     → 1 pixel de hauteur, toute la largeur
  fill_rect(bord gauche)  → toute la hauteur, 1 pixel de largeur
  fill_rect(bord droit)   → toute la hauteur, 1 pixel de largeur
  fill_rect(barre titre)
  fill_rect(contenu)
  → chaque pixel écrit exactement 1 fois ✓
```

### Variables locales

Pour éviter la répétition de `win.x`, `win.y`, etc. dans chaque appel :

```rust
let x = win.x;
let y = win.y;
let w = win.w;
let h = win.h;
```

---

## 4. Taskbar (`ui/taskbar.rs`)

### Constantes

| Constante    | Valeur | Description                            |
|--------------|--------|----------------------------------------|
| `TASKBAR_H`  | `30`   | Hauteur de la barre (pixels) — `pub`   |
| `BTN_W`      | `120`  | Largeur d'un bouton de fenêtre         |
| `BTN_H`      | `22`   | Hauteur d'un bouton de fenêtre         |
| `BTN_MARGIN` | `4`    | Marge entre les boutons et les bords   |

`TASKBAR_H` est déclarée `pub` pour que `window.rs` puisse l'importer et contraindre le clamp de position.

### Disposition

```
┌─────────────────────────────────────────────────────────────┐  ← écran
│                                                             │
│                         bureau                              │
│                                                             │
├──────────────────────────────────────────────────────────── ┤  ← sh - TASKBAR_H
│ [  Fenêtre 1  ] [  Fenêtre 2  ] [  Fenêtre 3  ]            │  ← taskbar (30px)
└─────────────────────────────────────────────────────────────┘
```

### `draw()`

```
1. fill_rect(0, sh - TASKBAR_H, sw, TASKBAR_H, TASKBAR_BG)
2. Pour chaque fenêtre i dans WINDOWS[0..WINDOW_COUNT] :
     btn_x = BTN_MARGIN + i * (BTN_W + BTN_MARGIN)
     btn_y = sh - TASKBAR_H + (TASKBAR_H - BTN_H) / 2
     couleur = si window.visible → BTN_VISIBLE, sinon → BTN_HIDDEN
     fill_rect(btn_x, btn_y, BTN_W, BTN_H, couleur)
     draw_str(titre de la fenêtre, btn_x + 4, btn_y + 4, BTN_FG)
```

### `on_click(mx, my) → bool`

Retourne `true` si le clic est dans la taskbar (pour indiquer qu'un redraw est nécessaire).

```
1. Si my < sh - TASKBAR_H → retourner false (clic hors taskbar)
2. Pour chaque fenêtre i :
     Calculer btn_x, btn_x + BTN_W
     Si mx dans [btn_x .. btn_x + BTN_W] :
       si window.visible :
         window.visible = false    → masquer
       sinon :
         window.visible = true     → restaurer
         bring_to_front(i)         → remettre au premier plan
       retourner true
3. Retourner true (clic dans la taskbar mais pas sur un bouton)
```

### Clamp fenêtre avec la taskbar

Lors du drag, la position Y d'une fenêtre est maintenant contrainte pour qu'elle ne passe **pas derrière la taskbar** :

```rust
new_y = new_y.min(screen_h - win.h - TASKBAR_H)
```

---

## 5. Intégration dans `mouse.rs`

### Ordre de priorité des clics

```
on_mouse_down(mx, my) :
  1. taskbar::on_click(mx, my)   → priorité max (barre en bas)
       si retourne true → set_needs_redraw() + return
  2. Vérifier is_dragging()      → déjà en drag → return
  3. Vérifier bouton fermer      → clic sur ✕ → return
  4. Démarrer drag sur barre de titre
```

La taskbar est testée **avant** le check de drag pour éviter qu'un clic sur un bouton taskbar ne démarre accidentellement un drag.

### Redraw après actions sans drag

```
on_mouse_move() ou on_mouse_up() :
  si needs_redraw() :
    clear(BACK_BUF, DESKTOP_BG)
    draw_all()
    taskbar::draw()
    dessiner curseur
    swap_buffers()
```

`taskbar::draw()` est appelé **après** `draw_all()` dans tous les redraws, de sorte que la barre s'affiche toujours par-dessus le fond du bureau.

---

## Points techniques

- **Deux niveaux de couleurs** : la palette de base isole les valeurs ARGB brutes ; les couleurs sémantiques expriment l'intention — plus lisible et plus facile à thèmer
- **`NEEDS_REDRAW` consommé** : `needs_redraw()` remet le flag à `false` en le lisant, ce qui évite les redraws doubles si la fonction est appelée plusieurs fois
- **`TASKBAR_H` pub** : permet à `window.rs` d'importer la constante sans dupliquer sa valeur
- **Clamp taskbar** : sans le clamp, une fenêtre pourrait être glissée derrière la taskbar et devenir impossible à attraper
- **Priorité taskbar avant drag** : garantit que les clics sur les boutons taskbar ne déclenchent jamais un drag involontaire
- **Optimisation bordures** : passer de 1 gros `fill_rect` + écrasement à 4 `fill_rect` ciblés élimine l'overdraw et réduit le nombre d'écritures mémoire par frame
