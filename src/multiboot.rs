/// Parse la structure Multiboot2 info passée par GRUB.
/// GRUB stocke en mémoire une liste de "tags" décrivant le hardware :
/// memory map, framebuffer, boot loader name, etc.
/// On cherche le tag type 6 = memory map (liste des zones RAM).

/// Tag générique — header commun à tous les tags Multiboot2.
/// Chaque tag commence par son type (u32) et sa taille (u32).
#[repr(C)]
struct Tag {
    tag_type:       u32,                          // identifiant du tag (6 = memory map, 0 = fin)
    size:           u32,                          // taille totale du tag en octets (header inclus)
}

/// Tag spécifique à la memory map (type 6).
/// Contient une liste d'entrées décrivant chaque zone mémoire.
#[repr(C)]
struct MemoryMapTag {
    tag_type:       u32,                          // toujours 6
    size:           u32,                          // taille totale du tag
    entry_size:     u32,                          // taille d'une entrée (24 octets en général)
    entry_version:  u32,                          // version du format (0)
    // Suivi par N entrées MemoryEntry
}

/// Une zone de mémoire physique détectée par GRUB.
#[repr(C)]
pub struct MemoryEntry {
    pub base_addr:  u64,                          // adresse physique de début de la zone
    pub length:     u64,                          // taille de la zone en octets
    pub entry_type: u32,                          // 1 = RAM libre, 2 = réservé, 3 = ACPI, etc.
    _reserved:      u32,                          // padding, toujours 0
}

impl MemoryEntry {
    /// Type 1 = RAM utilisable par le kernel
    pub fn is_available(&self) -> bool {
        self.entry_type == 1
    }
}

const TAG_MEMORY_MAP: u32 = 6;                    // identifiant du tag memory map
const TAG_END: u32 = 0;                           // identifiant du tag de fin

/// Parcourt les tags Multiboot2 pour trouver la memory map.
/// Pour chaque zone mémoire trouvée, appelle `callback(entry)`.
///
/// Structure en mémoire :
/// ```
/// [taille_totale: u32][reserved: u32]  ← 8 octets header
/// [tag1][tag2]...[tag_end]             ← tags alignés sur 8 octets
/// ```
pub fn parse_memory_map(multiboot_addr: u64, mut callback: impl FnMut(&MemoryEntry)) {
    unsafe {
        let base = multiboot_addr as *const u8;
        let mut offset: usize = 8; // saute le header (taille + reserved)

        loop {
            // Chaque tag est aligné sur 8 octets
            offset = (offset + 7) & !7;

            let tag = &*(base.add(offset) as *const Tag);

            if tag.tag_type == TAG_END {
                break; // fin de la liste des tags
            }

            if tag.tag_type == TAG_MEMORY_MAP {
                let mmap = &*(base.add(offset) as *const MemoryMapTag);
                let entry_size = mmap.entry_size as usize;
                let entries_start = offset + 16; // après les 4 champs du header
                let entries_end = offset + mmap.size as usize;

                // Parcourt chaque entrée de la memory map
                let mut entry_offset = entries_start;
                while entry_offset < entries_end {
                    let entry = &*(base.add(entry_offset) as *const MemoryEntry);
                    callback(entry);
                    entry_offset += entry_size;
                }
            }

            offset += tag.size as usize; // passe au tag suivant
        }
    }
}