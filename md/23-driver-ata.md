# 23 — Driver ATA (lecture/écriture disque)

## Concept

ATA (Advanced Technology Attachment) est le protocole standard pour communiquer
avec les disques durs via le bus IDE. Le CPU dialogue avec le disque via des ports I/O.

```
CPU ──── ports I/O (0x1F0-0x1F7) ──── contrôleur ATA ──── disque
```

## Ports ATA Primary Bus

| Port  | Nom            | Usage                          |
|-------|----------------|--------------------------------|
| 0x1F0 | DATA           | Lecture/écriture 16 bits       |
| 0x1F2 | SECTOR_COUNT   | Nombre de secteurs à lire/écrire |
| 0x1F3 | LBA_LOW        | Bits 0-7 de l'adresse LBA      |
| 0x1F4 | LBA_MID        | Bits 8-15                      |
| 0x1F5 | LBA_HIGH       | Bits 16-23                     |
| 0x1F6 | DRIVE          | Sélection drive + bits 24-27   |
| 0x1F7 | STATUS/COMMAND | Lire statut / envoyer commande |

## Mode LBA28

LBA (Logical Block Addressing) = adresse le disque comme un tableau de secteurs.
Chaque secteur = 512 octets = 256 mots de 16 bits.

```
Secteur 0 = premiers 512 octets du disque
Secteur 1 = octets 512-1023
...
```

## Protocole read/write

```
1. wait_ready()     → attendre BSY=0 (drive pas occupé)
2. outb(DRIVE)      → sélectionner master + bits 24-27 de LBA
3. outb(SECTOR_COUNT, 1)
4. outb(LBA_LOW/MID/HIGH)
5. outb(COMMAND, 0x20=READ / 0x30=WRITE)
6. wait_drq()       → attendre DRQ=1 (prêt à transférer)
7. 256× inw/outw    → lire/écrire les 256 mots
```

## Résultat du test

```
> ata-write   → écrit "KernOS" dans le secteur 0
> ata-read    → relit et affiche :
  [00] 4b 65  ('K' 'e')
  [02] 72 6e  ('r' 'n')
  [04] 4f 53  ('O' 'S')
```

## Disque virtuel QEMU

`run.ps1` / `run.sh` créent automatiquement `target/disk.img` (10 Mo, format raw)
si le fichier n'existe pas, et le passent à QEMU via `-drive`.

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/drivers/ata.rs` | Nouveau : ports, helpers I/O, `read_sector`, `write_sector` |
| `src/drivers/mod.rs` | `pub mod ata` |
| `src/shell.rs` | Commandes `ata-read` et `ata-write` |
| `run.ps1` | Auto-création `disk.img` + `-drive` QEMU |
| `run.sh` | Idem pour Linux |
