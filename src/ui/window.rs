/*
    ══════════════════════════════════════════════════════════
    ui/window.rs — Dessin d'une fenêtre graphique
    ══════════════════════════════════════════════════════════

    STRUCTURE D'UNE FENÊTRE :
    ─────────────────────────
    ┌──────────────────────────────────────────┐
    │ ████ Barre de titre (20px, bleu) ██████ │
    │                                          │
    │       Zone de contenu (gris clair)       │
    │                                          │
    └──────────────────────────────────────────┘

    - Bordure : 1px, gris foncé (0x333333)
    - Barre de titre : 20px de haut, fond bleu (0x336699), texte blanc
    - Contenu : fond gris clair (0xC0C0C0)

    CE MODULE FOURNIT :
    ───────────────────
    - draw_window(x, y, w, h, title) → dessine une fenêtre complète
*/

use crate::io::framebuffer::{fill_rect, draw_string};
use core::ptr::{addr_of, addr_of_mut};

use crate::ui::theme::*;
use crate::klib::cmp::min;
use crate::klib::mem::memcpy;

/// Fenêtre
const BORDER:       u32 = 1;            // épaisseur bordure
const TITLE_H:      u32 = 20;           // hauteur barre de titre

/// Nombre max de fenêtres
const MAX_WINDOWS: usize = 8;

/// Longueur max du titre
const MAX_TITLE: usize = 32;

const CLOSE_BTN_SIZE: u32 = 14; // taille bouton fermer

/// Une fenetre graphique
pub struct Window {
    pub x:          u32,
    pub y:          u32,
    pub w:          u32,
    pub h:          u32,
    pub title:      [u8; MAX_TITLE],
    pub title_len:  usize,
    pub visible:    bool,
}

/// Tableau global de fenêtre
pub static mut WINDOWS: [Window; MAX_WINDOWS] = [const { Window {
    x: 0, y: 0, w: 0, h: 0,
    title: [0; MAX_TITLE],
    title_len: 0,
    visible: false,
} }; MAX_WINDOWS];

pub static mut WINDOW_COUNT: usize = 0;

// ── Drag & drop ─────────────────────────────────────────────────────────
static mut DRAGGING:        bool = false;
static mut DRAG_WIN:        usize = 0;
static mut DRAG_OFFSET_X:   i32 = 0;
static mut DRAG_OFFSET_Y:   i32 = 0;

static mut Z_ORDER: [usize; MAX_WINDOWS] = [0; MAX_WINDOWS];

static mut NEEDS_REDRAW: bool = false;
static mut FOCUSED_WIN: usize = usize::MAX;

static mut OLD_WIN_X: u32 = 0;
static mut OLD_WIN_Y: u32 = 0;

/// Crée une nouvelle fenêtre et retourne son index
pub fn create_window(x: u32, y: u32, w: u32, h: u32, title: &str) -> usize {
    unsafe {
        let idx = WINDOW_COUNT;
        WINDOWS[idx].x = x;
        WINDOWS[idx].y = y;
        WINDOWS[idx].w = w;
        WINDOWS[idx].h = h;
        WINDOWS[idx].visible = true;

        // Copie le titre octet par octet
        let bytes = title.as_bytes();
        let len = min(bytes.len(), MAX_TITLE);
        memcpy(&mut WINDOWS[idx].title, bytes, len);
        WINDOWS[idx].title_len = len;

        Z_ORDER[WINDOW_COUNT] = WINDOW_COUNT;
        WINDOW_COUNT += 1;
        idx
    }
}

/// Dessine une seule fenêtre
fn draw_one(win: &Window, idx: usize) {
    if !win.visible { return; }

    let x = win.x;
    let y = win.y;
    let w = win.w;
    let h = win.h;

    // 4 bordures (1px) - seulement les bords, pas le fond entier (no redrawn)
    fill_rect(x,            y,          w,      BORDER,     WIN_BORDER);   // haut
    fill_rect(x,            y + h - 1,  w,      BORDER,     WIN_BORDER);   // bas
    fill_rect(x,            y,          BORDER, h,          WIN_BORDER);   // gauche
    fill_rect(x + w - 1,    y,          BORDER, h,          WIN_BORDER);   // droite

    // Barre de titre
    let title_bg = unsafe { if FOCUSED_WIN == idx { WIN_TITLE_BG_FOCUSED } else { WIN_TITLE_BG } };
    fill_rect(x + BORDER, y + BORDER, w - 2 * BORDER, TITLE_H, title_bg);

    // Titre
    let text_y = y + BORDER + (TITLE_H - 16) / 2;
    let title_str = core::str::from_utf8(&win.title[..win.title_len]).unwrap_or("");
    draw_string(x + BORDER + 4, text_y, title_str, WIN_TITLE_FG, title_bg);

    // Bouton fermer
    let btn_x = x + w - BORDER - CLOSE_BTN_SIZE - 2;
    let btn_y =  y + BORDER + (TITLE_H - CLOSE_BTN_SIZE) / 2;
    fill_rect(btn_x, btn_y, CLOSE_BTN_SIZE, CLOSE_BTN_SIZE, WIN_CLOSE);

    // Contenu
    fill_rect(x + BORDER, y + BORDER + TITLE_H, w - 2 * BORDER, h - 2 * BORDER - TITLE_H, WIN_CONTENT);
}

