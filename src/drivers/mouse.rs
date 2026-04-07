/*
    ══════════════════════════════════════════════════════════
    SOURIS PS/2 — Driver souris 3 octets via IRQ12
    ══════════════════════════════════════════════════════════

    COMMENT FONCTIONNE LA SOURIS PS/2 ?
    ────────────────────────────────────
    La souris est connectée au contrôleur PS/2 (le même que le clavier).
    Elle communique via 2 ports I/O :
      - 0x60 → données (lire/écrire des octets vers la souris)
      - 0x64 → status/commandes (vérifier si la souris est prête)

    Au boot, la souris est désactivée. Il faut :
    1. Activer le port souris (commande 0xA8 sur port 0x64)
    2. Activer l'IRQ12 (bit 1 de l'octet de config du contrôleur)
    3. Envoyer 0xF4 à la souris (commande "enable data reporting")

    PROTOCOLE — PAQUETS DE 3 OCTETS :
    ──────────────────────────────────
    Chaque mouvement de souris envoie 3 octets via IRQ12 :

    ┌─────────────────────────────────────────────────────┐
    │  Octet 0 (flags)                                    │
    │  ┌───┬───┬───┬───┬───┬───┬───┬───┐                  │
    │  │ Y │ X │ YS│ XS│ 1 │ M │ R │ L │                  │
    │  │ ov│ ov│ gn│ gn│   │ id│ bt│ bt│                  │
    │  └───┴───┴───┴───┴───┴───┴───┴───┘                  │
    │  bit 0 = bouton gauche (L)                          │
    │  bit 1 = bouton droit  (R)                          │
    │  bit 2 = bouton milieu (M)                          │
    │  bit 3 = toujours 1 (sinon paquet invalide)         │
    │  bit 4 = signe du déplacement X (1 = négatif)       │
    │  bit 5 = signe du déplacement Y (1 = négatif)       │
    │  bits 6-7 = overflow X/Y                            │
    ├─────────────────────────────────────────────────────┤
    │  Octet 1 = déplacement X (dx) signé sur 8 bits      │
    │  Octet 2 = déplacement Y (dy) signé sur 8 bits      │
    └─────────────────────────────────────────────────────┘

    dx positif = souris vers la droite
    dy positif = souris vers le haut (inversé par rapport à l'écran !)

    CE MODULE FOURNIT :
    ───────────────────
    - init()            → active la souris PS/2 et l'IRQ12
    - on_irq(byte)      → reçoit un octet, assemble les paquets 3 octets
    - MOUSE_X, MOUSE_Y  → position globale du curseur en pixels
*/

use core::arch::asm;
use core::ptr::{addr_of, addr_of_mut};

// ── Sprite du curseur (flèche 12x16) ───────────────────────────────────
const CURSOR_W: u32 = 12;
const CURSOR_H: u32 = 16;

/// Forme du curseur — chaque u16 = une ligne, bit 1 = pixel blanc, bit 0 = transparent
/// Le bit de poids fort (0x800) = pixel le plus à gauche
static CURSOR_SHAPE: [u16; 16] = [
    0b1000_0000_0000_0000,  // X
    0b1100_0000_0000_0000,  // XX
    0b1110_0000_0000_0000,  // XXX
    0b1111_0000_0000_0000,  // XXXX
    0b1111_1000_0000_0000,  // XXXXX
    0b1111_1100_0000_0000,  // XXXXXX
    0b1111_1110_0000_0000,  // XXXXXXX
    0b1111_1111_0000_0000,  // XXXXXXXX
    0b1111_1111_1000_0000,  // XXXXXXXXX
    0b1111_1100_0000_0000,  // XXXXXX
    0b1111_0000_0000_0000,  // XXXX
    0b1101_1000_0000_0000,  // XX XX
    0b1000_1100_0000_0000,  // X  XX
    0b0000_0110_0000_0000,  //    XX
    0b0000_0011_0000_0000,  //     XX
    0b0000_0000_0000_0000,  //
];

/// Envoie une commande au contrôleur PS/2
unsafe fn ps2_write_cmd(cmd: u8) {
    // Attendre que le buffer d'entrée soit vide
    while unsafe { read_status() & 0x02 } != 0 {}
    unsafe { outb(0x64, cmd); }
}

