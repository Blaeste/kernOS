/*
    ══════════════════════════════════════════════════════════
    IPC — Inter-Process Communication (Phase F)
    ══════════════════════════════════════════════════════════

    PIPE — canal de communication unidirectionnel
    ─────────────────────────────────────────────
    Un pipe connecte un writer fd à un reader fd via un buffer
    circulaire de 256 octets résidant dans le kernel.

      writer fd  →  [data: 256 o, write_pos]
                           ↓
      reader fd  ←  [data: 256 o, read_pos]

    Non-bloquant : si le buffer est vide, read renvoie 0.
    Si writers = 0 et buffer vide, le pipe est libéré.

    UTILISATION DEPUIS USER
    ───────────────────────
      let (fd_r, fd_w) = pipe();     // crée le pipe
      write(fd_w, b"hello");         // écrit dans le pipe
      let n = read(fd_r, &mut buf);  // lit depuis le pipe
      close(fd_w);
      close(fd_r);

    TYPES DE FD
    ───────────
    FileDesc.kind discrimine le type de ressource dans la fd table :
*/

use core::ptr::{addr_of, addr_of_mut};


// ── Types de fd ───────────────────────────────────────────────────────────

/// fd pointant ver sun fichier KFS (comportement par défault).
pub const FD_KFS:           u8 = 0;

/// fd = extrémité lecture d'un pipe. FileDesc.pos = index dans PIPES.
pub const FD_PIPE_READ:     u8 = 1;

/// fd = extrémité écriture d'un piep. FileDesc.pos = index dans PIPES.
pub const FD_PIPE_WRITE:    u8 = 2;


// ── Structure Pipe ────────────────────────────────────────────────────────

/// Taille du buffer circulaire d'un pipe.
const PIPE_BUF:             usize = 256;

/// Nombre max de pipes simultanés dans le kernel.
pub const MAX_PIPES:        usize = 4;

pub struct Pipe {

    pub data:       [u8; PIPE_BUF],
    pub read_pos:   usize,
    pub write_pos:  usize,
    pub len:        usize,                  // octets disponibles en lecture
    pub writers:    u8,                     // fd write encore ouverts (0 = EOF côté reader)
    pub active:     bool,
}

pub static mut PIPES: [Pipe; MAX_PIPES] = [const { Pipe {

    data:           [0; PIPE_BUF],
    read_pos:       0,
    write_pos:      0,
    len:            0,
    writers:        0,
    active:         false,
} }; MAX_PIPES];


// ── Helpers kernel ────────────────────────────────────────────────────────

/// Aloue un slot pipe libre. Retourne l'index ou usize::MAX si table pleine.
pub fn alloc_pipe() -> usize {

    unsafe {
        for i in 0..MAX_PIPES {

            if !(*addr_of!(PIPES))[i].active {

                (*addr_of_mut!(PIPES))[i] = Pipe {
                    data:       [0; PIPE_BUF],
                    read_pos:   0,
                    write_pos:  0,
                    len:        0,
                    writers:    1,
                    active:     true,
                };
                return i;
            }
        }
        usize::MAX
    }
}

/// Ecrit 'data' dans le pipe. Retourne les octets écrits (0 si buffer plein).
pub fn pipe_write(idx: usize, data: &[u8]) -> usize {

    unsafe {
        let p = &mut (*addr_of_mut!(PIPES))[idx];
        if !p.active { return 0; }

        let mut n = 0usize;

        for &b in data {

            if p.len >= PIPE_BUF { break; }

            p.data[p.write_pos] = b;
            p.write_pos = (p.write_pos + 1) % PIPE_BUF;
            p.len += 1;
            n += 1;
        }
        n
    }
}

/// Lit jusqu'à 'out.len()' octets depuis le pipe. Non_bloquant (0 si vide).
pub fn pipe_read(idx: usize, out: &mut [u8]) -> usize {

    unsafe {
        let p = &mut (*addr_of_mut!(PIPES))[idx];
        if !p.active || p.len == 0 { return 0; }

        let mut n = 0usize;

        while n < out.len() && p.len > 0 {

            out[n] = p.data[p.read_pos];
            p.read_pos = (p.read_pos + 1) % PIPE_BUF;
            p.len -= 1;
            n += 1;
        }
        n
    }
}

/// Ferme l'extrémité écriture. Libère le pipe si plus de writer ni de données.
pub fn pipe_close_write(idx: usize) {

    unsafe {
        let p = &mut (*addr_of_mut!(PIPES))[idx];
        if p.writers > 0 { p.writers -= 1; }
        if p.writers == 0 { p.active = false; }
    }
}

/// Ferme l'extrémité lecture. libère le pipe immédiatement.
pub fn pipe_close_read(idx: usize) {

    unsafe { (*addr_of_mut!(PIPES))[idx].active = false; }
}