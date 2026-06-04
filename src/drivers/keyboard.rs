use core::ptr::{addr_of, addr_of_mut};

// Table scancode Set 1 -> Caractère ASCII (index = scancode)
// 0 = pas de caractère (touche spéciales)
static QWERTY_NORMAL: [u8; 58] = [
    0,  27,                                                         // 0x00=rien, 0x01=Escape
    b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0',     // 0x02-0x0B
    b'-', b'=', 8,                                                  // 0x0C=-, 0x0D==, 0x0E=Backspace
    9,                                                              // 0x0F=Tab
    b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p',     // 0x10-0x19
    b'[', b']', b'\n',                                              // 0x1A=[, 0x1B=], 0x1C=Enter
    0,                                                              // 0x1D=Left Ctrl
    b'a', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l',           // 0x1E-0x26
    b';', b'\'', b'`',                                              // 0x27=;, 0x28=', 0x29=`
    0,                                                              // 0x2A=Left Shift
    b'\\',                                                          // 0x2B=backslash
    b'z', b'x', b'c', b'v', b'b', b'n', b'm',                       // 0x2C-0x32
    b',', b'.', b'/',                                               // 0x33=,  0x34=.  0x35=/
    0,                                                              // 0x36=Right Shift
    0,                                                              // 0x37=Keypad *
    0,                                                              // 0x38=Left Alt
    b' ',                                                           // 0x39=Space
];

static QWERTY_SHIFT: [u8; 58] = [
    0,  27,                                                         // 0x00=rien, 0x01=Escape
    b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')',     // 0x02-0x0B
    b'_', b'+', 8,                                                  // 0x0C=_, 0x0D=+, 0x0E=Backspace
    9,                                                              // 0x0F=Tab
    b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P',     // 0x10-0x19
    b'{', b'}', b'\n',                                              // 0x1A={, 0x1B=}, 0x1C=Enter
    0,                                                              // 0x1D=Left Ctrl
    b'A', b'S', b'D', b'F', b'G', b'H', b'J', b'K', b'L',           // 0x1E-0x26
    b':', b'"', b'~',                                               // 0x27=:, 0x28=", 0x29=~
    0,                                                              // 0x2A=Left Shift
    b'|',                                                           // 0x2B=|
    b'Z', b'X', b'C', b'V', b'B', b'N', b'M',                       // 0x2C-0x32
    b'<', b'>', b'?',                                               // 0x33=<, 0x34=>, 0x35=?
    0,                                                              // 0x36=Right Shift
    0,                                                              // 0x37=Keypad *
    0,                                                              // 0x38=Left Alt
    b' ',                                                           // 0x39=Space
];

static AZERTY_NORMAL: [u8; 58] = [
    0,   27,                                                        // 0x00=rien, 0x01=Escape
    b'&', b'e', b'"', b'\'', b'(', b'-', b'e', b'_', b'c', b'a',    // 0x02-0x0B  (chiffres AZERTY sans accent)
    b')', b'=', 8,                                                  // 0x0C=), 0x0D==, 0x0E=Backspace
    9,                                                              // 0x0F=Tab
    b'a', b'z', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p',     // 0x10-0x19
    b'^', b'$', b'\n',                                              // 0x1A=^, 0x1B=$, 0x1C=Enter
    0,                                                              // 0x1D=Left Ctrl
    b'q', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l',           // 0x1E-0x26
    b'm', b'u', b'`',                                               // 0x27=m, 0x28=u, 0x29=`
    0,                                                              // 0x2A=Left Shift
    b'*',                                                           // 0x2B=*
    b'w', b'x', b'c', b'v', b'b', b'n', b',',                       // 0x2C-0x32
    b';', b':', b'!',                                               // 0x33=;, 0x34=:, 0x35=!
    0,                                                              // 0x36=Right Shift
    0,                                                              // 0x37=Keypad *
    0,                                                              // 0x38=Left Alt
    b' ',                                                           // 0x39=Space
];

static AZERTY_SHIFT: [u8; 58] = [
    0,   27,                                                        // 0x00=rien, 0x01=Escape
    b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0',     // 0x02-0x0B
    b'_', b'+', 8,                                                  // 0x0C=_, 0x0D=+, 0x0E=Backspace
    9,                                                              // 0x0F=Tab
    b'A', b'Z', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P',     // 0x10-0x19
    b'"', b'L', b'\n',                                              // 0x1A=", 0x1B=L, 0x1C=Enter
    0,                                                              // 0x1D=Left Ctrl
    b'Q', b'S', b'D', b'F', b'G', b'H', b'J', b'K', b'L',           // 0x1E-0x26
    b'M', b'%', b'~',                                               // 0x27=M, 0x28=%, 0x29=~
    0,                                                              // 0x2A=Left Shift
    b'|',                                                           // 0x2B=|
    b'W', b'X', b'C', b'V', b'B', b'N', b'?',                       // 0x2C-0x32
    b'.', b'/', b'!',                                               // 0x33=., 0x34=/, 0x35=!
    0,                                                              // 0x36=Right Shift
    0,                                                              // 0x37=Keypad *
    0,                                                              // 0x38=Left Alt
    b' ',                                                           // 0x39=Space
];

