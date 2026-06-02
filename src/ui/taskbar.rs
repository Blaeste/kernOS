/*
    ══════════════════════════════════════════════════════════
    ui/taskbar.rs — Barre des tâches en bas de l'écran
    ══════════════════════════════════════════════════════════

    Affiche une barre de 30px en bas de l'écran avec un bouton
    par fenêtre enregistrée. Cliquer sur un bouton rend la
    fenêtre visible et la met au premier plan.

    CE MODULE FOURNIT :
    ───────────────────
    - draw()            → dessine la taskbar
    - on_click(mx, my)  → gère le clic sur un bouton
*/

use crate::io::framebuffer::{fill_rect, draw_string};
use crate::klib::string::{to_str, to_str_or, pad2};
use core::ptr::addr_of;

use crate::ui::theme::*;

pub const TASKBAR_H:    u32 = 30;
const BTN_W:            u32 = 120;
const BTN_H:            u32 = 22;
const BTN_MARGIN:       u32 = 4;
const LAUNCH_W:         u32 = 96;

/// Dessine la taskbar en bas de l'écran
pub fn draw() {
    unsafe {
        let sw = *addr_of!(crate::io::framebuffer::FB_WIDTH);
        let sh = *addr_of!(crate::io::framebuffer::FB_HEIGHT);

        // Fond
        fill_rect(0, sh - TASKBAR_H, sw, TASKBAR_H, TASKBAR_BG);

        // Un bouton par fenetre
        let count = *addr_of!(crate::ui::window::WINDOW_COUNT);
        let mut i  = 0;
        while i < count {
            let win = &crate::ui::window::WINDOWS[i];
            let btn_x = BTN_MARGIN + i as u32 * (BTN_W + BTN_MARGIN);
            let btn_y = sh - TASKBAR_H + (TASKBAR_H - BTN_H) / 2;
            let color = if win.visible { BTN_VISIBLE } else { BTN_HIDDEN };
            fill_rect(btn_x, btn_y, BTN_W, BTN_H, color);
            let title_str = to_str(&win.title[..win.title_len]);
            draw_string(btn_x + 4, btn_y + (BTN_H - 16) / 2, title_str, BTN_FG, color);
            i += 1;
        }

        // ── Bouton launcher (lance l'ELF hello en ring 3) ─────
        // Placé à gauche de l'horloge, en vert pour le distinguer.
        let launch_x = sw - 156;
        let launch_y = sh - TASKBAR_H + (TASKBAR_H - BTN_H) / 2;
        fill_rect(launch_x, launch_y, LAUNCH_W, BTN_H, 0x2E8B57);

        // Uptime à droite
        let ticks = crate::drivers::timer::get_ticks();
        let secs = ticks / 100;
        let mins = secs / 60;
        let s = secs % 60;

        // Convertir en string "MM::SS"
        let mut buf = [0u8; 8];
        pad2(mins as u32, &mut buf[0..2]);
        buf[2] = b':';
        pad2(s as u32, &mut buf[3..5]);
        let time_str = to_str_or(&buf[..5], "00:00");

        let clock_w: u32 = 5 * 8 + 8; // 5 chars * 8px + padding;
        let clock_x = sw - clock_w - 4;
        let clock_y = sh - TASKBAR_H + (TASKBAR_H - 16) / 2;
        fill_rect(clock_x - 4, sh - TASKBAR_H + 4, clock_w + 4, BTN_H, TASKBAR_BG);
        draw_string(clock_x, clock_y, time_str, BTN_FG, TASKBAR_BG);
    }
}

/// Retourne true si le clic était dans la taskbar
pub fn on_click(mx: i32, my: i32) -> bool {
    unsafe {
        let sh = *addr_of!(crate::io::framebuffer::FB_HEIGHT);
        if my < (sh - TASKBAR_H) as i32 { return false; }

        let sw = *addr_of!(crate::io::framebuffer::FB_WIDTH);
        // Bouton launcher ; lance l'ELF hello
        let launch_x = (sw - 156) as i32;
        let launch_y = (sh - TASKBAR_H + (TASKBAR_H - BTN_H) / 2) as i32;
        if mx >= launch_x && mx < launch_x + LAUNCH_W as i32
            && my >= launch_y && my < launch_y + BTN_H as i32 {
            crate::userspace::spawn_elf();
            return false;
        }

        let count = *addr_of!(crate::ui::window::WINDOW_COUNT);
        let mut i  = 0;
        while i < count {
            let btn_x = (BTN_MARGIN + i as u32 * (BTN_W + BTN_MARGIN)) as i32;
            let btn_y = (sh - TASKBAR_H + (TASKBAR_H - BTN_H) / 2) as i32;
            if mx >= btn_x && mx < btn_x + BTN_W as i32 && my >= btn_y && my < btn_y + BTN_H as i32 {
                let win = &mut crate::ui::window::WINDOWS[i];
                if win.visible {
                    win.visible = false;
                    if crate::ui::window::get_focused_win() == i {
                        crate::ui::window::set_focused_win(usize::MAX);
                    }
                } else {
                    win.visible = true;
                    crate::ui::window::bring_to_front_pub(i);
                    crate::ui::window::set_focused_win(i);
                }
                return true;
            }
            i += 1;
        }
        false
    }
}
