/*
    ══════════════════════════════════════════════════════════
    ELF64 LOADER — Chargement de binaires en mémoire user
    ══════════════════════════════════════════════════════════

    POURQUOI UN ELF LOADER ?
    ────────────────────────
    Sans loader, tout le code userspace doit être compilé directement
    dans le kernel et lancé via un pointeur de fonction. C'est ce que
    fait userspace.rs aujourd'hui avec user_task().
    Un ELF loader permet de charger un programme arbitraire depuis la
    mémoire (ou plus tard le disque), de le placer aux bonnes adresses
    virtuelles, et de le lancer en ring 3 de façon autonome.

    QU'EST-CE QU'UN ELF ?
    ──────────────────────
    ELF (Executable and Linkable Format) est le format binaire standard
    sous Linux/x86_64. Un fichier ELF contient :

    ┌──────────────────────────────────────────────┐
    │  ELF Header (64 octets)                      │
    │  → magic, architecture, point d'entrée,      │
    │    offset vers les Program Headers           │
    ├──────────────────────────────────────────────┤
    │  Program Headers  (56 octets × N)            │
    │  → chaque segment : où le mettre en mémoire, │
    │    quoi copier depuis le fichier             │
    ├──────────────────────────────────────────────┤
    │  Segment .text  (code exécutable)            │
    │  Segment .data  (données initialisées)       │
    │  Segment .bss   (données non initialisées)   │
    └──────────────────────────────────────────────┘

    COMMENT ON CHARGE UN SEGMENT ?
    ───────────────────────────────
    Chaque segment PT_LOAD décrit :
      p_offset → où sont les octets dans le fichier ELF
      p_vaddr  → à quelle adresse virtuelle les mettre en RAM
      p_filesz → combien d'octets copier depuis le fichier
      p_memsz  → combien d'octets réserver en RAM
                 (si memsz > filesz, la différence = BSS → zéro)

    On traite ça page par page (4 Ko) :
      1. frame::alloc()      → obtenir un frame physique libre
      2. paging::map_user()  → le relier à l'adresse virtuelle p_vaddr
      3. copier les octets   → memcpy depuis le slice ELF vers le frame
      4. zero BSS            → mettre à 0 le reste si memsz > filesz

    CE MODULE EXPOSE :
    ──────────────────
    - load(bytes) → parse + charge un ELF64, retourne le point d'entrée
*/

use crate::mem::{frame, paging};

// ── Structures ELF64 ─────────────────────────────────────────────────────
//
// Ces structs mappent directement le format binaire ELF64.
// #[repr(C)] garantit que Rust ne réordonne pas les champs.
// On les lit avec un cast de pointeur brut sur le slice d'octets.

/// Header principal d'un binaire ELF64 — toujours les 64 premiers octets.
///
/// Les champs importants pour le loader :
///   e_ident[0..4]  → magic 0x7f 'E' 'L' 'F' (vérifie que c'est bien un ELF)
///   e_entry        → adresse virtuelle de _start (là où on saute en ring 3)
///   e_phoff        → offset du tableau de Program Headers dans le fichier
///   e_phentsize    → taille d'une entrée Program Header (toujours 56 en ELF64)
///   e_phnum        → nombre de Program Headers à lire
#[repr(C)]
pub struct Elf64Ehdr {
    pub e_ident:        [u8; 16],
    pub e_type:         u16,
    pub e_machine:      u16,
    pub e_version:      u32,
    pub e_entry:        u64,
    pub e_phoff:        u64,
    pub e_shoff:        u64,
    pub e_flags:        u32,
    pub e_ehsize:       u16,
    pub e_phentsize:    u16,
    pub e_phnum:        u16,
    pub e_shentsize:    u16,
    pub e_shnum:        u16,
    pub e_shstrndx:     u16,
}

/// Program Header ELF64 - décrit un segment à charger en mémoire.
///
/// Le loader ne traite que les segments de type PT_LOAD (p_type == 1)
/// Les autres (PT_DYNAMIC, PT_NOTE, etc.) sont ignorés.
///
/// p_filesz ≤ p_memsz toujours :
///   - si p_filesz == p_memsz → copie pure, pas de BSS
///   - si p_filesz <  p_memsz → copie p_filesz octets, puis zéro jusqu'à p_memsz
#[repr(C)]
pub struct Elf64Phdr {
    pub p_type:     u32,    // type de segment — PT_LOAD = 1
    pub p_flags:    u32,    // permissions — PF_X=1 (exec), PF_W=2 (write), PF_R=4 (read)
    pub p_offset:   u64,    // offset du segment dans le fichier ELF
    pub p_vaddr:    u64,    // adresse virtuelle cible en mémoire
    pub p_paddr:    u64,    // adresse physique — ignorée en userspace
    pub p_filesz:   u64,    // octets à copier depuis le fichier
    pub p_memsz:    u64,    // octets à réserver en mémoire
    pub p_align:    u64,    // alignement requis (puissance de 2, en général 0x1000)
}