#[derive(Clone, Copy, PartialEq)]
pub enum Layout { Qwerty, Azerty }

pub static mut LAYOUT: Layout = Layout::Qwerty;

static mut SHIFT_HELD: bool = false;
static mut CTRL_HELD:  bool = false;
static mut E0_PENDING: bool = false;

pub fn on_key(scancode: u8) {

    // Ctrl gauche : appuyé (0x1D) / relâché (0x9D)
    if scancode == 0x1D { unsafe { CTRL_HELD = true; } return; }
    if scancode == 0x9D { unsafe { CTRL_HELD = false; } return; }

    if scancode == 0xE0 {
        unsafe { E0_PENDING = true; }
        return;
    }

    if unsafe { E0_PENDING } {
        unsafe { E0_PENDING = false; }
        match scancode {
            0x49 => { // pgUp
                unsafe {
                    let w = &mut *addr_of_mut!(crate::io::framebuffer::FB_WRITER);
                    let focused = crate::ui::window::get_focused_win();
                    if focused == w.bound_win { w.scroll_up(); }
                }
            }
            0x51 => { // PgDn
                unsafe {
                    let w = &mut *addr_of_mut!(crate::io::framebuffer::FB_WRITER);
                    let focused = crate::ui::window::get_focused_win();
                    if focused == w.bound_win { w.scroll_down(); }
                }
            }
            _ => {}
        }
        return;
    }

    // Shift appuyé (0x2A=gauche, 0x36=droite)
    if scancode == 0x2A || scancode == 0x36 {
        unsafe { SHIFT_HELD = true; }
        return;
    }
    // Shift relâché
    if scancode == 0xAA || scancode == 0xB6 {
        unsafe { SHIFT_HELD = false; }
        return;
    }

    // Ctrl gauche : appuyé (0x1D) / relâché (0x9D)
    if scancode == 0x1D { unsafe { CTRL_HELD = true;  } return; }
    if scancode == 0x9D { unsafe { CTRL_HELD = false; } return; }

    // Ignorer les relâchements
    if scancode >= 0x80 {
        return;
    }

    // Convertir en caractère
    if (scancode as usize) < QWERTY_NORMAL.len() {

        let ch = unsafe {
            match LAYOUT {
                Layout::Qwerty => if SHIFT_HELD { QWERTY_SHIFT[scancode as usize] } else { QWERTY_NORMAL[scancode as usize] },
                Layout::Azerty => if SHIFT_HELD { AZERTY_SHIFT[scancode as usize] } else { AZERTY_NORMAL[scancode as usize] },
            }
        };

        // Si un process userpace attend stdin, on lui route le caractere
        let stdin_pending = unsafe { crate::io::stdin::WAITING_TASK } != usize::MAX;
        if stdin_pending {

            if ch == 8 {
                // Backspace : efface du buffer + echo
                if crate::io::stdin::pop_last() {
                    unsafe {
                        (*addr_of_mut!(crate::io::framebuffer::FB_WRITER)).backspace();
                        crate::io::framebuffer::swap_buffers();
                    }
                }
            }

            else if ch == b'\n' {
                // Retour ligne
                crate::io::stdin::push(b'\n');
                crate::println!();
                crate::io::stdin::wake_waiting();
            }

            else {
                crate::io::stdin::push(ch);
                crate::print!("{}", ch as char);
            }
            return;
        }

        if ch == 0 {
            return; // Escape et touches speciales -> ignorer
        }

        let focused = crate::ui::window::get_focused_win();
        let in_editor = crate::ui::editor::is_open() && focused == crate::ui::editor::EDITOR_WIN_IDX;

        if ch == 27 {

            if in_editor { crate::ui::editor::on_key(ch, false); }
            return ;
        }

        if in_editor {

            crate::ui::editor::on_key(ch, unsafe { CTRL_HELD });
        } else {

            let bound = unsafe { (*addr_of!(crate::io::framebuffer::FB_WRITER)).bound_win };
            if focused == bound {
                crate::shell::on_char(ch);
            }
        }
    }
}
