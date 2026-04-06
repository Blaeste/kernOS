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

/// Couleurs de la fenêtre
const BORDER_COLOR: u32 = 0x333333;     // gris foncé
const TITLE_BG:     u32 = 0x336699;     // bleu
const TITLE_FG:     u32 = 0xFFFFFF;     // blanc
const CONTENT_BG:   u32 = 0xC0C0C0;     // gris clair
const BORDER:       u32 = 1;            // épaisseur bordure
const TITLE_H:      u32 = 20;           // hauteur barre de titre

/// Nombre max de fenêtres
const MAX_WINDOWS: usize = 8;

/// Longueur max du titre
const MAX_TITLE: usize = 32;

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
static mut WINDOWS: [Window; MAX_WINDOWS] = [const { Window {
    x: 0, y: 0, w: 0, h: 0,
    title: [0; MAX_TITLE],
    title_len: 0,
    visible: false,
} }; MAX_WINDOWS];

static mut WINDOW_COUNT: usize = 0;

// ── Drag & drop ─────────────────────────────────────────────────────────
static mut DRAGGING:        bool = false;
static mut DRAG_WIN:        usize = 0;
static mut DRAG_OFFSET_X:   i32 = 0;
static mut DRAG_OFFSET_Y:   i32 = 0;

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
        let len = if bytes.len() < MAX_TITLE { bytes.len() } else { MAX_TITLE };
        let mut i = 0;
        while i < len {
            WINDOWS[idx].title[i] = bytes[i];
            i += 1;
        }
        WINDOWS[idx].title_len = len;

        WINDOW_COUNT += 1;
        idx
    }
}

/// Dessine une seule fenêtre
fn draw_one(win: &Window) {
    if !win.visible { return; }

    // Bordure
    fill_rect(win.x, win.y, win.w, win.h, BORDER_COLOR);

    // Barre de titre
    fill_rect(win.x + BORDER, win.y + BORDER, win.w - 2 * BORDER, TITLE_H, TITLE_BG);

    // Titre
    let text_y = win.y + BORDER + (TITLE_H - 16) / 2;
    let title_str = core::str::from_utf8(&win.title[..win.title_len]).unwrap_or("");
    draw_string(win.x + BORDER + 4, text_y, title_str, TITLE_FG, TITLE_BG);

    // Contenu
    fill_rect(win.x + BORDER, win.y + BORDER + TITLE_H, win.w - 2 * BORDER, win.h - 2 * BORDER - TITLE_H, CONTENT_BG);
}

/// Dessine toutes les fenêtres (z-order = odre du tableau)
pub fn draw_all() {
    unsafe {
        let mut i = 0;
        while i < WINDOW_COUNT {
            draw_one(&WINDOWS[i]);
            i += 1;
        }
    }
}

/// CLic gauche enfoncé - comme le drag si on clique sur une barre de titre
pub fn on_mouse_down(mx: i32, my: i32) {
    unsafe {
        if DRAGGING { return; }
        // Parcourir les fenêtre de la plsu haute (dernière) à la plus basse
        let mut i = WINDOW_COUNT;
        while i > 0 {
            i -= 1;
            let win = &WINDOWS[i];
            if !win.visible { continue; }

            let wx = win.x as i32;
            let wy = win.y as i32;
            let ww = win.w as i32;

            // Est-ce que le clic est dans la abrre de titre ?
            if mx >= wx && mx < wx + ww && my >= wy && my < wy + TITLE_H as i32 + BORDER as i32 {
                DRAGGING = true;
                DRAG_WIN = i;
                DRAG_OFFSET_X = mx - wx;
                DRAG_OFFSET_Y = my - wy;
                return;
            }
        }
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
        if new_x < 0 { new_x = 0; }
        if new_y < 0 { new_y = 0; }
        if new_x + ww > sw { new_x = sw - ww; }
        if new_y + wh > sh { new_y = sh - wh; }

        WINDOWS[DRAG_WIN].x = new_x as u32;
        WINDOWS[DRAG_WIN].y = new_y as u32;

        // Verrouille la souris sur la fenêtre
        *addr_of_mut!(crate::drivers::mouse::MOUSE_X) = new_x + DRAG_OFFSET_X;
        *addr_of_mut!(crate::drivers::mouse::MOUSE_Y) = new_y + DRAG_OFFSET_Y;
    }
}

/// Clic relâché -  arrête le drag
pub fn on_mouse_up() {
    unsafe {
        DRAGGING = false;
    }
}