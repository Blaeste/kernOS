/// Multitâche coopératif — scheduler round-robin

use core::ptr::{addr_of_mut, addr_of};

/// Taille de la pile pour chaque tâche (8 Ko)
const STACK_SIZE:   usize = 8192;

/// Nombre max de tâches simultanées
pub const MAX_TASKS:    usize = 8;

/// Une tâche = une pile + un pointeur de pile sauvegardé
/// Quand on switch de tâche, on sauvegarde RSP (Register Stack Pointer) ici
/// et on restaure le RSP de la prochaine tâche.
#[repr(C)]
pub struct Task {
    pub rsp:    u64,               // pointeur de pile sauvegardé
    pub stack:  [u8; STACK_SIZE],  // La pile elle-même
    pub active: bool,
}

/// Tableau de toutes les tâches (slots fixes)
pub static mut TASKS: [Task; MAX_TASKS] = [const { Task {
    rsp:    0,
    stack:  [0; STACK_SIZE],
    active: false,
} }; MAX_TASKS];

/// Index de la tâche en cours d'exécution
pub static mut CURRENT: usize = 0;

/// Crée une nouvelle tâche qui exécutera la fonction 'entry'.
/// Trouve un slot libre, prépare sa pile et l'active.
pub fn spawn(entry: fn()) {
    // Cherche un slot libre dans le tableau
    let slot = unsafe {
        let mut s = usize::MAX;
        for i in 0..MAX_TASKS {
            if !(*addr_of!(TASKS))[i].active {
                s = i;
                break;
            }
        }
        s
    };

    if slot == usize::MAX {
        crate::println!("spawn: plus de slots !");
        return;
    }

    unsafe {
        let task = &mut (*addr_of_mut!(TASKS))[slot];

        // Le sommet de la pile = adresse du début du tableau + STACK_SIZE
        // (la pile grandit vers le bas en x86, donc on commence en haut)
        let stack_top = task.stack.as_mut_ptr().add(STACK_SIZE) as u64;

        // On prépare la pile comme si la tache avait été interrompue :
        // le context switch va faire 15 pop + iretq, donc on empile
        // 15 registres à 0 + 5 valeurs iretq = 20 valeurs
        let ptr = (stack_top - 160) as *mut u64;    // 20 valeurs x 8 octets
        for j in 0..15 {
            ptr.add(j).write(0);                    // 15 registres = 0
        }
        ptr.add(15).write(entry as u64);            // RIP = adresse de la fonction
        ptr.add(16).write(crate::gdt::KERNEL_CODE as u64);  // CS = segment de code 64-bit (GDT)
        ptr.add(17).write(0x202);                   // RFLAGS = interruptions activées (IF=1)
        ptr.add(18).write(stack_top);               // RSP = sommet de la pile
        ptr.add(19).write(0x00);                    // SS = segment de données

        task.rsp = stack_top - 160;                 // RSP pointe vers le haut de ce qu'on a empilé
        task.active = true;
    }
}

/// Appelée par le timer (100Hz). Choisit la prochaine tâche active
/// et fait le context switch.
/// old_rsp = le RSP de la tâche qu'on vient d'interrompre
/// Retourne le RSP d ela prochaine tâche à executer
pub fn schedule(old_rsp: u64) -> u64 {
    unsafe {
        // Sauvegarde le RSP de la tâche courrante
        let cur = *addr_of!(CURRENT);
        (*addr_of_mut!(TASKS))[cur].rsp = old_rsp;

        // Round-robin : cherche la prochaine tâche active
        let mut next = cur;
        for _ in 0..MAX_TASKS {
            next = (next + 1) % MAX_TASKS;
            if (*addr_of!(TASKS))[next].active {
                break;
            }
        }

        // Met à jour l'index courant et retourne le RSP de la prochaine tâche
        *addr_of_mut!(CURRENT) = next;
        (*addr_of!(TASKS))[next].rsp
    }
}

/// Termine la tâche courante
pub fn task_exit() {
    unsafe {
        let cur = *addr_of!(CURRENT);
        (*addr_of_mut!(TASKS))[cur].active = false;
        // Force un reschedule immédiat
        core::arch::asm!("int 0x20", options(nostack));
    }
    loop {} // jamais atteint
}