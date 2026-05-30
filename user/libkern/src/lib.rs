/*
    ══════════════════════════════════════════════════════════
    LIBKERN — Bibliothèque userspace pour KernOS
    ══════════════════════════════════════════════════════════

    POURQUOI LIBKERN ?
    ──────────────────
    Sans libkern, chaque programme userspace réécrit ses propres
    wrappers syscall (write, open, read...) et son propre allocateur.
    Libkern centralise tout ça dans une crate no_std partagée.

    CE QUE LIBKERN FOURNIT
    ──────────────────────
    syscall.rs  → wrappers typés autour de int 0x80
                  write, exit, getpid, yield_cpu, open, close, read
    allocator.rs → bump allocator statique (16 Ko dans le BSS user)
                   active extern crate alloc → Vec, String, Box
                   disponibles dans tous les programmes userspace

    COMMENT L'UTILISER
    ──────────────────
    Dans Cargo.toml du programme :
      [dependencies]
      libkern = { path = "../libkern" }

    Dans main.rs :
      extern crate alloc;
      use libkern::*;      // write, open, Vec, ...
*/

#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

pub mod syscall;
pub mod allocator;

// Re-exporte les wrapper sycall + les types les plus utiles
pub use syscall::*;
pub use alloc::{vec::Vec, string::String, boxed::Box};