# 22 — Fenêtre Fichiers (ramfs visuel)

## Résultat

La fenêtre **Fichiers** affiche la liste des fichiers du ramfs avec leur taille en octets,
mise à jour à chaque redraw (drag, clear, etc.).

```
┌─ Fichiers ──────────────────────────────────┐
│ test                                      11 │
│ hhhh                                       4 │
│                                              │
└──────────────────────────────────────────────┘
```

## Implémentation

### `fs/ramfs.rs` — `list(cb)`

Nouvelle fonction qui itère les fichiers sans `println` :

```rust
pub fn list(mut cb: impl FnMut(&[u8], usize)) {
    for i in 0..MAX_FILES {
        let f = &files()[i];
        if f.used {
            cb(&f.name[..name_len(&f.name)], f.size);
        }
    }
}
```

### `ui/filewin.rs` — module dédié

- `FILE_WIN_IDX = 1` : index fixe de la fenêtre Fichiers
- `draw()` : lit la position/taille depuis `WINDOWS[1]`, efface le fond, liste les fichiers ligne par ligne
- Nom à gauche, taille en octets à droite (formatage manuel sans alloc)
- Affiche `(vide)` si aucun fichier

### `ui/window.rs` — `draw_all()`

```rust
if idx == crate::ui::filewin::FILE_WIN_IDX {
    crate::ui::filewin::draw();
}
```

Le contenu est redessiné automatiquement au bon z-order à chaque redraw.

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/fs/ramfs.rs` | `pub fn list(cb)` — itération sans println |
| `src/ui/filewin.rs` | Nouveau module, `draw()` |
| `src/ui/mod.rs` | `pub mod filewin` |
| `src/ui/window.rs` | `draw_all` appelle `filewin::draw()` au bon z-order |
| `src/main.rs` | `ui::filewin::draw()` au boot |
