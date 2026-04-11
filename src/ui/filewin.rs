use crate::io::framebuffer::{fill_rect, draw_string};
use crate::ui::theme::*;
use crate::klib::string::{to_str, itoa, strlen};
use crate::klib::mem::memcpy;

use core::ptr::addr_of;

const BORDER:               u32 = 1;
const TITLE_H:              u32 = 20;
const LINE_H:               u32 = 16;
const PADDING:              u32 = 4;

const MAX_VISIBLE:          usize = 32;

static mut FILE_NAMES:      [[u8; 28]; MAX_VISIBLE] = [[0; 28]; MAX_VISIBLE];
static mut FILE_Y:          [u32; MAX_VISIBLE] = [0; MAX_VISIBLE];
static mut FILE_COUNT_VIS:  usize = 0;

/// Index de la fenêtre Fichiers dans WINDOWS
pub const FILE_WIN_IDX: usize = 1;

fn file_at_y(my: u32) -> Option<usize> {
    unsafe {
        let mut i = 0;
        while i < FILE_COUNT_VIS {
            if my >= FILE_Y[i] && my < FILE_Y[i] + LINE_H {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

pub fn on_click(mx: i32, my: i32) {
    let _ = mx;
    if let Some(idx) = file_at_y(my as u32) {
        unsafe {
            let name = &FILE_NAMES[idx];
            let len = strlen(name);
            let mut buf = [0u8; 512];
            let sz = crate::fs::kfs::read(&name[..len], &mut buf);
            if sz > 0 {
                crate::println!("{}", to_str(&buf[..sz]));
            }
        }
    }
}

/// Dessine le contenu de la fenêtre fichiers
pub fn draw() {
    unsafe {
        FILE_COUNT_VIS = 0;
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
        crate::fs::kfs::list(|name, size| {
            if FILE_COUNT_VIS >= MAX_VISIBLE { return; }
            let y = cy + PADDING + row * LINE_H;

            let len = strlen(name);
            memcpy(&mut FILE_NAMES[FILE_COUNT_VIS], name, len);
            FILE_NAMES[FILE_COUNT_VIS][len.min(27)] = 0;
            FILE_Y[FILE_COUNT_VIS] = y;
            FILE_COUNT_VIS += 1;

            let name_str = to_str(name);
            draw_string(cx + PADDING, y, name_str, TEXT_BG, WIN_CONTENT);
            // Taille à droite
            let mut buf = [0u8; 8];
            let size_str = to_str(itoa(size as u32, &mut buf));
            draw_string(cx + cw - PADDING - (size_str.len() as u32 * 8), y, size_str, TEXT_BG, WIN_CONTENT);
            row += 1;
        });

        if row == 0 {
            draw_string(cx + PADDING, cy + PADDING, "(vide)", TEXT_BG, WIN_CONTENT);
        }
    }
}