/// Envoie une donnée à la souris
unsafe fn ps2_write_data(data: u8) {
    while unsafe { read_status() & 0x02 } != 0 {}
    unsafe { outb(0x60, data); }
}

/// Lit le registre status du contrôleur PS/2 (port 0x64)
unsafe fn read_status() -> u8 {
    unsafe { inb(0x64) }
}

// ── Init PS/2 ───────────────────────────────────────────────────────────
pub unsafe fn init() {
    unsafe { ps2_write_cmd(0xA8); }                     // activer la souris

    unsafe { ps2_write_cmd(0x20); }                     // lire octet de config
    while unsafe { read_status() & 0x01 } == 0 {}       // attendre que les données soient prêtes
    let config = unsafe { inb(0x60) } | 0x02;           // activer IRQ12
    unsafe { ps2_write_cmd(0x60); }                     // ecrire config
    unsafe { ps2_write_data(config); }                  // envoyer la config modifiée à la souris

    unsafe { ps2_write_cmd(0xD4); }                     // prochain octet -> soruis
    unsafe { ps2_write_data(0xF4); }                    // enable data reporting
}

// ── Paquets souris ──────────────────────────────────────────────────────
static mut PACKET:          [u8; 3] = [0; 3];
static mut PACKET_IDX:      usize = 0;

// ── Position du curseur ─────────────────────────────────────────────────
pub static mut MOUSE_X:     i32 = 0;                      // position X en pixels
pub static mut MOUSE_Y:     i32 = 0;                      // position Y en pixels

static mut OLD_MOUSE_X:     i32 = 0;                      // ancienne position X (pour effacer)
static mut OLD_MOUSE_Y:     i32 = 0;                      // ancienne position Y (pour effacer)

static mut CURSOR_BACKUP:   [u32; 192] = [0; 192];        // sauvegarde des 12×16 pixels sous le curseur

/// Centre la souris et sauvgarde les pixels sous el curseur initial.
// Doit être appelé après init du framebuffer et l'affichage du texte.
pub fn init_cursor() {
    unsafe {
        let w = *addr_of!(crate::io::framebuffer::FB_WIDTH) as i32;
        let h = *addr_of!(crate::io::framebuffer::FB_HEIGHT) as i32;

        *addr_of_mut!(MOUSE_X) = w / 2;
        *addr_of_mut!(MOUSE_Y) = h / 2;

        let mx = *addr_of!(MOUSE_X) as u32;
        let my = *addr_of!(MOUSE_Y) as u32;

        let mut i: u32 = 0;
        while i < CURSOR_H {
            let mut j: u32 = 0;
            while j < CURSOR_W {
                CURSOR_BACKUP[(i * CURSOR_W + j) as usize] = crate::io::framebuffer::get_pixel(mx + j, my + i);
                j += 1;
            }
            i += 1;
        }

        *addr_of_mut!(OLD_MOUSE_X) = w / 2;
        *addr_of_mut!(OLD_MOUSE_Y) = h / 2;
    }
}

