# 24 — KFS v2 : KernOS File System

## Objectif

Filesystem persisté sur disque ATA. Fichiers de taille variable, 64 entrées max, 64 Ko par fichier.

## Architecture disque (v2)

```
Secteur 0        → SuperBlock (magic=0x4B465300, version=2)
Secteurs 1-4     → Table fichiers : 64 entrées × 32 octets (16 par secteur)
Secteur 5        → FAT : 128 u32 (liste chaînée de blocs)
Secteurs 6-133   → Blocs de données : 128 × 512 octets = 64 Ko total
```

## FAT (File Allocation Table)

Chaque entrée FAT[i] est un u32 :
- `0x00000000` = bloc libre
- `0xFFFFFFFF` = fin de chaîne (dernier bloc du fichier)
- `n` = index du bloc suivant dans la chaîne

Exemple : fichier 1200 octets → 3 blocs → `FAT[2]=7, FAT[7]=11, FAT[11]=0xFFFFFFFF`

## Structures

### SuperBlock (512 octets, secteur 0)
| Champ   | Type     | Description             |
|---------|----------|-------------------------|
| magic   | u32      | `0x4B465300` = "KFS\0"  |
| version | u32      | Version = 2             |
| _pad    | [u8;504] | Padding 512 octets      |

### FileEntry (32 octets, 16 par secteur)
| Champ | Type      | Description                              |
|-------|-----------|------------------------------------------|
| used  | u8        | 0=libre, 1=utilisé                       |
| name  | [u8;23]   | Nom du fichier                           |
| size  | u32       | Taille en octets                         |
| first | u32       | Index du premier bloc FAT (0xFFFF=vide)  |

## Fonctions (fs/kfs.rs)

- `init()` — formate si magic/version invalide (migration v1→v2 automatique)
- `create(name, data)` — alloue les blocs nécessaires, chaîne la FAT, écrit
- `read(name, out)` — lit depuis le début du fichier dans `out`
- `read_at(name, offset, out)` — lit depuis `offset` (utilisé par `sys_read` + `fd.pos`)
- `list(cb)` — callback (name, size) pour chaque fichier
- `delete(name)` — libère la chaîne FAT + efface l'entrée

## Commandes shell

| Commande                     | Action                              |
|------------------------------|-------------------------------------|
| `kfsLs`                      | Liste les fichiers sur disque       |
| `kfsWrite <nom> <contenu>`   | Crée un fichier sur disque          |
| `kfsRead <nom>`              | Lit et affiche un fichier disque    |
| `kfsDel <nom>`               | Supprime un fichier disque          |

## Limites actuelles

- Max 64 fichiers
- Max 64 Ko par fichier (128 blocs × 512 octets)
- Pas de sous-dossiers
- Pas de cache secteur (chaque accès = lecture ATA)

## Fichiers

- `src/fs/kfs.rs` — réécriture complète v2 (FAT, read_at, create multi-blocs)
