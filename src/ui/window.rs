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
use crate::io::font::FONT;

/// Couleurs de la fenêtre
const BORDER_COLOR: u32 = 0x333333;     // gris foncé
const TITLE_BG:     u32 = 0x336699;     // bleu
const TITLE_FG:     u32 = 0xFFFFFF;     // blanc
const CONTENT_BG:   u32 = 0xC0C0C0;     // gris clair
const BORDER:       u32 = 1;            // épaisseur bordure
const TITLE_H:      u32 = 20;           // hauteur barre de titre

/// Dessine une fenêtre complète à la position (x, y) avec taille w x h
pub fn draw_window(x: u32, y: u32, w: u32, h: u32, title: &str) {
    // 1. Bordure (rectangle gris foncé = fond complet)
    fill_rect(x, y, w, h, BORDER_COLOR);

    // 2. Barre titre (à l'intérieur de la bordure)
    fill_rect(x + BORDER, y + BORDER, w - 2 * BORDER, TITLE_H, TITLE_BG);

    // 3. Texte du titre (centré verticalement dans la barre, décalé de 4 px à gauche)
    let text_y = y + BORDER + (TITLE_H - 16) / 2;
    draw_string(x + BORDER + 4, text_y, title, TITLE_FG, TITLE_BG);

    // 4. Zone de contenu (sous la barre de titre)
    fill_rect(x + BORDER, y + BORDER + TITLE_H, w - 2 * BORDER, h - 2 * BORDER - TITLE_H, CONTENT_BG);
}