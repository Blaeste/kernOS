/*
    ══════════════════════════════════════════════════════════
    STDIN — Buffer clavier ligne pour les processus userspace
    ══════════════════════════════════════════════════════════

    POURQUOI STDIN ?
    ────────────────
    Sans stdin, un programme userspace ne peut pas lire de saisie
    clavier : sys_read(fd=0) retournait 0 immédiatement.
    Ce module implémente un buffer ligne partagé entre l'IRQ clavier
    et les tâches qui bloquent sur read(0).

    FONCTIONNEMENT
    ──────────────
    1. Un processus appelle sys_read(fd=0, buf, len).
    2. Si le buffer est vide → tâche marquée stdin_wait=true, yield.
    3. L'IRQ clavier écrit chaque caractère dans BUF + echo terminal.
    4. Sur '\n' → wake_waiting() remet stdin_wait=false.
    5. Le scheduler reprend la tâche → elle lit BUF et retourne.

    BUFFER CIRCULAIRE
    ─────────────────
    ┌──────────────────────────────┐
    │ BUF [0 .. CAP)               │
    │      RPOS──────►WPOS         │
    │      (lecture)   (écriture)  │
    └──────────────────────────────┘
    LEN = octets disponibles entre RPOS et WPOS.

    LINE-BUFFERED
    ─────────────
    La tâche n'est réveillée qu'après le '\n' (Enter).
    Les caractères sont accumulés avec echo à l'écran.
    Backspace retire le dernier octet du buffer et efface l'echo.
*/

use core::ptr::{addr_of, addr_of_mut};

// ── Constantes ────────────────────────────────────────────────────────────

/// Capacite du buffer stdin (en octets)
const CAP:			usize = 256;


// ── Buffer circulaire ─────────────────────────────────────────────────────

static mut BUF:		[u8; CAP] = [0; CAP];
static mut RPOS:	usize = 0;			// position de lecture courante
static mut WPOS:	usize = 0;			// position d'ecriture courante
static mut LEN:		usize = 0;			// octets disponible


// ── État d'attente ────────────────────────────────────────────────────────

/// Index dee la tache bloquee sur read(0). usize::MAX = personne n'attend.
pub static mut WAITING_TASK:	usize = usize::MAX;


// ── API publique ──────────────────────────────────────────────────────────

/// Pousse un octet dans le buffer (appele depuis IRQ clavier).
/// Silencieux si le buffer est plein.
pub fn push(ch: u8) {

	unsafe {
		if *addr_of!(LEN) < CAP {

			BUF[*addr_of!(WPOS)] = ch;
			WPOS = (*addr_of!(WPOS) + 1) % CAP;
			LEN += 1;
		}
	}
}

/// Retire le dernier octet ecrit - gere le backspace clavier.
/// Retourne true si un octet a bien ete supprime.
pub fn pop_last() -> bool {

	unsafe {
		if *addr_of!(LEN) > 0 {

			WPOS = (*addr_of!(WPOS) + CAP - 1) % CAP;
			LEN -= 1;
			true
		} else {
			false
		}
	}
}

/// Lit jusqu'a 'dst.len()' octets depuis le buffer.
/// Avance RPOS et decrement LEN. Retourne le nombre d'octers lus.
pub fn read(dst: &mut [u8]) -> usize {

	unsafe {
		let n = dst.len().min(*addr_of!(LEN));
		for i in 0..n {

			dst[i] = BUF[(*addr_of!(RPOS) + i) % CAP];
		}
		RPOS = (*addr_of!(RPOS) + n) % CAP;
		LEN -= n;
		n
	}
}

/// Retourne le nombre d'octets disponible en lecture.
pub fn available() -> usize {

	unsafe { *addr_of!(LEN) }
}

/// Reveille la tache bloque sur stdin.
/// Appele par le clavier apres la reception de '\n'.
pub fn wake_waiting() {

	unsafe {
		let  t = *addr_of!(WAITING_TASK);
		if t != usize::MAX {

			(*addr_of_mut!(crate::task::scheduler::TASKS))[t].stdin_wait = false;
			WAITING_TASK = usize::MAX;
		}
	}
}
