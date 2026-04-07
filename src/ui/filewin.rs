use crate::io::framebuffer::{fill_rect, draw_string};
use crate::ui::theme::*;
use crate::klib::string::to_str;
use core::ptr::addr_of;

const BORDER:   u32 = 1;
const TITLE_H:  u32 = 20;
const LINE_H:   u32 = 16;
const PADDING:  u32 = 4;

/// Index de la fenêtre Fichiers dans WINDOWS
pub const FILE_WIN_IDX: usize = 1;

/// Dessine le contenu de la fenêtre fichiers
pub fn draw() {
    unsafe {
        let count = *addr_of!(crate::ui::window::WINDOW_COUNT);
        if FILE_WIN_IDX >= count { return; }
        let win = &crate::ui::window::WINDOWS[FILE_WIN_IDX];
        if !win.visible { return; }

        let cx = win.x + BORDER;
        let cy = win.y + BORDER + TITLE_H;
        let cw = win.w - 2 * BORDER;
        let ch = win.h - 2 * BORDER - TITLE_H;

        // Fond contenu
        fill_rect(cx, cy, cw, ch, WIN_CONTENT);

        // List les fichiers
        let mut row: u32 = 0;
        crate::fs::ramfs::list(|name, size| {
            let y = cy + PADDING + row * LINE_H;
            let name_str = to_str(name);
            draw_string(cx + PADDING, y, name_str, TEXT_BG, WIN_CONTENT);
            // Taille à droite
            let mut buf = [0u8; 8];
            let mut n = size;
            let mut len = 0;
            if n == 0 {
                buf[0] = b'0';
                len = 1;
            } else {
                while n > 0 && len < 8 {
                    buf[7 - len] = b'0' + (n % 10) as u8;
                    n /= 10;
                    len += 1;
                }
            }
            let size_str = to_str(&buf[8 - len..]);
            draw_string(cx + cw - PADDING - (len as u32 * 8), y, size_str, TEXT_BG, WIN_CONTENT);
            row += 1;
        });

        if row == 0 {
            draw_string(cx + PADDING, cy + PADDING, "(vide)", TEXT_BG, WIN_CONTENT);
        }
    }
}