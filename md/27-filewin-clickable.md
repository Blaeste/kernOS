# 27 — Fenêtre Fichiers cliquable

## Objectif

Rendre la fenêtre Fichiers interactive : cliquer sur un fichier affiche son contenu dans le terminal.

## Fonctionnement

### Enregistrement des fichiers visibles

À chaque `draw()`, on vide `FILE_COUNT_VIS` et on remplit deux tableaux statiques :

- `FILE_NAMES[i]` — nom du fichier i (28 octets, null-terminé)
- `FILE_Y[i]` — coordonnée Y pixel du fichier i à l'écran

```rust
static mut FILE_NAMES:     [[u8; 28]; MAX_VISIBLE] = [[0; 28]; MAX_VISIBLE];
static mut FILE_Y:         [u32; MAX_VISIBLE]       = [0; MAX_VISIBLE];
static mut FILE_COUNT_VIS: usize                    = 0;
```

### Détection du clic

`file_at_y(my)` parcourt `FILE_Y` et retourne l'index du fichier dont la ligne contient `my` :

```rust
fn file_at_y(my: u32) -> Option<usize> {
    // my >= FILE_Y[i] && my < FILE_Y[i] + LINE_H
}
```

### Lecture et affichage

`on_click(mx, my)` :
1. Appelle `file_at_y` pour identifier le fichier
2. Coupe le nom aux octets non-nuls (`&name[..len]`) pour que `kfs::read` puisse le matcher
3. Appelle `kfs::read` → retourne le nombre d'octets lus
4. `println!` le contenu dans le terminal

```rust
pub fn on_click(mx: i32, my: i32) {
    if let Some(idx) = file_at_y(my as u32) {
        let name = &FILE_NAMES[idx];
        let len = strlen(name);
        let mut buf = [0u8; 512];
        let sz = kfs::read(&name[..len], &mut buf);
        if sz > 0 { println!("{}", to_str(&buf[..sz])); }
    }
}
```

### Branchement dans window.rs

Dans `on_mouse_down`, quand le clic tombe dans la zone contenu d'une fenêtre :

```rust
if idx == crate::ui::filewin::FILE_WIN_IDX {
    crate::ui::filewin::on_click(mx, my);
}
```

## Bug corrigé

`kfs::read` compare `&entry.name[..len]` avec le slice passé. Si on passe `&FILE_NAMES[idx]` (28 octets avec zéros), la comparaison échoue. Fix : passer `&name[..len]` pour ne transmettre que les octets significatifs.

## Utilisation

```
kfsWrite test hello    // crée un fichier
// clique sur "test" dans la fenêtre Fichiers
// → "hello" s'affiche dans le terminal
```
