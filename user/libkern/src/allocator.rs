/*
    ══════════════════════════════════════════════════════════
    BUMP ALLOCATOR — Heap userspace (16 Ko statiques)
    ══════════════════════════════════════════════════════════

    POURQUOI UN ALLOCATEUR USERSPACE ?
    ───────────────────────────────────
    extern crate alloc (Vec, String, Box...) exige qu'un #[global_allocator]
    soit défini. Le kernel en a un (FreeListAllocator), mais il tourne en
    ring 0 et n'est pas accessible depuis ring 3.
    Libkern fournit son propre allocateur, embarqué dans le BSS de l'ELF.

    FONCTIONNEMENT DU BUMP ALLOCATOR
    ─────────────────────────────────
    On maintient un pointeur `next` qui avance à chaque allocation.
    L'alignement est respecté en rongeant next vers le haut.

    ┌────────────────────────────────────────────────┐
    │ HEAP [0 .. 16 Ko]                              │
    │ ████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
    │             ↑ next (avance, ne recule jamais)  │
    └────────────────────────────────────────────────┘

    LIMITATIONS
    ───────────
    - dealloc() est un no-op : la mémoire libérée n'est pas recyclée
    - La heap entière est récupérée quand le processus se termine (exit)
    - Pas de fragmentation, pas d'overhead par bloc
    - 16 Ko suffisent pour des programmes simples (Vec, String courts)
*/

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Taille de la heap userspace - 16 ko dans le BSS de l'ELF.
const HEAP_SIZE: usize = 16 * 1024;

// aligne(16) : garantit que HEAP.data est aligné sur 16 octets,
// ce que certains type (SSE, u128...) peuvent exiger.
#[repr(align(16))]
struct BumpHeap {
    data: UnsafeCell<[u8; HEAP_SIZE]>,
    /// Offset du prochain octet libre (depuis le début de data).
    next: AtomicUsize,
}

// BumpHeap est partagée en lecture par plusieur alloc() potentiels,
// mais AtomicUsize garentit la cohérence dans lock.
unsafe impl Sync for BumpHeap {}

static HEAP: BumpHeap = BumpHeap {
    data: UnsafeCell::new([0; HEAP_SIZE]),
    next: AtomicUsize::new(0),
};

pub struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {

    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {

        let align   = layout.align();
        let size    = layout.size();
        let base    = HEAP.data.get() as usize;

        // Cas loop : aligne next puis avance atomiquement de size.
        // compare_exchange_weak est plus rapide que _strong sur x86_64
        // (l'échec spurieux est réessayé immédiatement).
        let mut old = HEAP.next.load(Ordering::Relaxed);

        loop {

            let aligned = (old + align - 1) & !(align - 1);
            let new     = aligned + size;

            if new > HEAP_SIZE { return core::ptr::null_mut(); }

            match HEAP.next.compare_exchange_weak(
                old, new, Ordering::Relaxed, Ordering::Relaxed,
            ) {
                Ok(_)   => return (base + aligned) as *mut u8,
                Err(e)  => old = e,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {

        // Bump : on ne libère pas individuellement - no-op volontaire.
        // La heap entière disparait avec le processus (exit -> task_exit).
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

#[alloc_error_handler]
fn alloc_error(_: Layout) -> ! {

    // Plus de mémoire disponible - boucle infinie (même comportement que le kernel).
    loop {}
}