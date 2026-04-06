# Étape 16 — Framebuffer Graphique

## Concept

Le framebuffer remplace le mode texte VGA par un vrai affichage graphique pixel par pixel. On passe de 80×25 caractères à une résolution de 800×600 en 32 bits par pixel (BGRA), ce qui permet d'afficher des fenêtres, une souris, des polices lisibles, etc.

## Demande du framebuffer (boot.asm)

Dans le header Multiboot2, on ajoute un **tag framebuffer** (type 5) pour demander au bootloader un mode graphique :

```
; Tag framebuffer (type 5)
dw 5            ; type
dw 0            ; flags
dd 20           ; taille du tag
dd 800          ; largeur demandée
dd 600          ; hauteur demandée
dd 32           ; profondeur (bits par pixel)
```

GRUB/Multiboot2 configure le GPU et nous donne l'adresse du framebuffer dans les tags de boot.

## Identity Map 4 Go

Le framebuffer est mappé en mémoire physique par le GPU (souvent au-dessus de 1 Go). Pour y accéder, on doit **identity mapper toute la plage 0–4 Go** :

```
4 tables P2 × 512 entrées × 2 Mo = 4 Go mappés
```

Chaque entrée P2 utilise des **pages de 2 Mo** (bit PS = 1) pour couvrir tout l'espace sans avoir besoin de tables P1. Cela garantit que toute adresse physique (y compris celle du framebuffer) est accessible via la même adresse virtuelle.

## Parsing du tag framebuffer (multiboot.rs)

### Structure

```rust
#[repr(C, packed)]
pub struct FramebufferTag {
    pub tag_type:  u32,
    pub size:      u32,
    pub addr:      u64,     // adresse physique du framebuffer
    pub pitch:     u32,     // octets par ligne (peut inclure du padding)
    pub width:     u32,     // largeur en pixels
    pub height:    u32,     // hauteur en pixels
    pub bpp:       u8,      // bits par pixel (32)
    pub fb_type:   u8,
    // ...
}
```

### Fonction

| Fonction | Rôle |
|---|---|
| `find_framebuffer(mbi: usize) → Option<&FramebufferTag>` | Parcourt les tags Multiboot2, cherche type 8 (framebuffer), retourne le tag si trouvé |

Si le tag n'existe pas (QEMU sans `-vga std`), on reste en mode VGA texte.

## Module : `src/io/framebuffer.rs`

### Variables globales

| Variable | Type | Rôle |
|---|---|---|
| `FB_PTR` | `*mut u8` | Pointeur vers le framebuffer physique |
| `FB_PITCH` | `u32` | Octets par ligne (inclut le padding GPU) |
| `FB_WIDTH` | `u32` | Largeur en pixels (800) |
| `FB_HEIGHT` | `u32` | Hauteur en pixels (600) |
| `FB_ACTIVE` | `bool` | `true` si le framebuffer est disponible |
| `BACK_BUF` | `[u8; 800×600×4]` | Buffer arrière pour le double buffering |

### Fonctions graphiques

| Fonction | Rôle |
|---|---|
| `init(tag: &FramebufferTag)` | Initialise les variables globales avec les infos du tag |
| `put_pixel(x, y, color)` | Écrit un pixel BGRA dans le back buffer |
| `get_pixel(x, y) → u32` | Lit un pixel depuis le back buffer |
| `fill_rect(x, y, w, h, color)` | Remplit un rectangle de pixels |
| `clear(color)` | Remplit tout l'écran d'une couleur |
| `draw_char(x, y, c, fg, bg)` | Dessine un caractère 8×16 à la position pixel |
| `draw_string(x, y, s, fg, bg)` | Dessine une chaîne de caractères pixel par pixel |

### Pitch vs Width×4

Le **pitch** (octets par ligne) n'est pas toujours égal à `width × 4`. Le GPU peut ajouter du **padding** en fin de ligne pour l'alignement mémoire. On utilise toujours le pitch pour calculer l'offset d'un pixel :

```
offset = y * pitch + x * 4
```

Si on utilisait `width × 4` à la place, l'image serait décalée/déformée sur certains GPU.

## Police bitmap : `src/io/font.rs`

La police est une table de **128 caractères ASCII** au format VGA CP437 :

