/*
    ══════════════════════════════════════════════════════════
    ui/editor.rs — Éditeur de texte (fenêtre à contenu custom)
    ══════════════════════════════════════════════════════════

    Modèle calqué sur filewin.rs : la fenêtre est créée au boot
    (cachée) à l'index EDITOR_WIN_IDX ; draw() repeint son contenu.

    - open(name)  → charge un fichier KFS dans BUF, affiche + focus
    - draw()      → rend le texte (wrap + \n) + le curseur
    Étape 2 ajoutera on_key() pour l'édition et la sauvegarde.
*/

use crate::io::framebuffer::{fill_rect, draw_char};
use crate::ui::theme::*;
use core::ptr::{addr_of, addr_of_mut};

const BORDER:               u32 = 1;
const TITLE_H:              u32 = 20;
const PADDING:              u32 = 4;
const CHAR_W:               u32 = 8;
const CHAR_H:               u32 = 16;


/// Index de la fenêtre Editeur (créée au boot, cachée jusqu'à 'edit')
pub const EDITOR_WIN_IDX:   usize = 2;

const BUF_MAX:              usize = 4096;

static mut BUF:             [u8; BUF_MAX] = [0; BUF_MAX];
static mut LEN:             usize = 0;
static mut CURSOR:          usize = 0;
static mut FILENAME:        [u8; 28] = [0; 28];
static mut FNAME_LEN:       usize = 0;
static mut OPEN:            bool = false;

pub fn is_open() -> bool { unsafe { OPEN } }


/// Ouvre un fichier KFS dans l'éditeur (vide si introuvable).
pub fn open(name: &[u8]) {

    unsafe {
        let n = name.len().min(28);
        for i in 0..n { FILENAME[i] = name[i]; }
        FNAME_LEN = n;

        // Charge le contenu directement dans BUF
        let buf = &mut *addr_of_mut!(BUF);
        let sz = crate::fs::kfs::read(&name[..n], buf);

        LEN = sz.min(BUF_MAX);
        CURSOR = LEN;
        OPEN = true;

        // Affiche + focus la fenêtre éditeur
        let win = &mut (*addr_of_mut!(crate::ui::window::WINDOWS))[EDITOR_WIN_IDX];

        win.visible = true;
        crate::ui::window::bring_to_front_pub(EDITOR_WIN_IDX);
        crate::ui::window::set_focused_win(EDITOR_WIN_IDX);

        crate::io::framebuffer::clear(crate::ui::theme::DESKTOP_BG);
        crate::ui::window::draw_all();
        crate::ui::taskbar::draw();
        crate::io::framebuffer::swap_buffers();
    }
}

// Rend le contenu de l'éditeur dans sa fenêtre (appelé par draw_all).
pub fn draw() {

    unsafe {
        if !OPEN { return; }

        let win = &(*addr_of!(crate::ui::window::WINDOWS))[EDITOR_WIN_IDX];
        if !win.visible { return; }

        let cx = win.x + BORDER;
        let cy = win.y + BORDER + TITLE_H;
        let cw = win.w - 2 * BORDER;
        let ch = win.h - 2 * BORDER - TITLE_H;

        fill_rect(cx, cy, cw, ch, WIN_CONTENT);

        let cols = cw.saturating_sub(2 * PADDING) / CHAR_W;
        let rows = ch.saturating_sub(2 * PADDING) / CHAR_H;

        let mut col:    u32 = 0;
        let mut row:    u32 = 0;

        let mut cur_x = cx + PADDING;
        let mut cur_y = cy + PADDING;

        let buf = &*addr_of!(BUF);
        let mut i = 0;
        while i <= LEN {

            // Mémorise la position écran du curseur à l'index CURSOR
            if i == CURSOR {

                cur_x = cx + PADDING + col * CHAR_W;
                cur_y = cy + PADDING + row * CHAR_H;
            }

            if i == LEN { break; }

            let c = buf[i];
            if c == b'\n' {

                col = 0;
                row += 1;
            } else {

                if row < rows && col < cols {

                    let x = cx + PADDING + col * CHAR_W;
                    let y = cy + PADDING + row * CHAR_H;

                    draw_char(x, y, c, TEXT_BG, WIN_CONTENT);
                }
                col += 1;
                if col >= cols { col = 0; row += 1; }
            }
            i += 1;
        }

        // Curseur : barre verticale noire
        if cur_y + CHAR_H <= cy + ch {
            fill_rect(cur_x, cur_y, 2, CHAR_H, 0x000000);
        }

        // Poignée de resize
        fill_rect(win.x + win.w - BORDER - 10, win.y + win.h - BORDER - 10, 10, 10, WIN_BORDER);
    }
}

/// Traite une frappe clavier (appelé depuis keyboard.rs quand l'editeur est en focus).
pub fn on_key(ch: u8, ctrl: bool) {

    unsafe {
        if ctrl {

            match ch {
                b's' | b'S' => { save(); return; }
                b'q' | b'Q' => { close(); return; }
                _           => return,                  // autres raccouris Ctrl ignorés
            }
        }

        if ch == 27 { close(); return; }                // Escape ferme l'éditeur

        let buf = &mut *addr_of_mut!(BUF);

        match ch {
            8 => {                                      // Backspace : supprime l'octet avant le curseur
                if CURSOR > 0 {
                    let mut i = CURSOR;
                    while i < LEN { buf[i - 1] = buf[i]; i += 1; }
                    CURSOR -= 1;
                    LEN -= 1;
                } 
            }
            _ => {                                      // Insertion (caractère imprimable ou '\n') à la position du curseur
                if LEN < BUF_MAX {
                    let mut i = LEN;
                    while i > CURSOR { buf[i] = buf[i - 1]; i -= 1; }
                    buf[CURSOR] = ch;
                    CURSOR += 1;
                    LEN += 1;
                }
            }
        }

        // Redraw le contenu de l'éditeur + affiche
        draw();
        crate::io::framebuffer::swap_buffers();
    }
}

/// Sauvegarde le buffer dans le fichier KFS (écrase l'existant).
fn save() {

    unsafe {
        let buf = &*addr_of!(BUF);
        let name = &*addr_of!(FILENAME);
        crate::fs::kfs::create(&name[..FNAME_LEN], &buf[..LEN]);
    }
}

/// Ferme l'éditeur, rend le focus au Terminal et redessine.
fn close() {

    unsafe {
        OPEN = false;
        let win = &mut (*addr_of_mut!(crate::ui::window::WINDOWS))[EDITOR_WIN_IDX];
        win.visible = false;

        // Rend le focus à la fenêtre Terminal (writer lié)
        let term = (*addr_of!(crate::io::framebuffer::FB_WRITER)).bound_win;
        crate::ui::window::set_focused_win(term);

        crate::io::framebuffer::clear(crate::ui::theme::DESKTOP_BG);
        crate::ui::window::draw_all();
        crate::ui::taskbar::draw();
        crate::io::framebuffer::swap_buffers();
    }
}