// ── Constantes ───────────────────────────────────────────────────────────

/// Type de segment à charger - les autres types sont ignorés par le loader.
const PT_LOAD: u32 = 1;

/// Les 4 premiers octets de tout fichier ELF valide.
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

// ── Loader ───────────────────────────────────────────────────────────────

/// Parse et charge un ELF64 en memoire user. Retourne le point d'entrée ou None.
///
/// 'bytes' est le contenu brut du fichier ELF (depuis include_bytes! ou KFS).
///
/// Retourne None si :
///     - le fichier est trop court ou le magic est absent
///     - un Program Header dépasse la taille du fichier
///     - frame::alloc() échoue (plus de RAM dispo)
pub fn load(bytes: &[u8], cr3: u64) -> Option<u64> {
    // ── Validation de base ───────────────────────────────
    // 64 octets minimum pour lire le header complet..
    // le magic 0x7fELF identifie un fichier ELF - sans ça on refuse.
    if bytes.len() < 64 { return None; }
    if bytes[0..4] != ELF_MAGIC { return None; }

    // Cast les premiers octets en référence vers Elf64Ehdr.
    // Sûr ici : on a vérifié la taille minimale juste au-dessus
    let ehdr = unsafe { &*(bytes.as_ptr() as *const Elf64Ehdr) };

    let phoff   = ehdr.e_phoff      as usize;
    let phnum   = ehdr.e_phnum      as usize;
    let phsz    = ehdr.e_phentsize  as usize;

    // ── Parcours des Program Headers ─────────────────────
    for i in 0..phnum {
        let off = phoff + i * phsz;

        // Verifie que ce Program Header est entièrement dans le fichier
        if off + phsz > bytes.len() { return None; }

        let phdr = unsafe { &*(bytes.as_ptr().add(off) as *const Elf64Phdr) };

        // On ne traite que PT_LOAD - le reste (métadonnées, debug, etc.) est ignoré
        if phdr.p_type != PT_LOAD { continue; }

        let src_off = phdr.p_offset     as usize;
        let filesz  = phdr.p_filesz     as usize;
        let memsz   = phdr.p_memsz      as usize;
        let vaddr   = phdr.p_vaddr;

        // ── Chargement page par page ──────────────────────
        // Un segment peut couvrir plusieur pages (ex: 12 Ko = 3 frames).
        // On avance de 4096 en 4096 jusqu'à avoir couvert tout memsz.
        let mut done = 0usize;
        while done < memsz {
            // Si la page est déjà mappée (autre segment sur la même page),
            // on réutilise le frame existant au lieu d'en allouer un nouveau.
            let vpage = (vaddr as usize + done) & !0xFFF;
            let frame_phys = match paging::translate_in(cr3, vpage as u64) {
                Some(phys) => phys & !0xFFF,
                None => {
                    let f = frame::alloc()?;
                    paging::map_user_at(cr3, vpage as u64, f);
                    f
                }
            };

            // La première page peut démarrer en millieu de frame si p_vaddr
            // n'est pas aligné sur 4 Ko (ex: 0x401008 → page_off = 8).
            // Les pages suivantes démarrent toujours à l'offset 0.
            let page_off = if done == 0 { vaddr as usize & 0xFFF } else { 0 };

            // Nombre d'octets du fichier à copier dans cette page
            let copy_len = (4096 - page_off).min(filesz.saturating_sub(done));

            unsafe {
                let dst = (frame_phys + page_off as u64) as *mut u8;

                // ── Copie des octets du fichier ───────────
                for j in 0..copy_len {
                    dst.add(j).write(bytes[src_off + done + j]);
                }

                // ── Zero du BSS ───────────────────────────
                // Les octets entre filesz et memsz (section .bss) doivent
                // être à zéro — le programme suppose que c'est le cas.
                for j in copy_len..(4096 - page_off) {
                    dst.add(j).write(0);
                }
            }

            done += 4096;
        }
    }

    // Retourne e_entry : l'adresse virtuelle de _start
    Some(ehdr.e_entry)
}