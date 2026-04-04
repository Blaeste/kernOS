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

// Import modules
mod vga;

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
pub extern "C" fn _start() -> ! {

    vga::clear();
    vga::center(b"Welcome to KernOS", 12, 0x0B);

    loop {}
}