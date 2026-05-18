use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

const HEAP_SIZE: usize = 256 * 1024;

// align(16) garantit que FreeBlock (size: usize + next: *mut) est correctement aligné
#[repr(align(16))]
struct Heap([u8; HEAP_SIZE]);
static mut HEAP: Heap = Heap([0; HEAP_SIZE]);

#[repr(C)]
struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

const MIN_BLOCK: usize = core::mem::size_of::<FreeBlock>();

static mut FREE_HEAD: *mut FreeBlock = ptr::null_mut();

pub fn init() {
    unsafe {
        let head = core::ptr::addr_of_mut!(HEAP.0) as *mut FreeBlock;
        (*head).size = HEAP_SIZE;
        (*head).next = ptr::null_mut();
        FREE_HEAD = head;
    }
}

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

    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let align = layout.align().max(MIN_BLOCK);
            let size = (layout.size() + MIN_BLOCK + align - 1) & !(align - 1);
            let size = size.max(MIN_BLOCK * 2);

            let mut prev: *mut FreeBlock = ptr::null_mut();
            let mut cur = FREE_HEAD;

            while !cur.is_null() {
                if (*cur).size >= size {
                    if (*cur).size >= size + MIN_BLOCK * 2 {
                        // Découpe : crée un bloc résiduel après la zone allouée
                        let rest = (cur as *mut u8).add(size) as *mut FreeBlock;
                        (*rest).size = (*cur).size - size;
                        (*rest).next = (*cur).next;
                        (*cur).size = size;
                        if prev.is_null() {
                            FREE_HEAD = rest;
                        } else {
                            (*prev).next = rest;
                        }
                    } else {
                        // Pas de découpe — retire le bloc entier de la free list
                        if prev.is_null() {
                            FREE_HEAD = (*cur).next;
                        } else {
                            (*prev).next = (*cur).next;
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

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe {
            if ptr.is_null() { return; }
            let block = ptr.sub(MIN_BLOCK) as *mut FreeBlock;

            // Insertion triée par adresse croissante (requis pour la coalescence)
            let mut prev: *mut FreeBlock = ptr::null_mut();
            let mut cur = FREE_HEAD;
            while !cur.is_null() && (cur as usize) < (block as usize) {
                prev = cur;
                cur = (*cur).next;
            }
            (*block).next = cur;
            if prev.is_null() {
                FREE_HEAD = block;
            } else {
                (*prev).next = block;
            }

            // Fusionne avec le bloc suivant si adjacent
            if !(*block).next.is_null() {
                let end = (block as *mut u8).add((*block).size);
                if end == (*block).next as *mut u8 {
                    (*block).size += (*(*block).next).size;
                    (*block).next = (*(*block).next).next;
                }
            }
            // Fusionne avec le bloc précédent si adjacent
            if !prev.is_null() {
                let end = (prev as *mut u8).add((*prev).size);
                if end == block as *mut u8 {
                    (*prev).size += (*block).size;
                    (*prev).next = (*block).next;
                }
            }
        }
    }
}
