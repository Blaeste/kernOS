/// Multitâche coopératif — scheduler round-robin

use core::ptr::{addr_of_mut, addr_of};

/// Taille de la pile pour chaque tâche (8 Ko)
const STACK_SIZE:   usize = 8192;

/// Nombre max de tâches simultanées
pub const MAX_TASKS:    usize = 8;

/// Nombre max de fichiers ouverts par tâche (0=stdin, 1=stdout, 2=stderr, 3..7=KFS)
pub const MAX_FDS:      usize = 8;

/// Entrée de la table des descipteurs de fichiers d'une tâche.
/// fd 0/1/2 sont réservés (stdin/stdout/stderr) et ne sont pas dans ce tableau
/// ils0 sont gérés directement par les handlers syscall
/// fd 3..7 pointent vers des fichiers KFS ouverts.
pub struct FileDesc {
    pub active:     bool,
    pub kind:       u8,         // 0=KFS 1=pipe_read 2=pipe_write
    pub name:       [u8; 28],   // nom KFS (NAME_MAX = 28)
    pub pos:        usize,      // position de lecture courante
}

/// Une tâche = une pile + un pointeur de pile sauvegardé
/// Quand on switch de tâche, on sauvegarde RSP (Register Stack Pointer) ici
/// et on restaure le RSP de la prochaine tâche.
// align(16) : stack_top = &stack[0] + STACK_SIZE doit être ≡ 0 (mod 16)
// pour que le premier iretq vers la tâche place RSP correctement.
// stack en premier (offset 0) → &stack[0] = adresse de la Task = 16-byte alignée.
#[repr(C, align(16))]
pub struct Task {
    pub stack:          [u8; STACK_SIZE],       // La pile (en premier → offset 0, alignée 16)
    pub rsp:            u64,                    // pointeur de pile sauvegardé
    pub active:         bool,
    pub cr3:            u64,
    pub fds:            [FileDesc; MAX_FDS],
    pub waiting_for:    usize,                  // usize::MAX = pas en attente
    pub stdin_wait:     bool,
}

/// Tableau de toutes les tâches (slots fixes)
pub static mut TASKS: [Task; MAX_TASKS] = [const { Task {
    stack:          [0; STACK_SIZE],
    rsp:            0,
    active:         false,
    cr3:            0,
    fds:            [const { FileDesc { active: false, kind: 0, name: [0; 28], pos: 0 } }; MAX_FDS],
    waiting_for:    usize::MAX,
    stdin_wait:     false,
} }; MAX_TASKS];

/// CR3 à charger lors du prochain context switch - écrit par schedule(),
/// lu par irq_timer_stub en assembleur.
pub static mut NEXT_CR3: u64 = 0;

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

/// Spawn user ring3
pub fn spawn_ring3(entry: u64, user_stack: u64, cr3: u64) {
    unsafe {
        let slot = {
            let tasks = &mut *addr_of_mut!(TASKS);
            let mut found = usize::MAX;
            let mut i = 1;
            while i < MAX_TASKS {
                if !tasks[i].active { found = i; break; }
                i += 1;
            }
            found
        };
        if slot == usize::MAX { return; }

        let tasks = &mut *addr_of_mut!(TASKS);
        let task = &mut tasks[slot];

        // Frame iretq écrit sur la pile KERNEL du slot (pas sur user_stack).
        // user_stack n'est que la valeur RSP stockée dans l'entrée iretq[18] :
        // iretq la chargera dans RSP au moment du saut en ring 3.
        // Ecrire sur al pile kernel (identité mappée) écite d'accèder à
        // une adresse virtuelle user qui n'est pas encore dans le CR3 courant.
        let kstack_top = task.stack.as_mut_ptr().add(STACK_SIZE) as u64;
        let ptr = (kstack_top - 160) as *mut u64;
        let mut j = 0;
        while j < 15 { ptr.add(j).write(0); j += 1; }
        ptr.add(15).write(entry);
        ptr.add(16).write(crate::gdt::USER_CODE as u64);
        ptr.add(17).write(0x202);
        ptr.add(18).write(user_stack);
        ptr.add(19).write(crate::gdt::USER_DATA as u64);

        // réinitialise le slot - peut être réutilisé après task_exit
        task.waiting_for = usize::MAX;
        for fd in task.fds.iter_mut() {
            fd.active = false;
        }

        task.rsp    = kstack_top - 160;
        task.cr3    = cr3;
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

            let t = &(*addr_of!(TASKS))[next];
            if t.active {

                // Runnable si : pas en attente OU la tâche attendue a terminé
                let wf = t.waiting_for;
                let runnable = (wf == usize::MAX || !(*addr_of!(TASKS))[wf].active) && !t.stdin_wait;
                if runnable { break; }
            }
        }

        // Met à jour l'index courant et retourne le RSP de la prochaine tâche
        *addr_of_mut!(CURRENT) = next;
        // Informe le stub quel CR3 charger lors du switch
        *addr_of_mut!(NEXT_CR3) = (*addr_of!(TASKS))[next].cr3;

        // Pile kernel de la prochaine tâche. Le CPU y bascule automatiquement
        // à chaque entrée en ring 0 depuis le ring 3 (IRQ timer ou int 0x80).
        // Sans mise à jour ici, toutes les tâches ring 3 partageraient la pile
        // fixée au boot et écraseraient mutuellement leur contexte sauvegardé
        // (voir gdt::set_kernel_stack).
        let kstack_top = (*addr_of_mut!(TASKS))[next].stack.as_mut_ptr().add(STACK_SIZE) as u64;
        crate::gdt::set_kernel_stack(kstack_top);
        (*addr_of!(TASKS))[next].rsp
    }
}

/// Termine la tâche courante
pub fn task_exit() {
    unsafe {
        let cur = *addr_of!(CURRENT);

        // Reveille toute tâche bloquée sur ce pid
        for i in 0..MAX_TASKS {

            if (*addr_of!(TASKS))[i].waiting_for == cur {

                (*addr_of_mut!(TASKS))[i].waiting_for = usize::MAX;
            }
        }

        // Si la tâche qui meurt attendait stdin, libère le clavier
        // (sinon il resterait routé vers le stdin d'un process mort).
        if crate::io::stdin::WAITING_TASK == cur {
            crate::io::stdin::WAITING_TASK = usize::MAX;
        }

        // Libère les frames du process (sauf le kernel qui partage le boot CR3).
        // Sûr ici : aucune allocation entre free_cr3() et le int 0x20 qui
        // bascule sur le CR3 kernel, donc les frames libérés ne sont pas réutilisés
        // tant qu'ils sont encore l'espace d'adressage actif.
        let dying_cr3 = (*addr_of!(TASKS))[cur].cr3;
        let kernel_cr3 = (*addr_of!(TASKS))[0].cr3;
        if dying_cr3 != 0 && dying_cr3 != kernel_cr3 {
            crate::mem::paging::free_cr3(dying_cr3);
        }

        (*addr_of_mut!(TASKS))[cur].active = false;
        // Force un reschedule immédiat
        core::arch::asm!("int 0x20", options(nostack));
    }
    loop {} // jamais atteint
}
