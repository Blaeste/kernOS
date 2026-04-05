use core::arch::asm;
use core::arch::naked_asm;
use crate::println;
use crate::drivers::pic;				// pour appeler pic::eoi()
use crate::drivers::mouse;
use crate::drivers::keyboard;
use crate::drivers::timer;

/* 
	Entrée IDT — 16 octets exacts lus directement par le CPU.
	L'adresse du handler est découpée en 3 morceaux : héritage du x86 16-bit,
	le format n'a jamais changé pour rester compatible. 
*/
#[derive(Clone, Copy)]		// permet de copier la struct (nécessaire pour [IdtEntry; 256])
#[repr(C)] 					// ordre des champs garanti = celui déclaré (le CPU lit cette struct directement)
pub struct IdtEntry {
	offset_low:     u16,    // bits  0-15  de l'adresse du handler
	selector:       u16,    // sélecteur de segment de code dans la GDT (0x08 = notre segment 64-bit)
	ist:            u8,     // Interrupt Stack Table — 0 = utilise la stack courante, pas de stack dédiée
	flags:          u8,     // attributs : 0x8E = 1000_1110 (present=1, DPL=0 kernel, type=interrupt gate 64-bit)
	offset_mid:     u16,    // bits 16-31 de l'adresse du handler
	offset_high:    u32,    // bits 32-63 de l'adresse du handler
	_reserved:      u32,    // réservé CPU — doit être 0, sinon comportement indéfini
}

impl IdtEntry {
	// Entrée nulle — pour pré-remplir les 256 slots avant de les configurer
	pub const fn empty() -> Self {
		IdtEntry {
			offset_low:		0,
			selector:		0,
			ist:			0,
			flags:			0,
			offset_mid:		0,
			offset_high:	0,
			_reserved:		0,
		}
	}

	/*
		Crée une entrée valide pointant vers un handler.
		handler as u64 → récupère l'adresse mémoire de la fonction
		Les 3 découpages (& 0xFFFF, >> 16, >> 32) extraient chaque partie de l'adresse 64-bit 
	*/
	pub fn new(handler: unsafe extern "C" fn()) -> Self {
		// adresse de la fonction en 64-bit
		let addr = handler as u64;
		IdtEntry {
			offset_low:		(addr & 0xFFFF) as u16,				// bits 0-15
			selector:		0x08,								// segment de code de la GDT
			ist:			0,									// 0 = utilise la stack courante, pas de stack dédiée aux interruptions
			flags:			0x8E,								// 1000_1110 : present=1, DPL=0, type=interrupt gate
			offset_mid:		((addr >> 16) & 0xFFFF) as u16,		// bits 16-31
			offset_high:	((addr >> 32) & 0xFFFFFFFF) as u32,	// bits 32-63
			_reserved:		0,
		}
	}
}

/*
	Table IDT complète — 256 entrées de 16 octets = 4096 octets.
	Entrées 0-31  : exceptions CPU (page fault, double fault, division par zéro...)
	Entrées 32-255 : interruptions matérielles et logicielles (clavier, timer...) 
*/
pub struct Idt {
	entries: [IdtEntry; 256],
}

/* 
	Pointeur IDT — structure de 10 octets passée à l'instruction lidt.
	#[repr(C, packed)] : pas de padding — le CPU exige exactement 2+8=10 octets consécutifs 
*/
#[repr(C, packed)]
struct IdtPointer {
	limit:	u16,	// taille totale de la table - 1 (256 * 16 - 1 = 4095)
	base:	u64,	// adresse mémoire du début de la table IDT
}

impl Idt {
	// Initialise une IDT avec 256 entrées vides
	pub const fn new() -> Self {
		Idt {
			entries: [IdtEntry::empty(); 256],
		}
	}

	// Enregistre un handler pour l'entrée index (0-255)
	pub fn set_handler(&mut self, index: u8, handler: unsafe extern "C" fn()) {
		self.entries[index as usize] = IdtEntry::new(handler);
	}

