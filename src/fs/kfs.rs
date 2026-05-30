/*
    ══════════════════════════════════════════════════════════
    KFS v2 — KernOS File System, version 2
    ══════════════════════════════════════════════════════════

    LIMITES DE KFS v1
    ─────────────────
    v1 stockait chaque fichier dans UN secteur fixe (512 octets max, 8 fichiers).
    v2 introduit une liste chaînée de blocs (FAT) pour des fichiers de taille
    variable, et une table de 64 entrées pour plus de fichiers simultanés.

    LAYOUT DISQUE
    ─────────────
    ┌─────────────┬────────────────────────────────────────────┐
    │ Secteur 0   │ Superbloc (magic, version)                 │
    │ Secteurs 1-4│ Table fichiers : 64 entrées × 32 octets    │
    │             │   16 entrées par secteur → 4 secteurs      │
    │ Secteur 5   │ FAT : 128 u32 (liste chaînée de blocs)     │
    │ Secteurs 6+ │ Blocs de données : 128 × 512 = 64 Ko       │
    └─────────────┴────────────────────────────────────────────┘

    LISTE CHAÎNÉE (FAT)
    ───────────────────
    FAT[i] == FAT_FREE (0)       → bloc i libre
    FAT[i] == FAT_END  (0xFFF…)  → bloc i = dernier de la chaîne
    FAT[i] == j                  → bloc i suivant = bloc j

    Exemple : fichier de 1200 octets → 3 blocs
      FileEntry.first = 2
      FAT[2] = 7  → FAT[7] = 11  → FAT[11] = FAT_END

    ENTRÉE FICHIER (32 octets)
    ──────────────────────────
      used  : 1 octet  — 0=libre, 1=utilisé
      name  : 23 octets — nom (sans null terminal obligatoire)
      size  : 4 octets  — taille en octets
      first : 4 octets  — index du premier bloc (FAT_END si vide)
*/

use crate::drivers::ata::{read_sector, write_sector};
use crate::klib::string::{eq, strlen};
use crate::klib::mem::memcpy;
use crate::klib::cmp::min;

// ── Constantes ───────────────────────────────────────────────────────────

const KFS_MAGIC:            u32     = 0x4B465300;       // "KFS\0"
const KFS_VERSION:          u32     = 2;
pub const MAX_FILES:        usize   = 64;
const MAX_BLOCKS:           usize   = 128;
const FAT_SECTOR:           u32     = 5;
const DATA_START:           u32     = 6;
const FAT_FREE:             u32     = 0x0000_0000;
const FAT_END:              u32     = 0xFFFF_FFFF;
pub const NAME_MAX:         usize   = 23;
const ENTRY_SIZE:           usize   = 32;
const ENTRIES_PER_SECTOR:   usize   = 512 / ENTRY_SIZE; // 16


// ── Structures ───────────────────────────────────────────────────────────

/// Superbloc - secteur 0
#[repr(C)]
struct SuperBlock {
    magic:      u32,
    version:    u32,
    _pad:       [u8; 504],
}

/// Entrée de fichier - 32 octets exactement (16 par secteur)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FileEntry {
    pub used:       u8,
    pub name:       [u8; NAME_MAX],
    pub size:       u32,
    pub first:      u32,                // index du premier bloc dans la FAT (FAT_END = pas de données)
}


// ── I/O bas niveau ───────────────────────────────────────────────────────

/// Lit le superbloc depuis le secteur 0
fn read_superblock() -> SuperBlock {
    let mut buf = [0u16; 256];
    read_sector(0, &mut buf);
    unsafe { core::ptr::read(buf.as_ptr() as *const SuperBlock) }
}

/// Ecrit le superbloc dans le secteur 0
fn write_superblock(sb: &SuperBlock) {
    let mut buf = [0u16; 256];
    unsafe { core::ptr::write(buf.as_mut_ptr() as *mut SuperBlock, core::ptr::read(sb)); }
    write_sector(0, &buf);
}

/// Lit la FAT entière (1 secteur = 128 u32).
fn read_fat() -> [u32; MAX_BLOCKS] {

    let mut buf = [0u16; 256];

    read_sector(FAT_SECTOR, &mut buf);

    let bytes = unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u8, 512) };
    let mut fat = [0u32; MAX_BLOCKS];

    for i in 0..MAX_BLOCKS {

        fat[i] = u32::from_le_bytes([
            bytes[i*4], bytes[i*4+1], bytes[i*4+2], bytes[i*4+3],
        ]);
    }
    fat
}

