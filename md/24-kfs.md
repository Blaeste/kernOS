# 24 — KFS : KernOS File System

## Objectif

Avoir un vrai filesystem persisté sur disque. Les fichiers survivent au reboot.

## Architecture disque

```
Secteur 0       → SuperBlock (magic, version, file_count)
Secteurs 1-8    → FileEntry (1 entrée par secteur, max 8 fichiers)
Secteurs 9-16   → Données (1 fichier = 1 secteur = 512 octets max)
```

## Structures

### SuperBlock (512 octets, secteur 0)
| Champ       | Type     | Description                        |
|-------------|----------|------------------------------------|
| magic       | u32      | `0x4B465300` = "KFS\0"             |
| version     | u32      | Version du filesystem (1)          |
| file_count  | u32      | Nombre de fichiers (non utilisé)   |
| _pad        | [u8;500] | Padding pour remplir 512 octets    |

### FileEntry (512 octets, secteurs 1-8)
| Champ     | Type      | Description                          |
|-----------|-----------|--------------------------------------|
| used      | u8        | 0 = libre, 1 = occupé               |
| name      | [u8;28]   | Nom du fichier (null-terminated)     |
| size      | u32       | Taille en octets                     |
| data_lba  | u32       | Secteur LBA des données             |
| _pad      | [u8;475]  | Padding pour remplir 512 octets      |

## Fonctions (fs/kfs.rs)

- `init()` — au boot, lit le superbloc. Si magic absent → formate le disque
- `create(name, data)` — trouve un slot libre, écrit l'entrée + les données
- `list(cb)` — itère les entrées utilisées, appelle le callback avec (name, size)
- `read(name, out)` — cherche par nom, lit les données dans `out`, retourne la taille
- `delete(name)` — cherche par nom, écrase le slot avec une entrée vide

## Helpers

- `name_len(name)` — longueur du nom jusqu'au premier `\0`
- `name_eq(a, b)` — compare deux slices octet par octet

## Commandes shell

| Commande                     | Action                              |
|------------------------------|-------------------------------------|
| `kfsLs`                      | Liste les fichiers sur disque       |
| `kfsWrite <nom> <contenu>`   | Crée un fichier sur disque          |
| `kfsRead <nom>`              | Lit et affiche un fichier disque    |
| `kfsDel <nom>`               | Supprime un fichier disque          |

## Limites actuelles

- Max 8 fichiers
- Max 512 octets par fichier (1 secteur)
- Pas de sous-dossiers

## Fichiers modifiés

- `src/fs/kfs.rs` — nouveau, tout le filesystem + helpers `name_len`, `name_eq`
- `src/fs/mod.rs` — `pub mod kfs`
- `src/shell.rs` — commandes kfsLs, kfsWrite, kfsRead, kfsDel + help (concat!)
- `src/main.rs` — `fs::kfs::init()` au boot + `bind_writer_to_win` auto
- `src/io/framebuffer.rs` — `bind_writer_to_win`, `TEXT_MAX_COLS/ROWS` constantes, text_buf agrandi
