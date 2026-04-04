# 04 — Affichage VGA Text Mode

## Comment ça marche
L'adresse `0xB8000` est une zone mémoire spéciale — le matériel VGA
la lit en continu et affiche son contenu. Pas de driver nécessaire.

## Format d'un caractère
Chaque caractère = **2 octets** :
```
[ octet pair   : caractère ASCII ]
[ octet impair : couleur         ]
```

## Byte de couleur
```
bits 7-4 : couleur du fond
bits 3-0 : couleur du texte

Exemples :
0x0F = blanc brillant sur noir
0x0B = cyan clair sur noir
0x02 = vert sur noir
0x04 = rouge sur noir
```

## Calcul de position
```rust
// Écran = 80 colonnes × 25 lignes
let offset = (row * 80 + col) * 2;  // * 2 car chaque char = 2 octets
```

## Code
```rust
let vga = 0xB8000 as *mut u8;

// Effacer l'écran
for i in 0..2000 {
    unsafe {
        *vga.add(i * 2)     = b' ';
        *vga.add(i * 2 + 1) = 0x00;
    }
}

// Écrire centré ligne 12
let message = b"Welcome to KernOS";
let col = (80 - message.len()) / 2;
let offset = (12 * 80 + col) * 2;

for (i, &byte) in message.iter().enumerate() {
    unsafe {
        *vga.add(offset + i * 2)     = byte;
        *vga.add(offset + i * 2 + 1) = 0x0B;
    }
}
```

## Concepts clés
| Concept | Explication |
|---|---|
| `as *mut u8` | Cast d'adresse en pointeur mutable vers octets |
| `unsafe { }` | Écriture directe en mémoire — Rust ne peut pas vérifier |
| `b"texte"` | Byte string — tableau d'octets ASCII, pas de `String` |
| `.add(n)` | Avance un pointeur de n octets |
| `*ptr` | Déréférencement — accède à la valeur à l'adresse du pointeur |
