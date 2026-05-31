/// Frame Allocator - gère les pages physiques de 4 Ko
/// Bitmap : chaque bit = un frame. 0 = libre, 1 = occupé.

const FRAME_SIZE:   usize = 4096;               // 4 Ko par frame
const MAX_FRAMES:   usize = 32768;              // 128 Mo / 4 Ko
const BITMAP_SIZE:  usize = MAX_FRAMES / 8;     // 4096 octets

static mut BITMAP: [u8; BITMAP_SIZE] = [0xFF; BITMAP_SIZE]; // tout occupé par défaut

/// Marque un frame comme libre
fn set_free(frame: usize) {
    let byte = frame / 8;
    let bit = frame % 8;                            
    unsafe {
        let bm = core::ptr::addr_of_mut!(BITMAP);
        (*bm)[byte] &= !(1 << bit);
    }
}

/// Marque un frame comme occupé
fn set_used(frame: usize) {
    let byte = frame / 8;
    let bit = frame % 8;                            
    unsafe {
        let bm = core::ptr::addr_of_mut!(BITMAP);
        (*bm)[byte] |= 1 << bit;
    }
}

/// Verifie si un frame est libre
fn is_free(frame: usize) -> bool {
    let byte = frame / 8;
    let bit = frame % 8;
    unsafe {
        let bm = core::ptr::addr_of!(BITMAP);
        (*bm)[byte] & (1 << bit) == 0
    }
}

/// Initialise le bitmap à partir de la memory map Multiboot2.
/// Marque les zones type 1 (RAM libre) comme disponibles,
/// puis protège le kernel (0x0 à 0x200000) en le marquant occupé
pub fn init(multiboot_addr: u64) {
    crate::multiboot::parse_memory_map(multiboot_addr, |entry| {
        if entry.is_available() {
            let start_frame = entry.base_addr as usize / FRAME_SIZE;
            let end_frame = (entry.base_addr + entry.length) as usize / FRAME_SIZE;
            for f in start_frame..end_frame {
                if f < MAX_FRAMES {
                    set_free(f);
                }
            }
        }
    });

    // Protège TOUT le kernel : de 0 jusqu'à kernel_end (symbole du linker).
    // Le BSS contient BACK_BUFF (~8 Mo), TASKS, le heap...
    // Sans ça, frame::alloc() rendrait des frames physiques situés DANS le BSS
    // du kernel -> corruption des page tables / structures -> triple fault.
    unsafe extern "C" {
        static kernel_end: u8;
    }
    
    let kend = core::ptr::addr_of!(kernel_end) as usize;
    let end_frame = (kend + FRAME_SIZE - 1) / FRAME_SIZE;

    for f in 0..end_frame {

        if f < MAX_FRAMES {

            set_used(f);
        }
    }
}

/// Alloue un frame libre. Retourne l'adresse physique ou None.
pub fn alloc() -> Option<u64> {
    for f in 0..MAX_FRAMES {
        if is_free(f) {
            set_used(f);
            return Some((f * FRAME_SIZE) as u64);
        }
    }
    None
}

/// Libère un frame (le rend disponible)
pub fn free(addr: u64) {
    let f = addr as usize / FRAME_SIZE;
    if f < MAX_FRAMES {
        set_free(f);
    }
}

/// COmpte les frames libres.
pub fn free_count() -> usize {
    let mut count = 0;
    for f in 0..MAX_FRAMES {
        if is_free(f) { count += 1; }
    }
    count
}