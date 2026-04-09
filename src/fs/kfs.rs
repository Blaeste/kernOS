/// KernOS File System - filesystem persiste sur disque ATA

// Layout disque :
// Secteur 0     = superbloc
// Secteurs 1-8  = table des fichiers (1 entrée par secteur)
// Secteurs 9+   = données (1 fichier = 1 secteur = 212 octets max)

const KFS_MAGIC:         u32 = 0x4B465300;   // "KFS\0"
const MAX_FILES:         usize = 8;
const DATA_START_LBA:    u32 = 9;
const NAME_MAX:          usize = 28;         // octets

/// Superbloc - secteur 0
#[repr(C)]
pub struct SuperBlock {
    pub magic:      u32,
    pub version:    u32,
    pub file_count: u32,
    pub _pad:       [u8; 500],              // remplit 512 octets
}

/// Entrée de fichier - 1 par secteur (secteur 1-8)
#[repr(C)]
pub struct FileEntry {
    pub used:       u8,
    pub name:       [u8; NAME_MAX],
    pub size:       u32,
    pub data_lba:   u32,                    // secteur où sont les données
    pub _pad:       [u8; 475],              // remplit 512 octets
}

use crate::drivers::ata::{read_sector, write_sector};

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

/// Lit une entrée de fichier (idx = 0..7, secteur 1..8)
fn read_entry(idx: usize) -> FileEntry {
    let mut buf = [0u16; 256];
    read_sector(1 + idx as u32, &mut buf);
    unsafe { core::ptr::read(buf.as_ptr() as *const FileEntry) }
}

/// Ecrit une entrée de fichier (idx = 0..7, secteur 1..8)
fn write_entry(idx: usize, entry: &FileEntry) {
    let mut buf = [0u16; 256];
    unsafe { core::ptr::write(buf.as_mut_ptr() as *mut FileEntry, core::ptr::read(entry)); }
    write_sector(1 + idx as u32, &buf);
}

/// Initialise le filesystem (à appeler au boot)
/// Si le magic est absent -> formate le disque
pub fn init() {
    let sb = read_superblock();
    if sb.magic != KFS_MAGIC {
        // Premier boot : formate
        let new_sb = SuperBlock {
            magic: KFS_MAGIC,
            version: 1,
            file_count: 0,
            _pad: [0; 500],
        };
        write_superblock(&new_sb);
        // Efface les 8 entrées
        let mut i = 0;
        while i < MAX_FILES {
            let entry = FileEntry {
                used: 0,
                name: [0; NAME_MAX],
                size: 0,
                data_lba: 0,
                _pad: [0; 475],
            };
            write_entry(i, &entry);
            i += 1;
        }
    }
}

/// Crée un fichier sur le disque
pub fn create(name: &[u8], data: &[u8]) -> bool {
    let mut i = 0;
    while i < MAX_FILES {
        let entry = read_entry(i);
        if entry.used == 0 {
            // Slot libre trouvé
            let mut new_entry = FileEntry {
                used: 1,
                name: [0; NAME_MAX],
                size: data.len() as u32,
                data_lba: DATA_START_LBA + i as u32,
                _pad: [0; 475],
            };
            // Copie le nom
            let len = if name.len() < NAME_MAX { name.len() } else { NAME_MAX };
            let mut j = 0;
            while j < len { new_entry.name[j] = name[j]; j += 1; }
            write_entry(i, &new_entry);
            // Ecrit les données
            let mut buf = [0u16; 256];
            let bytes = unsafe {
                core::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, 512)
            };
            let dlen = if data.len() < 512 { data.len() } else { 512 };
            let mut k = 0;
            while k < dlen { bytes[k] = data[k]; k += 1; }
            write_sector(DATA_START_LBA + i as u32, &buf);
            return true;
        }
        i += 1;
    }
    false
}

/// Liste les fichiers
pub fn list(mut cb: impl FnMut(&[u8], u32)) {
    let mut i = 0;
    while i < MAX_FILES {
        let entry = read_entry(i);
        if entry.used == 1 {
            let len = {
                let mut l = 0;
                while l < NAME_MAX && entry.name[l] != 0 { l += 1; }
                l
            };
            cb(&entry.name[..len], entry.size);
        }
        i += 1;
    }
}

/// Lit le contenu d'un fichier
pub fn read(name: &[u8], out: &mut [u8; 512]) -> usize {
    let mut i = 0;
    while i < MAX_FILES {
        let entry = read_entry(i);
        if entry.used == 1 {
            let len = {
                let mut l = 0;
                while l < NAME_MAX && entry.name[l] != 0 { l += 1; }
                l
            };
            if len == name.len() {
                let mut eq = true;
                let mut j = 0;
                while j < len { if entry.name[j] != name[j] { eq = false; } j += 1; }
                if eq {
                    let mut buf = [0u16; 256];
                    read_sector(entry.data_lba, &mut buf);
                    let bytes = unsafe {
                        core::slice::from_raw_parts(buf.as_ptr() as *const u8, 512)
                    };
                    let sz = entry.size as usize;
                    let mut k = 0;
                    while k < sz && k < 512 { out[k] = bytes[k]; k += 1; }
                    return sz;
                }
            }
        }
        i += 1;
    }
    0
}