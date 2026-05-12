use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

/// Taille du heap : 256 Ko
const HEAP_SIZE: usize = 256 * 1024;

// align(16) : le compilateur peut générer MOVAPS/SSE pour copier FreeBlock (16 octets).
// Sans alignement, #GP immédiat sur Linux si HEAP atterrit à une adresse impaire.
#[repr(align(16))]
struct Heap([u8; HEAP_SIZE]);
static mut HEAP: Heap = Heap([0; HEAP_SIZE]);

/// En-tête d'un bloc libre (liste chainée)
#[repr(C)]
struct FreeBlock {
    size: usize,                    // taille du bloc (header inclus)
    next: *mut FreeBlock,           // pointeur vers le prochain bloc libre
}

const MIN_BLOCK: usize = core::mem::size_of::<FreeBlock>();

/// Tête de liste libre
static mut FREE_HEAD: *mut FreeBlock = ptr::null_mut();

/// Initialise le heap : un seul gros bloc libre
pub fn init() {
    unsafe {
        let head = core::ptr::addr_of_mut!(HEAP.0) as *mut FreeBlock;
        (*head).size = HEAP_SIZE;
        (*head).next = ptr::null_mut();
        FREE_HEAD = head;
    }
}

/// Compte les blocs libres et la mémoire disponible
pub fn heap_info() -> (usize, usize, usize) {
    unsafe {
        let mut free = 0usize;
        let mut count = 0usize;
        let mut cur = FREE_HEAD;
        while !cur.is_null() {
            free += (*cur).size;
            count += 1;
            cur = (*cur).next;
        }
        (HEAP_SIZE, free, count)
    }
}

pub struct FreeListAllocator;

unsafe impl GlobalAlloc for FreeListAllocator {

    /// 
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let align = layout.align().max(MIN_BLOCK);
            let size = (layout.size() + MIN_BLOCK + align - 1) & !(align - 1);
            let size = if size < MIN_BLOCK * 2 { MIN_BLOCK * 2 } else { size };
            
            // Cherche un bloc assez grand (first-fit)
            let mut prev: *mut FreeBlock = ptr::null_mut();
            let mut cur = FREE_HEAD;

            while !cur.is_null() {
                if (*cur).size >= size {
                    // Découpe si le reste est assez grand
                    if (*cur).size >= size + MIN_BLOCK * 2 {
                        let rest = (cur as *mut u8).add(size) as *mut FreeBlock;
                        (*rest).size = (*cur).size - size;
                        (*rest).next = (*cur).next;
                        (*cur).size = size;
                        // Relie la liste
                        if prev.is_null() {
                            FREE_HEAD = rest;
                        } else {
                            (*prev).next = rest;
                        }
                    }
                    return (cur as *mut u8).add(MIN_BLOCK);
                }
                prev = cur;
                cur = (*cur).next;
            }
            ptr::null_mut()
        }
    }

    ///
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe {
            if ptr.is_null() { return; }
            // Recule pour retrouver le header
            let block = ptr.sub(MIN_BLOCK) as *mut FreeBlock;

            // Insère en tête de liste
            (*block).next = FREE_HEAD;
            FREE_HEAD = block;

            // Fusionne les blocs adjacents
            merge_free_blocks();
        }
    }
}

/// Fusionne les blocs libres adjacents en mémoire
unsafe fn merge_free_blocks() {
    unsafe {
        // Tri insertion par adresse (la liste est courte)
        sort_free_list();

        let mut cur = FREE_HEAD;
        while !cur.is_null() {
            let next = (*cur).next;
            if !next.is_null() {
                let end = (cur as *mut u8).add((*cur).size);
                if end == next as *mut u8 {
                    (*cur).size += (*next).size;
                    (*cur).next = (*next).next;
                    continue; // re-check le même cur
                }
            }
            cur = (*cur).next;
        }
    }
}

/// Tri la free list par adresse croissante
unsafe fn sort_free_list() {
    unsafe {
        if FREE_HEAD.is_null() { return; }

        let mut sorted;
        loop {
            sorted = true;
            let mut cur = FREE_HEAD;
            while !(*cur).next.is_null() {
                let next = (*cur).next;
                if (cur as usize) > (next as usize) {
                    // Swap les données
                    let tmp_size = (*cur).size;
                    let tmp_next_ptr = (*next).next;
                    (*cur).size = (*next).size;
                    (*next).size = tmp_size;
                    (*cur).next = tmp_next_ptr;
                    (*next).next = cur;
                    if cur == FREE_HEAD {
                        FREE_HEAD = next;
                    }
                    sorted = false;
                }
                cur = (*cur).next;
                if cur.is_null() { break; }
            }
            if sorted { break; }
        }
    }
}