// ── IRQ12 — paquets souris ──────────────────────────────────────────────
pub fn on_irq(byte: u8) {
    let result = unsafe {
        // Si on attend le 1er octet et que le bit n'est aps et -> octet invalide, on ignore
        if PACKET_IDX == 0 && (byte & 0x08) == 0 {
            return;
        }

        PACKET[PACKET_IDX] = byte;
        PACKET_IDX += 1;

        if PACKET_IDX == 3 {
            PACKET_IDX = 0;
            let flags = PACKET[0];
            let dx = PACKET[1] as i8;
            let dy = PACKET[2] as i8;
            Some((dx, dy, (flags & 0x01) != 0, (flags & 0x02) != 0))
        } else {
            None
        }
    };

    if let Some((dx, dy, btn_left, _btn_right)) = result {
        unsafe {
            // Met à jour la position (dy iversé : souris vers le haut = valeur positive, mais écran Y augmente vers le bas)
            *addr_of_mut!(MOUSE_X) += dx as i32;
            *addr_of_mut!(MOUSE_Y) -= dy as i32; // inversé !

            // Clamp : empêche le curseur de sortir de l'écran
            let w = *addr_of!(crate::io::framebuffer::FB_WIDTH) as i32;
            let h = *addr_of!(crate::io::framebuffer::FB_HEIGHT) as i32;

            let mx = *addr_of!(MOUSE_X);
            if mx < 0 { *addr_of_mut!(MOUSE_X) = 0; }
            if mx >= w { *addr_of_mut!(MOUSE_X) = w - 1; }

            let my = *addr_of!(MOUSE_Y);
            if my < 0 { *addr_of_mut!(MOUSE_Y) = 0; }
            if my >= h { *addr_of_mut!(MOUSE_Y) = h - 1; }

            // Gestion du drag des fenêtres
            let mx = *addr_of!(MOUSE_X);
            let my = *addr_of!(MOUSE_Y);

            let mut redrawn = false;
            if btn_left {
                crate::ui::window::on_mouse_down(mx, my);
                if crate::ui::taskbar::on_click(mx, my) {
                    crate::ui::window::set_needs_redraw();
                }
                crate::ui::window::erase_dragged_window();
                crate::ui::window::on_mouse_move(mx, my);
                if crate::ui::window::is_dragging() || crate::ui::window::needs_redraw() {
                    // Efface tout le fond pour éviter les fenêtres fantômes
                    crate::io::framebuffer::clear(crate::ui::theme::DESKTOP_BG);
                    crate::ui::window::draw_all();
                    crate::ui::taskbar::draw();
                    redrawn = true;
                }
            } else {
                crate::ui::window::on_mouse_up();
            }

            // Efface l'ancien curseur (replace ancien pixels sauvgardé)
            if !redrawn {
                let ox = *addr_of!(OLD_MOUSE_X) as u32;
                let oy = *addr_of!(OLD_MOUSE_Y) as u32;
                let mut i = 0;

                while i < CURSOR_H {
                    let mut j = 0;
                    while j < CURSOR_W {
                        crate::io::framebuffer::put_pixel(ox + j, oy + i, CURSOR_BACKUP[(i * CURSOR_W + j) as usize]);
                        crate::io::framebuffer::put_pixel_direct(ox + j, oy + i, CURSOR_BACKUP[(i * CURSOR_W + j) as usize]);
                        j += 1;
                    }
                    i += 1;
                }
            }
            
            // Sauvegarde les pixels sous le nouveau curseur
            let nx = *addr_of!(MOUSE_X) as u32;
            let ny = *addr_of!(MOUSE_Y) as u32;
            let mut i: u32 = 0;
            while i < CURSOR_H {
                let mut j: u32 = 0;
                while j < CURSOR_W {
                    CURSOR_BACKUP[(i * CURSOR_W + j) as usize] = crate::io::framebuffer::get_pixel(nx + j, ny + i);
                    j += 1;
                }
                i += 1;
            }

            // Dessine le curseur flèche
            i = 0;
            while i < CURSOR_H {
                let row = CURSOR_SHAPE[i as usize];
                let mut j: u32 = 0;
                while j < CURSOR_W {
                    if (row >> (15 - j)) & 1 == 1 {
                        crate::io::framebuffer::put_pixel(nx + j, ny + i, 0xFFFFFF);
                        crate::io::framebuffer::put_pixel_direct(nx + j, ny + i, 0xFFFFFF);
                    }
                    j += 1;
                }
                i += 1;
            }

            // Affiche tout d'un coup seulement si on a redessiné
            if redrawn {
                crate::io::framebuffer::swap_buffers();
            }

            // Sauvegarde la position actuelle
            *addr_of_mut!(OLD_MOUSE_X) = *addr_of!(MOUSE_X);
            *addr_of_mut!(OLD_MOUSE_Y) = *addr_of!(MOUSE_Y);
        }
    }
}

// ── Helpers I/O ─────────────────────────────────────────────────────────

unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    unsafe { asm!("in al, dx", out("al") v, in("dx") port, options(nostack)); }
    v
}

unsafe fn outb(port: u16, value: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") value, options(nostack)); }
}