/// Dessine toutes les fenêtres (z-order = odre du tableau)
pub fn draw_all() {
    unsafe {
        let bound = (*addr_of!(crate::io::framebuffer::FB_WRITER)).bound_win;
        let mut i = 0;
        while i < WINDOW_COUNT {
            let idx = Z_ORDER[i];
            draw_one(&WINDOWS[idx], idx);
            if idx == crate::ui::filewin::FILE_WIN_IDX {
                crate::ui::filewin::draw();
            }
            if idx == bound {
                redraw_writer_if_visible();
            }
            i += 1;
        }
    }
}

/// CLic gauche enfoncé - comme le drag si on clique sur une barre de titre
pub fn on_mouse_down(mx: i32, my: i32) {
    unsafe {
        if DRAGGING { return; }

        // Clic dans la taskbar : géré par taskbar::on_click. On ne defocus pas
        // ici -> évite un redraw plein écran inutile à chaque clic sur la barre.
        let sh_tb = *addr_of!(crate::io::framebuffer::FB_HEIGHT) as i32;
        if my >= sh_tb - crate::ui::taskbar::TASKBAR_H as i32 { return; }
        
        // Parcourir les fenêtre de la plsu haute (dernière) à la plus basse
        let mut i = WINDOW_COUNT;
        while i > 0 {
            i -= 1;
            let idx = Z_ORDER[i];
            let win = &WINDOWS[idx];
            if !win.visible { continue; }

            let wx = win.x as i32;
            let wy = win.y as i32;
            let ww = win.w as i32;

            // Bouton fermer ?
            let btn_x = (win.x + win.w - BORDER - CLOSE_BTN_SIZE - 2) as i32;
            let btn_y = (win.y + BORDER + (TITLE_H - CLOSE_BTN_SIZE) / 2) as i32;
            let btn_s = CLOSE_BTN_SIZE as i32;

            if mx >= btn_x && mx < btn_x + btn_s && my >= btn_y && my < btn_y + btn_s {
                WINDOWS[idx].visible = false;
                if FOCUSED_WIN == idx { FOCUSED_WIN = usize::MAX; }
                NEEDS_REDRAW = true;
                return;
            }

            // Est-ce que le clic est dans la barre de titre ?
            if mx >= wx && mx < wx + ww && my >= wy && my < wy + TITLE_H as i32 + BORDER as i32 {
                DRAGGING = true;
                DRAG_WIN = idx;
                *addr_of_mut!(OLD_WIN_X) = WINDOWS[idx].x;
                *addr_of_mut!(OLD_WIN_Y) = WINDOWS[idx].y;
                bring_to_front(idx);
                DRAG_OFFSET_X = mx - wx;
                DRAG_OFFSET_Y = my - wy;
                FOCUSED_WIN = idx;
                NEEDS_REDRAW = true;
                return;
            }

            // CLic dans la zone contenu ?
            let content_y = wy + TITLE_H as i32 +  BORDER as i32;
            let wh = win.h as i32;
            if mx >= wx && mx < wx + ww && my >= content_y && my < wy + wh {
                FOCUSED_WIN = idx;
                bring_to_front(idx);
                NEEDS_REDRAW = true;
                if idx == crate::ui::filewin::FILE_WIN_IDX {
                    crate::ui::filewin::on_click(mx, my);
                }
                return;
            }
        }
        // CLic hors de toute fenêtre -> perd le focus
        FOCUSED_WIN = usize::MAX;
        NEEDS_REDRAW = true;
    }
}

