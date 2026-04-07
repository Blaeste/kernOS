# Étape 17 — Window Manager (Fenêtres & Drag and Drop)

## Concept

Le window manager permet d'afficher des **fenêtres graphiques** à l'écran et de les **déplacer à la souris** (drag & drop). Chaque fenêtre a une barre de titre, une position, une taille, et peut être glissée par sa barre de titre.

## Curseur souris visible

### Variables globales

| Variable | Type | Rôle |
|---|---|---|
| `MOUSE_X` | `i32` | Position X courante du curseur (pixels) |
| `MOUSE_Y` | `i32` | Position Y courante du curseur (pixels) |
| `MOUSE_BACKUP` | `[u32; 12×16]` | Sauvegarde des pixels sous le curseur |

### Sprite du curseur

Le curseur est une **flèche 12×16 pixels** définie comme un tableau de `u16` :

```rust
static CURSOR_SHAPE: [u16; 16] = [
    0b1000_0000_0000_0000,  // X
    0b1100_0000_0000_0000,  // XX
    // ... chaque bit = 1 pixel blanc, 0 = transparent
];
```

Le bit de poids fort (0x8000) = pixel le plus à gauche. Seuls les bits à 1 sont dessinés en blanc.

### Dessin du curseur

```
1. Restaurer MOUSE_BACKUP à l'ancienne position  → efface l'ancien curseur
2. Sauvegarder les pixels à la nouvelle position  → MOUSE_BACKUP
3. Dessiner CURSOR_SHAPE à la nouvelle position   → affiche le curseur
```

### init_cursor()

```
1. MOUSE_X = FB_WIDTH / 2        → centre horizontal
2. MOUSE_Y = FB_HEIGHT / 2       → centre vertical
3. Sauvegarder les pixels sous la position initiale dans MOUSE_BACKUP
```

## Structure Window

```rust
pub struct Window {
    pub x:         u32,              // position X (coin supérieur gauche)
    pub y:         u32,              // position Y
    pub w:         u32,              // largeur
    pub h:         u32,              // hauteur
    pub title:     [u8; MAX_TITLE],  // titre (copié, pas de référence)
    pub title_len: usize,           // longueur du titre
    pub visible:   bool,            // fenêtre affichée ou masquée
}
```

## Gestion des fenêtres

### Variables globales

| Variable | Type | Rôle |
|---|---|---|
| `WINDOWS` | `[Window; 8]` | Tableau fixe de 8 fenêtres max |
| `WINDOW_COUNT` | `usize` | Nombre de fenêtres actuellement créées |
| `Z_ORDER` | `[usize; 8]` | Ordre d'affichage (indices dans WINDOWS) |

### Fonctions

| Fonction | Rôle |
|---|---|
| `create_window(x, y, w, h, title)` | Crée une fenêtre dans le prochain slot libre, incrémente `WINDOW_COUNT` |
| `draw_one(idx)` | Dessine une seule fenêtre : barre de titre (fond coloré + texte) + corps (fond blanc/noir) |
| `draw_all()` | Dessine toutes les fenêtres visibles dans l'ordre du tableau (z-order : index 0 = arrière-plan, dernier = devant) |

### Z-order

L'ordre d'affichage est géré par le tableau `Z_ORDER` (indices dans `WINDOWS`) :

```
Z_ORDER[0]  → première fenêtre dessinée → en arrière-plan
Z_ORDER[1]  → dessinée par-dessus
...
Z_ORDER[n]  → dessinée en dernier       → au premier plan
```

Quand on clique sur une fenêtre, `bring_to_front(idx)` déplace son index en dernière position de `Z_ORDER`. Cela ne copie qu'un `usize`, pas la struct `Window` entière → performant.

| Fonction | Rôle |
|---|---|
| `bring_to_front(idx)` | Déplace l'index de la fenêtre en fin de `Z_ORDER` |
| `is_dragging()` | Retourne `true` si un drag est en cours |

## Drag & Drop

### Variables globales

| Variable | Type | Rôle |
|---|---|---|
| `DRAGGING` | `bool` | `true` si une fenêtre est en cours de déplacement |
| `DRAG_WIN` | `usize` | Index de la fenêtre en cours de drag |
| `DRAG_OFFSET_X` | `i32` | Décalage X entre le clic et le coin de la fenêtre |
| `DRAG_OFFSET_Y` | `i32` | Décalage Y entre le clic et le coin de la fenêtre |

### on_mouse_down(x, y, button)

Détection du clic sur une barre de titre :

```
1. Parcourir WINDOWS en ordre INVERSE (du premier plan vers l'arrière)
2. Pour chaque fenêtre visible :
     si (x, y) est dans la barre de titre (y entre window.y et window.y + TITLE_HEIGHT) :
       → DRAGGING = true
       → DRAG_WIN = index de la fenêtre
       → DRAG_OFFSET_X = x - window.x
       → DRAG_OFFSET_Y = y - window.y
       → break (on prend la fenêtre la plus devant)
```

Le parcours **inverse** est essentiel : si deux fenêtres se chevauchent, on attrape celle qui est visuellement devant (index le plus élevé).

### on_mouse_move(dx, dy)

Déplacement de la fenêtre pendant le drag :

```
1. Mettre à jour MOUSE_X += dx, MOUSE_Y += dy
2. Clamp MOUSE_X dans [0, FB_WIDTH - 1]
3. Clamp MOUSE_Y dans [0, FB_HEIGHT - 1]
4. Si DRAGGING :
     → window.x = MOUSE_X - DRAG_OFFSET_X
     → window.y = MOUSE_Y - DRAG_OFFSET_Y
     → Clamp window.x/y pour que la fenêtre reste dans l'écran
     → Redessiner toutes les fenêtres + swap_buffers()
```

Le **clamp** empêche la souris et les fenêtres de sortir de l'écran. Le verrouillage de la souris aux bords garantit une expérience fluide.

### on_mouse_up()

```
DRAGGING = false
```

Le drag s'arrête. La fenêtre reste à sa nouvelle position.

## Flux de rendu complet

```
1. clear(BACK_BUF, couleur_fond)     → efface le back buffer
2. draw_all()                         → dessine toutes les fenêtres (z-order)
3. dessiner le curseur souris         → par-dessus tout
4. swap_buffers()                     → copie vers le framebuffer (GPU)
```

## Points techniques

- **Tableau fixe [Window; 8]** : pas d'allocation dynamique, tout est statique — adapté au bare-metal
- **Z_ORDER séparé** : au lieu de déplacer les structs Window, on déplace seulement les indices → pas de lag
- **Parcours inverse pour le clic** : on teste la fenêtre devant en premier, comme dans un vrai window manager
- **DRAG_OFFSET** : sans l'offset, la fenêtre "sauterait" vers le curseur au début du drag — avec l'offset, elle suit naturellement
- **Verrouillage souris** : pendant le drag, `on_mouse_move` force `MOUSE_X/Y` pour que la souris reste collée à la fenêtre quand elle touche un bord
- **Clamp** : empêche la souris et les fenêtres de sortir des bords de l'écran
- **Double buffering sélectif** : `swap_buffers()` seulement pendant le drag. Le curseur seul utilise `put_pixel_direct` pour écrire directement au framebuffer → pas de lag
- **`if DRAGGING { return; }` dans on_mouse_down** : empêche de changer de fenêtre pendant un drag en cours