    /* 
		Charge l'IDT dans le registre IDTR du CPU via l'instruction lidt.
		Après ça, le CPU sait où chercher les handlers en cas d'exception. 
	*/
	pub fn load(&self) {
		let pointer = IdtPointer {
			limit: (256 * core::mem::size_of::<IdtEntry>() - 1) as u16,
			base: self as *const _ as u64,
		};
		unsafe {
			asm!("lidt [{}]", in(reg) &pointer, options(nostack));
		}
	}
}

// Numéros des exceptions CPU (définis par Intel, immuables)
pub const DIVIDE_ERROR:		u8 = 0;		// division par zéro
pub const DEBUG:			u8 = 1;		// point d'arrêt debug
pub const NMI:				u8 = 2;		// interruption non-maskable (erreur matérielle grave)
pub const BREAKPOINT:		u8 = 3;		// instruction int3
pub const OVERFLOW:			u8 = 4;		// dépassement arithmétique
pub const BOUND_RANGE:		u8 = 5;		// accès hors bornes
pub const INVALID_OPCODE:	u8 = 6;		// instruction inconnue du CPU
pub const DOUBLE_FAULT:		u8 = 8;		// deux exceptions en même temps - fatal
pub const GENERAL_PROTECT:	u8 = 13;	// violation de protection mémoire
pub const PAGE_FAULT:		u8 = 14;	// accès mémoire invalide

// Numéros IRQ dans l'IDT (après PIC : 32 + numéro IRQ)
pub const IRQ_TIMER:		u8 = 32;	// IRQ0 - horloge système
pub const IRQ_KEYBOARD:		u8 = 33;	// IRQ1 - clavier
pub const IRQ_CASCADE:		u8 = 34;	// IRQ2 - liaison PIC esclave
pub const IRQ_MOUSE:		u8 = 44;	// IRQ12 - souris

// Handlers - Le CPU saute ici en cas d'exception
// unsafe extern "C" : Le CPU appelle ces fonctions directement, pas Rust
pub unsafe extern "C" fn handler_divide_error() {
	println!("EXCEPTION: Division par zero !");
	loop {}
}

pub unsafe extern "C" fn handler_debug() {
	println!("EXCEPTION: Debug !");
	loop {}
}

pub unsafe extern "C" fn handler_nmi() {
	println!("EXCEPTION: NMI -  erreur matériel grave !");
	loop {}
}

pub unsafe extern "C" fn handler_breakpoint() {
	println!("EXCEPTION: Breakpoint !");
	// pas de loop{} — on continue l'exécution après un breakpoint
}

pub unsafe extern "C" fn handler_overflow() {
	println!("EXCEPTION: Overflow !");
	loop {}
}

pub unsafe extern "C" fn handler_bound_range() {
	println!("EXCEPTION: Bound Range - acces hors bornes !");
	loop {}
}

pub unsafe extern "C" fn handler_invalid_opcode() {
	println!("EXCEPTION: Instruction invalide !");
	loop {}
}

pub unsafe extern "C" fn handler_double_fault() {
	println!("EXCEPTION: Double Fault — kernel mort !");
	loop {}
}

pub unsafe extern "C" fn handler_general_protection() {
	println!("EXCEPTION: General Protection Fault !");
	loop {}
}

pub unsafe extern "C" fn handler_page_fault() {
	println!("EXCEPTION: Page Fault !");
	loop {}
}

// ── Vrais handlers (appelés depuis les stubs) ──
pub extern "C" fn irq_timer_inner() {
	timer::tick();
    unsafe { pic::eoi(0); }
}

pub extern "C" fn irq_keyboard_inner() {									
    let scancode = unsafe { pic::inb(0x60) };
	unsafe { pic::eoi(1); }	
	keyboard::on_key(scancode);
}

pub extern "C" fn irq_cascade_inner() {
    unsafe { pic::eoi(2); }
}

pub extern "C" fn irq_mouse_inner() {
    let byte = unsafe { pic::inb(0x60) };
	unsafe { pic::eoi(12); }
	mouse::on_irq(byte);
}

