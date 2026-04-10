# 09 — Mini-Shell

## Architecture

Le shell intercepte les caractères du clavier et les accumule dans un buffer.
Quand l'utilisateur appuie sur Enter, la commande est parsée et dispatchée.

```
Touche clavier
    → interrupt.rs (IRQ1 → scancode)
    → keyboard.rs  (scancode → ASCII)
    → shell.rs     (ASCII → buffer → execute)
```

---

## Buffer de commande

```rust
const MAX_CMD: usize = 78;  // 80 colonnes - 2 pour "> "
static mut BUFFER: [u8; MAX_CMD] = [0; MAX_CMD];
static mut LEN: usize = 0;
```

- Tableau fixe de 78 octets (pas d'allocateur → pas de `String`)
- `LEN` = nombre de caractères tapés
- Reset à 0 après chaque Enter

---

## Dispatch des caractères (`on_char`)

| Caractère | Action |
|-----------|--------|
| `\n` (Enter) | Exécute la commande, remet `LEN = 0`, réaffiche le prompt |
| `8` (Backspace) | Recule `LEN`, efface via `backspace()` (VGA ou framebuffer) |
| autre | Ajoute au buffer si pas plein, affiche le caractère |

---

## Parsing : `(name, args)`

Au début de `execute()`, la ligne est découpée sur le **premier espace** :

```rust
let (name, args) = match find_space(cmd) {
    Some(pos) => (&cmd[..pos], &cmd[pos+1..]),
    None      => (cmd, &b""[..]),
};
```

```
"kfsWrite monFichier contenu"
 └──────┘ └──────────────────┘
   name           args
```

- `name` est comparé **exactement** avec `eq()` → `"kfsDel"` ne matche jamais `"kfsDelete"`
- `args` est passé directement aux fonctions (plus de `&cmd[N..]`)

---

## Commandes

| Commande | Args | Action |
|----------|------|--------|
| `help` | — | Liste les commandes |
| `clear` | — | Efface l'écran (VGA ou framebuffer) |
| `reboot` | — | Redémarre via port PS/2 `0x64` |
| `uptime` | — | Affiche le temps en ticks et secondes |
| `memmap` | — | Affiche la carte mémoire GRUB |
| `frames` | — | Nombre de frames physiques libres |
| `pgtest` | — | Test alloc/map/write/read/unmap/free |
| `tasks` | — | Liste les tâches actives du scheduler |
| `ls` | — | Liste les fichiers ramfs |
| `cat` | `nom` | Affiche un fichier ramfs |
| `write` | `nom contenu` | Crée un fichier ramfs |
| `rm` | `nom` | Supprime un fichier ramfs |
| `ataRead` | — | Lit et affiche le secteur 0 du disque |
| `ataWrite` | — | Écrit "KernOS" dans le secteur 0 |
| `kfsLs` | — | Liste les fichiers KFS sur disque |
| `kfsWrite` | `nom contenu` | Crée un fichier KFS sur disque |
| `kfsRead` | `nom` | Lit un fichier KFS depuis le disque |
| `kfsDel` | `nom` | Supprime un fichier KFS du disque |
| *(vide)* | — | Ne fait rien |
| *(inconnu)* | — | Affiche `"commande inconnue: ..."` |

---

### Reboot
```rust
asm!("out dx, al", in("dx") 0x64u16, in("al") 0xFEu8);
```
Envoie la commande `0xFE` au port `0x64` (contrôleur PS/2) → le CPU redémarre.
