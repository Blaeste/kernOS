/*
    #![no_std]
    Le #! veut dire : "ceci s'applique à tout le fichier".
    no_std dit à Rust : n'inclus pas la bibliothèque standard (std).
    Pourquoi ? Parce que std utilise des fonctions de l'OS (mémoire, fichiers, threads...). Mais nous sommes l'OS — personne ne peut nous aider.
    On garde uniquement core : les types de base (nombres, slices...) qui ne dépendent de rien.

    #![no_main]
    Normalement, Rust s'attend à une fonction fn main(). Mais ce main est appelé par le runtime de l'OS (qui prépare la pile, les arguments...).
    On n'a pas d'OS pour faire ça. Donc on dit : je gère moi-même mon point d'entrée.
*/
#![no_std]
#![no_main]

extern crate alloc;

// Import modules
mod klib;
mod io;
mod drivers;
mod mem;
mod interrupt;
mod multiboot;
mod shell;
mod task;
mod fs;

#[global_allocator]
static ALLOCATOR: mem::allocator::BumpAllocator = mem::allocator::BumpAllocator;

/// Adresse Multiboot2 info — stockée au boot pour utilisation ultérieure (shell, etc.)
pub static mut MULTIBOOT_ADDR: u64 = 0;

/* 
    Macros d'affichage — remplacent std::print!/println! qu'on n'a pas en no_std
    $($arg:tt)* → capture n'importe quels arguments (texte, variables, expressions)
    core::fmt::Write → trait qui donne accès au formatage {} via write!()
    addr_of_mut! → pointeur brut obligatoire en Rust edition 2024 (pas de référence vers static mut)
*/
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        unsafe { write!(&mut *core::ptr::addr_of_mut!($crate::io::vga::WRITER), $($arg)*).unwrap() }
    });
}

/*
    println! — comme print! mais ajoute \n à la fin
    format_args! → construit les arguments de formatage sans allocation mémoire
*/
#[macro_export]
macro_rules! println {
    ()              => ($crate::print!("\n"));
    ($($arg:tt)*)   => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/*
    use core::panic::PanicInfo;
    use = importer quelque chose pour pouvoir l'utiliser.
    core = la mini-bibliothèque disponible sans OS.
    PanicInfo = un type qui décrit une erreur critique (le message, le fichier, la ligne).
    On l'importe car notre fonction panic en a besoin.
*/
use core::panic::PanicInfo;

/*
    #[panic_handler]
    Un #[...] s'appelle un attribut — c'est une instruction pour le compilateur.
    Celui-ci dit : "la fonction qui suit est mon gestionnaire de panique".
    En no_std, Rust exige qu'on en définisse un, sinon il refuse de compiler.

    fn panic(_info: &PanicInfo) -> !
    _info = le paramètre (le _ dit au compilateur "je sais qu'il existe mais je ne l'utilise pas").
    &PanicInfo = une référence (pointeur sûr) vers les infos de l'erreur.
    -> ! = le type de retour est ! = "cette fonction ne revient jamais". Le ! s'appelle le type "never". Logique : si le kernel panique, on ne peut pas continuer.
*/
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// fn task_test() {
//     loop {
//         crate::println!("[TACHE 1] salut !");
//         for _ in 0..10_000_000 { core::hint::spin_loop(); } //  CPU attends un peu sans rien faire. boucle de temporisation
//     }
// }

// fn task_test2() {
//     loop {
//         crate::println!("[TACHE 2] salut !");
//         for _ in 0..10_000_000 { core::hint::spin_loop(); } //  CPU attends un peu sans rien faire. boucle de temporisation
//     }
// }



/*
    #[unsafe(no_mangle)]
    Par défaut, Rust renomme les fonctions en interne (ça s'appelle le "mangling") pour éviter les conflits.
    _start deviendrait quelque chose comme _ZN6kernos6_startE.
    no_mangle dit : garde exactement le nom _start.
    Pourquoi ? Parce que le bootloader cherchera littéralement le symbole _start pour démarrer.

    pub extern "C" fn _start() -> !
    pub = public, visible depuis l'extérieur du programme.
    extern "C" = utilise la convention d'appel C (la façon dont les arguments et le retour sont gérés en assembleur). Le bootloader s'y attend.
    _start = le nom du point d'entrée, c'est une convention universelle en bare-metal.
    -> ! = encore le type "never" — le kernel ne doit jamais retourner. S'il retourne, il n'y a personne pour récupérer le résultat.
*/
#[unsafe(no_mangle)]
pub extern "C" fn _start(multiboot_addr: u64) -> ! {
    // Sauvegarde l'adresse Multiboot2 pour que le shell puisse y accéder
    unsafe { *core::ptr::addr_of_mut!(MULTIBOOT_ADDR) = multiboot_addr; }

    // addr_of_mut! → pointeur brut vers WRITER sans créer de référence (obligatoire en Rust edition 2024)
    // (*...) → déréférencement du pointeur pour accéder aux méthodes 
    unsafe { (*core::ptr::addr_of_mut!(io::vga::WRITER)).clear(); }         // efface l'écran et remet le curseur à (0,0)
    unsafe { (*core::ptr::addr_of_mut!(io::vga::WRITER)).color = 0x0B; }    // couleur : cyan clair sur noir

    interrupt::init();  // charge l'IDT
    drivers::pic::init();        // remapping + init PIC
    drivers::timer::init(100);   // 100 Hz = 10ms par tick
    unsafe { drivers::mouse::init(); }
    mem::frame::init(multiboot_addr);

     // Initialise le framebuffer si GRUB en a fourni un
    if let Some(fb) = multiboot::find_framebuffer(multiboot_addr) {
        io::framebuffer::init(fb);
        io::framebuffer::clear(0x003366);        // fond bleu foncé
        io::framebuffer::draw_string(100, 50, "KernOS v0.1", 0xFFFFFF, 0x003366);
        io::framebuffer::fill_rect(100, 100, 200, 150, 0xFF0000);  // rectangle rouge
        io::framebuffer::fill_rect(350, 200, 150, 150, 0x00FF00);  // rectangle vert
        io::framebuffer::fill_rect(550, 300, 100, 100, 0x0000FF);  // rectangle bleu
    }

    // Enregistre le kernel comme tâche 0
    unsafe {
        (*core::ptr::addr_of_mut!(task::scheduler::TASKS))[0].active = true;
    }

    // Active les interruptions
    unsafe { core::arch::asm!("sti", options(nostack)); }

    // test exception devide 0
    //// unsafe { core::arch::asm!("div rax", in("rax") 0u64, options(nostack)); }

    // Tache de test : affiche un message en boucle
    // task::scheduler::spawn(task_test);
    // task::scheduler::spawn(task_test2);

    println!("KernOS v0.1");

    shell::prompt();

    loop {}
}