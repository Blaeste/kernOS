# 25 — KLib : Bibliothèque utilitaire du kernel

## Objectif

Centraliser les fonctions bas-niveau réutilisées dans tout le kernel.
Avant klib, chaque module réimplémentait ses propres `name_len`, `name_eq`,
boucles de copie, formatage de nombres, etc. Maintenant tout est dans `src/klib/`.

---

## Structure

```
src/klib/
├── mod.rs      → déclare les sous-modules
├── string.rs   → manipulation de chaînes &[u8]
├── mem.rs      → opérations mémoire (copie, remplissage)
└── cmp.rs      → comparaisons (min, max)
```

---

## klib::string

| Fonction | Signature | Rôle |
|----------|-----------|------|
| `eq` | `(a: &[u8], b: &[u8]) -> bool` | Comparaison exacte de deux slices |
| `strlen` | `(s: &[u8]) -> usize` | Longueur jusqu'au premier `\0` (comme le strlen C) |
| `to_str` | `(bytes: &[u8]) -> &str` | Convertit en `&str` (fallback `"???"`) |
| `to_str_or` | `(bytes, fallback) -> &str` | Idem avec fallback personnalisé |
| `starts_with` | `(s, prefix) -> bool` | Vérifie un préfixe |
| `find_space` | `(s: &[u8]) -> Option<usize>` | Position du premier espace |
| `itoa` | `(n: u32, buf: &mut [u8]) -> &[u8]` | Entier → ASCII décimal (droite à gauche) |
| `atoi` | `(s: &[u8]) -> u32` | ASCII décimal → entier |
| `pad2` | `(n: u32, buf: &mut [u8])` | Écrit 2 chiffres zero-padded (ex: `05`) |

### Pourquoi `&[u8]` et pas `&str` ?

En `no_std` sans allocateur de strings, tout est `&[u8]`.
Les noms de fichiers viennent de buffers fixes (`[u8; 32]`), pas d'UTF-8 garanti.
`to_str` fait le pont vers `core::fmt` quand on a besoin d'afficher.

---

## klib::mem

| Fonction | Signature | Rôle |
|----------|-----------|------|
| `memcpy` | `(dst: &mut [u8], src: &[u8], len)` | Copie `len` octets de src vers dst |
| `memset` | `(dst: &mut [u8], val: u8, len)` | Remplit `len` octets avec `val` |
| `bzero` | `(dst: &mut [u8], len)` | Met à zéro (alias `memset(dst, 0, len)`) |

### Protection

Les trois fonctions sont protégées par bounds-checking sur les slices :
`memcpy` s'arrête au `min(len, dst.len(), src.len())`, jamais de débordement.

---

## klib::cmp

| Fonction | Signature | Rôle |
|----------|-----------|------|
| `min` | `(a: usize, b: usize) -> usize` | Retourne le plus petit |
| `max` | `(a: usize, b: usize) -> usize` | Retourne le plus grand |

---

## Code remplacé

| Fichier | Avant | Après |
|---------|-------|-------|
| `kfs.rs` | `name_len()` locale | `klib::string::strlen()` |
| `kfs.rs` | `name_eq()` locale | `klib::string::eq()` |
| `kfs.rs` | boucles `while k < len` | `klib::mem::memcpy()` |
| `kfs.rs` | `if x < y { x } else { y }` ×3 | `klib::cmp::min()` |
| `ramfs.rs` | `name_len()` locale | `klib::string::strlen()` |
| `ramfs.rs` | boucles `for j in 0..len` | `klib::mem::memcpy()` |
| `ramfs.rs` | `if data.len() > MAX` | `klib::cmp::min()` |
| `window.rs` | boucle copie titre | `klib::mem::memcpy()` |
| `window.rs` | `if bytes.len() < MAX_TITLE` | `klib::cmp::min()` |
| `filewin.rs` | boucle `% 10` manuelle | `klib::string::itoa()` |
| `taskbar.rs` | `b'0' + (mins / 10)` | `klib::string::pad2()` |