/// Souris bouge avec clic maintenu - déplace la fenêtre
pub fn on_mouse_move(mx: i32, my: i32) {
    unsafe {
        if !DRAGGING { return; }
        let mut new_x = mx - DRAG_OFFSET_X;
        let mut new_y = my - DRAG_OFFSET_Y;

        let sw = *addr_of!(crate::io::framebuffer::FB_WIDTH) as i32;
        let sh = *addr_of!(crate::io::framebuffer::FB_HEIGHT) as i32;
        let ww = WINDOWS[DRAG_WIN].w as i32;
        let wh = WINDOWS[DRAG_WIN].h as i32;

        // Clamp : empêche la fenêtre de sortir de l'écran
        let taskbar_h = crate::ui::taskbar::TASKBAR_H as i32;
        if new_x < 0 { new_x = 0; }
        if new_y < 0 { new_y = 0; }
        if new_x + ww > sw { new_x = sw - ww; }
        if new_y + wh > sh - taskbar_h { new_y = sh - taskbar_h - wh; }

        WINDOWS[DRAG_WIN].x = new_x as u32;
        WINDOWS[DRAG_WIN].y = new_y as u32;

        // Verrouille la souris sur la fenêtre
        *addr_of_mut!(crate::drivers::mouse::MOUSE_X) = new_x + DRAG_OFFSET_X;
        *addr_of_mut!(crate::drivers::mouse::MOUSE_Y) = new_y + DRAG_OFFSET_Y;
    }
}

/// Efface l'ancienne position de la fenêtre en drag (remplace par le fond)
pub fn erase_dragged_window() {
    unsafe {
        if !DRAGGING { return; }
        let ox = *addr_of!(OLD_WIN_X);
        let oy = *addr_of!(OLD_WIN_Y);
        let w = WINDOWS[DRAG_WIN].w;
        let h = WINDOWS[DRAG_WIN].h;
        crate::io::framebuffer::fill_rect(
            ox, oy, w, h,
            crate::ui::theme::DESKTOP_BG
        );
        // Sauvegarde la position courante comme "ancienne" pour le prochain appel
        *addr_of_mut!(OLD_WIN_X) = WINDOWS[DRAG_WIN].x;
        *addr_of_mut!(OLD_WIN_Y) = WINDOWS[DRAG_WIN].y;
    }
}

/// Clic relâché -  arrête le drag
pub fn on_mouse_up() {
    unsafe {
        DRAGGING = false;
    }
}

/// Amène la fenêtre idx au premier plan (dernière dans le z-order)
unsafe fn bring_to_front(idx: usize) {
    unsafe {
        let mut pos = 0;
        while pos < WINDOW_COUNT {
            if Z_ORDER[pos] == idx { break; }
            pos += 1;
        }
        if pos >= WINDOW_COUNT - 1 { return; }

        let saved = Z_ORDER[pos];
        let mut i = pos;
        while i < WINDOW_COUNT - 1 {
            Z_ORDER[i] = Z_ORDER[i + 1];
            i += 1;
        }
        Z_ORDER[WINDOW_COUNT - 1] = saved;
    }
}

/// Retourne true si une fenêtre est en cours de déplacement
pub fn is_dragging() -> bool {
    unsafe { DRAGGING }
}

/// Retourne bool si besoin de redrw ou pas
pub fn needs_redraw() -> bool {
    unsafe {
        if NEEDS_REDRAW {
            NEEDS_REDRAW = false;
            true
        } else {
            false
        }
    }
}

/// Wrapper public pour bring_to_front
pub fn bring_to_front_pub(idx: usize) {
    unsafe { bring_to_front(idx); }
}

/// Helper set need redraw
pub fn set_needs_redraw() {
    unsafe { NEEDS_REDRAW = true; }
}

/// Synchro position writer + redessin le text seulement si la fenetre est cisible
pub fn redraw_writer_if_visible() {
    unsafe {
        let writer = &mut *addr_of_mut!(crate::io::framebuffer::FB_WRITER);
        let idx = writer.bound_win;
        if idx >= WINDOW_COUNT { return; }
        let win = &WINDOWS[idx];
        if !win.visible { return; }
        // Sync la zone d'écriture sur la position actuelle de la fenetre
        writer.x_origin = win.x + BORDER;
        writer.y_origin = win.y + BORDER + TITLE_H;
        writer.max_cols = (win.w - 2 * BORDER) / 8;
        writer.max_rows = (win.h - 2 * BORDER - TITLE_H) / 16;
        writer.redraw_from_buf();
    }
}

/// get focused window
pub fn get_focused_win() -> usize {
    unsafe { FOCUSED_WIN }
}

/// set focused window
pub fn set_focused_win(idx: usize) {
    unsafe { FOCUSED_WIN = idx; }
}