// ── Stubs naked — sauvegarde registres + iretq ──
#[unsafe(naked)]
pub unsafe extern "C" fn irq_timer_stub() {
    naked_asm!(
        "push rax","push rbx","push rcx","push rdx",
        "push rsi","push rdi","push rbp",
        "push r8","push r9","push r10","push r11",
        "push r12","push r13","push r14","push r15",
        "call {f}",
        "pop r15","pop r14","pop r13","pop r12",
        "pop r11","pop r10","pop r9","pop r8",
        "pop rbp","pop rdi","pop rsi","pop rdx",
        "pop rcx","pop rbx","pop rax",
        "iretq",
        f = sym irq_timer_inner,
    );
}

#[unsafe(naked)]
pub unsafe extern "C" fn irq_keyboard_stub() {
    naked_asm!(
        "push rax","push rbx","push rcx","push rdx",
        "push rsi","push rdi","push rbp",
        "push r8","push r9","push r10","push r11",
        "push r12","push r13","push r14","push r15",
        "call {f}",
        "pop r15","pop r14","pop r13","pop r12",
        "pop r11","pop r10","pop r9","pop r8",
        "pop rbp","pop rdi","pop rsi","pop rdx",
        "pop rcx","pop rbx","pop rax",
        "iretq",
        f = sym irq_keyboard_inner,
    );
}

#[unsafe(naked)]
pub unsafe extern "C" fn irq_cascade_stub() {
    naked_asm!(
        "push rax","push rbx","push rcx","push rdx",
        "push rsi","push rdi","push rbp",
        "push r8","push r9","push r10","push r11",
        "push r12","push r13","push r14","push r15",
        "call {f}",
        "pop r15","pop r14","pop r13","pop r12",
        "pop r11","pop r10","pop r9","pop r8",
        "pop rbp","pop rdi","pop rsi","pop rdx",
        "pop rcx","pop rbx","pop rax",
        "iretq",
        f = sym irq_cascade_inner,
    );
}

#[unsafe(naked)]
pub unsafe extern "C" fn irq_mouse_stub() {
    naked_asm!(
        "push rax","push rbx","push rcx","push rdx",
        "push rsi","push rdi","push rbp",
        "push r8","push r9","push r10","push r11",
        "push r12","push r13","push r14","push r15",
        "call {f}",
        "pop r15","pop r14","pop r13","pop r12",
        "pop r11","pop r10","pop r9","pop r8",
        "pop rbp","pop rdi","pop rsi","pop rdx",
        "pop rcx","pop rbx","pop rax",
        "iretq",
        f = sym irq_mouse_inner,
    );
}

// Initialise l'IDT avec tous les handlers
pub fn init() {
	unsafe {
		let idt = core::ptr::addr_of_mut!(IDT);
		(*idt).set_handler(DIVIDE_ERROR,	handler_divide_error);
		(*idt).set_handler(DEBUG,			handler_debug);
		(*idt).set_handler(NMI,				handler_nmi);
		(*idt).set_handler(BREAKPOINT,		handler_breakpoint);
		(*idt).set_handler(OVERFLOW,		handler_overflow);
		(*idt).set_handler(BOUND_RANGE,		handler_bound_range);
		(*idt).set_handler(INVALID_OPCODE,	handler_invalid_opcode);
		(*idt).set_handler(DOUBLE_FAULT,	handler_double_fault);
		(*idt).set_handler(GENERAL_PROTECT,	handler_general_protection);
		(*idt).set_handler(PAGE_FAULT,		handler_page_fault);
		(*idt).set_handler(IRQ_TIMER,   	irq_timer_stub);
		(*idt).set_handler(IRQ_KEYBOARD, 	irq_keyboard_stub);
		(*idt).set_handler(IRQ_CASCADE, 	irq_cascade_stub);
		(*idt).set_handler(IRQ_MOUSE,   	irq_mouse_stub);
		(*idt).load();
	}
}

// IDT globale — une seule instance pour tout le kernel
pub static mut IDT: Idt = Idt::new();