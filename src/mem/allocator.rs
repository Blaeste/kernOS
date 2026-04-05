use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

// 64 Ko de heap
const HEAP_SIZE: usize = 64 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static mut OFFSET: usize = 0;

pub struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let offset = unsafe { &mut *core::ptr::addr_of_mut!(OFFSET) };

        // ALigner l'offset
        let align = layout.align();
        let aligned = (*offset + align - 1) & !(align - 1);
        let end = aligned + layout.size();

        if end > HEAP_SIZE {
            return ptr::null_mut(); // plus de memoire
        }

        *offset = end;
        unsafe { core::ptr::addr_of_mut!(HEAP).cast::<u8>().add(aligned) }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // bump allocator ne libère jamais (pout l'instant)
    }
}