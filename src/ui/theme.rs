/*
    ══════════════════════════════════════════════════════════
    ui/theme.rs — Palette de couleurs de KernOS
    ══════════════════════════════════════════════════════════
*/

// ── Couleurs de base ────────────────────────────────────────
pub const BLACK:                u32 = 0x000000;
pub const WHITE:                u32 = 0xFFFFFF;
pub const GRAY_DARK:            u32 = 0x1a1a1a;
pub const GRAY_MID:             u32 = 0x333333;
pub const GRAY_LIGHT:           u32 = 0x555555;
pub const GRAY_SILVER:          u32 = 0xC0C0C0;
pub const BLUE_DARK:            u32 = 0x003366;
pub const BLUE_MID:             u32 = 0x336699;
pub const BLUE_LIGHT:           u32 = 0x4488CC;
pub const RED:                  u32 = 0xFF3333;

// ── Fond ────────────────────────────────────────────────────
pub const DESKTOP_BG:           u32 = BLUE_DARK;    // fond du bureau
pub const TASKBAR_BG:           u32 = GRAY_DARK;    // fond de la taskbar

// ── Fenêtres ────────────────────────────────────────────────
pub const WIN_BORDER:           u32 = GRAY_MID;     // bordure
pub const WIN_TITLE_BG:         u32 = BLUE_MID;     // barre de titre
pub const WIN_TITLE_FG:         u32 = WHITE;        // texte titre
pub const WIN_CONTENT:          u32 = GRAY_SILVER;  // fond contenu
pub const WIN_CLOSE:            u32 = RED;          // bouton fermer
pub const WIN_TITLE_BG_FOCUSED: u32 = BLUE_LIGHT;   // titre fenêtre active (bleu plus clair)

// ── Taskbar ─────────────────────────────────────────────────
pub const BTN_VISIBLE:          u32 = BLUE_MID;     // bouton fenêtre visible
pub const BTN_HIDDEN:           u32 = GRAY_LIGHT;   // bouton fenêtre cachée
pub const BTN_FG:               u32 = WHITE;        // texte bouton

// ── Texte ───────────────────────────────────────────────────
pub const TEXT_FG:              u32 = WHITE;        // texte normal
pub const TEXT_BG:              u32 = BLUE_DARK;    // fond texte shell