```rust
pub static FONT_8X16: [[u8; 16]; 128] = [ ... ];
```

- Chaque caractère fait **8 pixels de large × 16 pixels de haut**
- Chaque ligne du caractère = 1 octet (8 bits = 8 pixels)
- Bit à 1 → pixel de la couleur du texte (foreground)
- Bit à 0 → pixel de la couleur de fond (background)

Pour dessiner le caractère `'A'` à la position (x, y) :

```
Pour chaque ligne (0..16) :
    octet = FONT_8X16['A' as usize][ligne]
    Pour chaque bit (0..8) :
        si bit allumé → put_pixel(x + bit, y + ligne, fg)
        sinon         → put_pixel(x + bit, y + ligne, bg)
```

## FbWriter : affichage texte sur framebuffer

Le `FbWriter` gère un **curseur texte** (ligne, colonne) par-dessus le framebuffer graphique, comme un terminal :

### Variables

| Variable | Rôle |
|---|---|
| `CURSOR_X` | Colonne courante (en caractères, 0..100) |
| `CURSOR_Y` | Ligne courante (en caractères, 0..37) |

### Fonctions

| Fonction | Rôle |
|---|---|
| `write_byte(b: u8)` | Écrit un octet : gère `\n`, backspace, retour à la ligne, scroll si nécessaire |
| `scroll()` | Scroll pixel-perfect : copie chaque ligne de pixels 16px vers le haut, efface la dernière ligne |
| `backspace()` | Recule le curseur d'un caractère et efface la cellule |
| `clear_screen()` | Efface tout l'écran et remet le curseur à (0, 0) |

### Scroll pixel-perfect

Contrairement au scroll VGA (copie de mots mémoire), le scroll framebuffer copie des **lignes de pixels** :

```
Pour chaque ligne de pixels y (de 16 à height) :
    copie la ligne y → ligne y - 16
Efface les 16 dernières lignes de pixels (la dernière ligne de texte)
```

Le résultat est un défilement fluide, pixel par pixel.

## Macro print!/println! conditionnelle

Les macros `print!` et `println!` vérifient `FB_ACTIVE` pour choisir le backend :

```
print!("Bonjour")
    → si FB_ACTIVE   → FbWriter::write_byte() pour chaque octet
    → si !FB_ACTIVE  → VGA Writer classique (mode texte 80×25)
```

Le fallback VGA garantit que le kernel peut toujours afficher du texte même si le bootloader n'a pas fourni de framebuffer (par exemple sous QEMU sans `-vga std`).

## Double Buffering

### Problème

Écrire directement dans le framebuffer provoque du **tearing** : l'écran affiche un mix entre l'ancienne et la nouvelle image pendant le dessin.

### Solution

On dessine dans un **buffer arrière** (`BACK_BUF`) en RAM, puis on copie tout d'un coup dans le framebuffer :

```rust
pub static mut BACK_BUF: [u8; 800 * 600 * 4] = [0; 800 * 600 * 4];

pub fn swap_buffers() {
    unsafe {
        core::ptr::copy_nonoverlapping(
            BACK_BUF.as_ptr(),
            FB_PTR,
            (FB_HEIGHT * FB_PITCH) as usize,
        );
    }
}
```

### Flux

```
1. clear(BACK_BUF)                → efface le back buffer
2. draw_all() dans BACK_BUF       → dessine fenêtres, texte, curseur
3. swap_buffers()                  → copie BACK_BUF → framebuffer (GPU)
```

L'utilisateur ne voit jamais d'image incomplète.

## Points techniques

- **Identity map 4 Go** : nécessaire car l'adresse du framebuffer varie selon le GPU/BIOS — on mappe tout pour être sûr
- **Pitch ≠ Width×4** : toujours utiliser le pitch du tag Multiboot2 pour calculer les offsets
- **`copy_nonoverlapping`** : plus rapide que `copy` car le compilateur sait que source et destination ne se chevauchent pas
- **Fallback VGA** : `FB_ACTIVE = false` → tout passe par le VGA classique, aucun crash si pas de framebuffer
- **Police 8×16** : compromis lisibilité / nombre de caractères à l'écran (100 colonnes × 37 lignes en 800×600)