/// Ecrit la FAT entière.
fn write_fat(fat: &[u32; MAX_BLOCKS]) {

    let mut buf = [0u16; 256];
    let bytes = unsafe { core::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, 512) };

    for i in 0..MAX_BLOCKS {

        let b = fat[i].to_le_bytes();

        bytes[i*4] = b[0];
        bytes[i*4+1] = b[1];
        bytes[i*4+2] = b[2];
        bytes[i*4+3] = b[3];
    }

    write_sector(FAT_SECTOR, &buf);
}

/// Lit une entrée fichier (idx 0..63).
/// Les entrées sont packées 16 par secteur dans les secteurs 1-4.
fn read_entry(idx: usize) -> FileEntry {

    let sector = 1 + (idx / ENTRIES_PER_SECTOR) as u32;
    let offset = idx % ENTRIES_PER_SECTOR;
    let mut buf = [0u16; 256];
    
    read_sector(sector, &mut buf);

    let bytes = unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u8, 512) };

    unsafe { core::ptr::read(bytes.as_ptr().add(offset * ENTRY_SIZE) as *const FileEntry) }
}

/// Ecrit une entrée fichier (idx 0..63).
/// Lit d'abord le secteur entier pour ne pas écraser les autres entrées.
fn write_entry(idx: usize, entry: &FileEntry) {

    let sector = 1 + (idx / ENTRIES_PER_SECTOR) as u32;
    let offset = idx % ENTRIES_PER_SECTOR;
    let mut buf = [0u16; 256];

    read_sector(sector, &mut buf);

    let bytes = unsafe { core::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, 512) };

    unsafe { core::ptr::write(bytes.as_mut_ptr().add(offset * ENTRY_SIZE) as *mut FileEntry, *entry); }

    write_sector(sector, &buf);
}

/// Lit un bloc de données (block_idx 0..127) dans out.
fn read_block(block_idx: u32, out: &mut [u8; 512]) {

    let mut buf = [0u16; 256];

    read_sector(DATA_START + block_idx, &mut buf);

    let bytes = unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u8, 512) };
    
    out.copy_from_slice(bytes);
}

/// Ecrit un bloc de données.
fn write_block(block_idx: u32, data: &[u8]) {

    let mut buf = [0u16; 256];
    let bytes = unsafe { core::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, 512) };
    let len = min(data.len(), 512);

    memcpy(bytes, data, len);

    write_sector(DATA_START + block_idx, &buf);
}


// ── API publique ─────────────────────────────────────────────────────────

/// Initialise KFS au boot. Formate si le magic ou la version est invalide.
pub fn init() {
    let sb = read_superblock();
    if sb.magic != KFS_MAGIC || sb.version != KFS_VERSION {

        write_superblock(&SuperBlock { magic: KFS_MAGIC, version: KFS_VERSION, _pad: [0; 504] });

        // FAT : tous les bloc libres
        write_fat(&[FAT_FREE; MAX_BLOCKS]);

        // Table des fichiers : toutes les entrées vides
        for i in 0..MAX_FILES {

            write_entry(i, &FileEntry { used: 0, name: [0; NAME_MAX], size: 0, first: FAT_END });
        }
    }
}

