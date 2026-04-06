/// Framebuffer - accès direct aux pixels de l'écran.
/// GRUB nous donne l'adresse mémoire, la taille, le pitch.
/// Chaque pixel = 4 octets : Bleu, Vert, Rouge, Alpha (format BGR en mémoire).

/// Infos globales du framebuffer, remplies au boot.
pub static mut FB_ADDR:     u64 = 0;        // adresse physique du framebuffer
pub static mut FB_WIDTH:    u32 = 0;        // largeur en pixels
pub static mut FB_HEIGHT:   u32 = 0;        // hauteur en pixels
pub static mut FB_PITCH:    u32 = 0;        // octets par ligne

/// Initialise le framebuffer avec le sinfos du tag Multiboot2.
pub fn init(fb: &crate::multiboot::FramebufferTag) {
    unsafe {
        *core::ptr::addr_of_mut!(FB_ADDR) = fb.addr;
        *core::ptr::addr_of_mut!(FB_WIDTH) = fb.width;
        *core::ptr::addr_of_mut!(FB_HEIGHT) = fb.height;
        *core::ptr::addr_of_mut!(FB_PITCH) = fb.pitch;
    }
}

/// Dessine un pixel à la position (x, y) avec la couleur donnée.
/// color = 0xRRGGBB (ex: 0xFF0000 = rouge)
pub fn put_pixel(x: u32, y: u32, color: u32) {
    unsafe {
        let addr = *core::ptr::addr_of!(FB_ADDR);
        let pitch = *core::ptr::addr_of!(FB_PITCH);

        // calcule l'adresse du pixel : base + (y * pitch) + (x * 4)
        let pixel = (addr + (y as u64 * pitch as u64) + (x as u64 * 4)) as *mut u32;
        *pixel = color;
    }
}

/// Dessine un rectangle rempli.
/// (x, y) = coin haut-gauche, w = largeur, h = hauteur
pub fn fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) {
    let mut dy = 0;
    while dy < h {
        let mut dx = 0;
        while dx < w {
            put_pixel(x + dx, y + dy, color);
            dx += 1;
        }
        dy += 1;
    }
}

/// Efface tout l'écran avec la couleur donnée.
pub fn clear(color: u32) {
    unsafe {
        let w = *core::ptr::addr_of!(FB_WIDTH);
        let h = *core::ptr::addr_of!(FB_HEIGHT);
        fill_rect(0, 0, w, h, color);
    }
}