/// Crée ou écrase un fichier. Retourne false si plus de slots ou de blocs.
pub fn create(name: &[u8], data: &[u8]) -> bool {

    // Cherche un slot libre (ou le slot existant du même nom pour l'écraser)
    let mut slot = MAX_FILES;

    for i in 0..MAX_FILES {

        let e = read_entry(i);

        if e.used == 0 && slot == MAX_FILES { slot = i; }

        if e.used == 1 {

            let nlen = strlen(&e.name);

            if eq(&e.name[..nlen], name) {

                // Fichier existant: libère ses blocs avant d'écraser
                let mut fat = read_fat();
                let mut blk = e.first;

                while blk != FAT_END && blk < MAX_BLOCKS as u32 {

                    let next = fat[blk as usize];
                    
                    fat[blk as usize] = FAT_FREE;
                    blk = next;
                }

                write_fat(&fat);
                slot = i;
                break;
            }
        }
    }

    if slot == MAX_FILES { return false; }

    // Alloue les blocs nécessaires dans la FAT
    let mut fat = read_fat();
    let blocks_needed = (data.len() + 511) / 512;
    let mut chain: [u32; MAX_BLOCKS] = [FAT_END; MAX_BLOCKS];
    let mut found = 0usize;
    let mut bi = 0usize;

    while bi < MAX_BLOCKS && found < blocks_needed {

        if fat[bi] == FAT_FREE {

            chain[found] = bi as u32;
            found += 1;
        }
        bi += 1;
    }

    if found < blocks_needed { return false; }

    // Chaine les blocs dans la FAT
    for i in 0..found {

        fat[chain[i] as usize] = if i + 1 < found { chain[i + 1] } else { FAT_END };
    }
    write_fat(&fat);

    // Ecrit les données bloc par bloc
    for i in 0..found {

        let offset = i * 512;
        let slice = if offset < data.len() { &data[offset..min(data.len(), offset + 512)] } else { &[] };

        write_block(chain[i], slice);
    } 

    // Ecrit l'entrée fichier
    let mut entry = FileEntry { used: 1, name: [0; NAME_MAX], size: data.len() as u32, first: if found > 0 { chain[0] } else { FAT_END } };
    let nlen = min(name.len(), NAME_MAX);

    memcpy(&mut entry.name, name, nlen);
    write_entry(slot, &entry);
    true
}

/// Lit jusqu'a out.len() octets depuis le début du fichier.
/// Retourne le nombre d'octets copiés (0 = introuvable)
pub fn read(name: &[u8], out: &mut [u8]) -> usize {
    read_at(name, 0, out)
}   

/// lit jusqu'a out.len() octets à aprtir de l'offset donnée.
/// Retourne le nombre d'octets copiés (0 = introuvable ou hors du fichier)
pub fn read_at(name: &[u8], offset: usize, out: &mut [u8]) -> usize {

    for i in 0..MAX_FILES {

        let e  = read_entry(i);
        if e.used != 1 { continue; }

        let nlen = strlen(&e.name);
        if !eq(&e.name[..nlen], name) { continue; }

        let file_size = e.size as usize;
        if offset >= file_size { return 0; }

        let to_read = min(out.len(), file_size - offset);
        let fat = read_fat();

        // Avance dans la chaine jusqu'au bloc contenant offset
        let start_block = offset / 512;
        let start_off   = offset % 512;
        let mut blk = e.first;

        for _ in 0..start_block {

            if blk == FAT_END || blk >= MAX_BLOCKS as u32 { return 0; }

            blk = fat[blk as usize];
        }

        // Copie les données bloc par bloc
        let mut copied = 0usize;
        let mut in_block_off = start_off;

        while copied < to_read && blk != FAT_END && blk < MAX_BLOCKS as u32 {

            let mut block = [0u8; 512];
            read_block(blk, &mut block);

            let available = min(512 - in_block_off, to_read - copied);
            memcpy(&mut out[copied..], &block[in_block_off..], available);
            copied += available;
            in_block_off = 0;
            blk = fat[blk as usize];
        }
        return copied;
    }
    0
}

/// Liste les fichiers (nom, taille) via callback.
pub fn list(mut cb: impl FnMut(&[u8], u32)) {
    
    for i in 0..MAX_FILES {

        let e = read_entry(i);
        if e.used == 1 {

            let nlen = strlen(&e.name);
            cb(&e.name[..nlen], e.size);
        }
    }
}

/// Supprime un fichier et libère ses blocs dans la FAT.
/// Retoune true si trouve et supprime, false si introuvable
pub fn delete(name: &[u8]) -> bool {
    
    for i in 0..MAX_FILES {

        let e = read_entry(i);
        if e.used != 1 { continue; }

        let nlen = strlen(&e.name);
        if !eq(&e.name[..nlen], name) { continue; }

        // Libère la chaine de blocs
        let mut fat = read_fat();
        let mut blk = e.first;

        while blk != FAT_END && blk < MAX_BLOCKS as u32 {

            let next = fat[blk as usize];
            fat[blk as usize] = FAT_FREE;
            blk = next;
        }
        write_fat(&fat);

        // Efface l'entée
        write_entry(i, &FileEntry { used: 0, name: [0; NAME_MAX], size:0, first: FAT_END });
        return true;
    }
    false
}
