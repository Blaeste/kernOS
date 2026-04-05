/*
    ══════════════════════════════════════════════════════════
    PAGINATION x86_64 — Tables de pages 4 niveaux
    ══════════════════════════════════════════════════════════

    POURQUOI LA PAGINATION ?
    ────────────────────────
    Sans pagination, chaque programme accède directement à la RAM physique.
    Problème : si deux programmes utilisent l'adresse 0x1000, ils s'écrasent
    mutuellement. La pagination crée une "illusion" : chaque programme croit
    avoir toute la mémoire pour lui, mais le CPU traduit ses adresses
    virtuelles en adresses physiques réelles via des tables.

    COMMENT ÇA MARCHE EN x86_64 ?
    ──────────────────────────────
    Le CPU découpe chaque adresse virtuelle de 48 bits en 5 morceaux :

    Adresse virtuelle (48 bits utiles) :
    ┌──────────┬──────────┬──────────┬──────────┬──────────────┐
    │  PML4    │  PDPT    │   PD     │   PT     │   Offset     │
    │ bits     │ bits     │ bits     │ bits     │   bits       │
    │ 47..39   │ 38..30   │ 29..21   │ 20..12   │  11..0       │
    │ (9 bits) │ (9 bits) │ (9 bits) │ (9 bits) │ (12 bits)    │
    └──────────┴──────────┴──────────┴──────────┴──────────────┘
      index      index      index      index     position dans
      dans P4    dans P3    dans P2    dans P1    la page (0-4095)

    9 bits = valeurs 0 à 511 → chaque table a 512 entrées.
    512 entrées × 8 octets = 4096 octets = exactement 1 frame (1 page).

    Le CPU suit la chaîne :
    CR3 → PML4[i4] → PDPT[i3] → PD[i2] → PT[i1] → page physique + offset

    Exemple concret avec l'adresse virtuelle 0x0040_1000 :
      i4 = 0, i3 = 0, i2 = 2, i1 = 1, offset = 0x000
      → PML4[0] → PDPT[0] → PD[2] → PT[1] → le frame physique mappé là

    FORMAT D'UNE ENTRÉE DE TABLE (64 bits / u64) :
    ┌────────────────────────────────────────────────────────┐
    │ Bits 63..52 : flags avancés (NX, etc.)                │
    │ Bits 51..12 : adresse physique de la table/page       │
    │               (alignée sur 4 Ko, donc bits 0-11 = 0)  │
    │ Bits 11..8  : ignorés par le CPU (libre pour l'OS)    │
    │ Bit 7       : HUGE PAGE (page 2 Mo ou 1 Go)           │
    │ Bits 6..2   : autres flags (dirty, accessed, etc.)    │
    │ Bit 1       : WRITABLE (écriture autorisée)           │
    │ Bit 0       : PRESENT (la page existe en RAM)         │
    └────────────────────────────────────────────────────────┘

    IDENTITY MAP (ce qu'a fait boot.asm) :
    ───────────────────────────────────────
    Au boot, on a créé un mapping simple : adresse virtuelle = adresse physique
    pour le 1er Go, avec des "huge pages" de 2 Mo (bit 7 activé dans PD).
    Ça veut dire que 0x1000 virtuel = 0x1000 physique.
    C'est crucial : nos tables de pages sont en RAM physique, et on y accède
    par des adresses virtuelles. Si virt != phys, on ne pourrait pas les lire !

    CE MODULE AJOUTE :
    ──────────────────
    - map(virt, phys)    → crée un mapping page 4 Ko (au-delà du 1er Go)
    - unmap(virt)        → supprime un mapping
    - translate(virt)    → retrouve l'adresse physique d'une adresse virtuelle
*/

use crate::mem::frame;

// ── Flags des entrées (bits du u64) ──────────────────────────────────────
//
// Chaque entrée dans une table de pages est un u64.
// Les bits bas contiennent des flags, les bits hauts l'adresse physique.

/// Bit 0 — PRESENT : si ce bit est à 0, le CPU considère que la page
/// n'existe pas et déclenche un "Page Fault" (exception #14) si on y accède.
const PRESENT: u64 = 1 << 0;

/// Bit 1 — WRITABLE : si ce bit est à 0, la page est en lecture seule.
/// Toute écriture déclenche un Page Fault. Utile pour protéger le code kernel.
const WRITABLE: u64 = 1 << 1;

/// Bit 7 — HUGE PAGE : change la signification de l'entrée.
/// Dans PD (niveau 2) : la page fait 2 Mo au lieu de 4 Ko (pas de PT en dessous).
/// Dans PDPT (niveau 3) : la page fait 1 Go (pas de PD ni PT en dessous).
/// C'est ce que boot.asm utilise pour mapper le 1er Go rapidement.
const HUGE_PAGE: u64 = 1 << 7;

/// Masque pour extraire l'adresse physique d'une entrée.
/// Les bits 12 à 51 contiennent l'adresse (alignée sur 4 Ko).
/// Les bits 0-11 sont les flags, les bits 52-63 sont d'autres flags.
/// 0x000F_FFFF_FFFF_F000 = tous les bits de 12 à 51 à 1, le reste à 0.
const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

// ── Accès aux registres CPU ─────────────────────────────────────────────

/// Lit le registre CR3 du processeur.
///
/// CR3 (Control Register 3) est LE registre de la pagination :
/// il contient l'adresse physique de la PML4 (la table racine).
/// Quand le CPU doit traduire une adresse, il commence par lire CR3
/// pour trouver la PML4, puis descend les 4 niveaux.
///
/// `mov {}, cr3` → copie la valeur de CR3 dans un registre général.
/// `out(reg) cr3` → Rust met le résultat dans la variable `cr3`.
fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe { core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack)); }
    cr3
}

/// Invalide une entrée du TLB pour une adresse donnée.
///
/// Le TLB (Translation Lookaside Buffer) est un cache interne du CPU
/// qui mémorise les traductions virt → phys récentes pour aller plus vite.
/// Problème : si on modifie une table de pages, le TLB a encore l'ancienne
/// traduction en cache ! Il faut lui dire "oublie cette adresse".
///
/// `invlpg [addr]` = "invalidate page" — dit au CPU de vider le cache
/// TLB pour l'adresse `addr`. Sans ça, le CPU utiliserait l'ancien mapping
/// et on aurait des bugs mystérieux (accès à la mauvaise page physique).
fn invlpg(addr: u64) {
    unsafe { core::arch::asm!("invlpg [{}]", in(reg) addr, options(nostack)); }
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Convertit une adresse physique en référence vers une table de 512 entrées.
///
/// Une table de pages = un tableau de 512 u64 en mémoire (512 × 8 = 4096 octets).
/// On reçoit l'adresse physique de cette table, et on la convertit en pointeur Rust.
///
/// POURQUOI ÇA MARCHE ? Grâce à l'identity map du boot !
/// L'adresse physique 0x5000 est aussi l'adresse virtuelle 0x5000.
/// Donc on peut caster directement phys → *mut → &mut.
/// ⚠️ Si on désactivait l'identity map, cette fonction crasherait !
///
/// `&'static mut` = une référence mutable qui vit pour toujours.
/// `[u64; 512]` = tableau de 512 éléments u64 (les entrées de la table).
fn table_at(phys: u64) -> &'static mut [u64; 512] {
    unsafe { &mut *(phys as *mut [u64; 512]) }
}

/// Remplit un frame (4096 octets) avec des zéros.
///
/// Quand on alloue un nouveau frame pour une table de pages, la RAM
/// contient n'importe quoi (des restes d'anciennes données).
/// Si on ne met pas tout à zéro, le CPU verrait des entrées avec
/// le bit PRESENT à 1 par hasard → il suivrait des adresses bidon → crash.
///
/// On écrit octet par octet avec `ptr.add(i).write(0)` :
/// - `ptr.add(i)` → avance le pointeur de i octets
/// - `.write(0)` → écrit 0 à cette position (sans lire l'ancienne valeur)
fn zero_frame(addr: u64) {
    let ptr = addr as *mut u8;
    for i in 0..4096 {
        unsafe { ptr.add(i).write(0); }
    }
}

// ── Fonctions publiques ─────────────────────────────────────────────────

/// Mappe une page virtuelle de 4 Ko vers un frame physique.
///
/// Exemple : map(0xDEAD_0000, 0x0080_0000)
/// → l'adresse virtuelle 0xDEAD_0000 pointera vers le frame physique à 0x0080_0000
///
/// La fonction descend les 4 niveaux de tables :
///   PML4 → PDPT → PD → PT → on écrit l'adresse physique
///
/// Si une table intermédiaire n'existe pas encore (bit PRESENT = 0),
/// on alloue un nouveau frame avec frame::alloc(), on le met à zéro,
/// et on l'inscrit dans l'entrée parent avec les flags PRESENT | WRITABLE.
///
/// Si on tombe sur une huge page 2 Mo (dans PD), on ne touche à rien :
/// cette zone est déjà mappée par le boot, la casser ferait tout planter.
pub fn map(virt: u64, phys: u64) {
    // Récupère la PML4 (table racine) depuis CR3
    // & ADDR_MASK → enlève les flags de CR3, ne garde que l'adresse
    let pml4 = table_at(read_cr3() & ADDR_MASK);

    // Découpe l'adresse virtuelle en 4 index de 9 bits chacun.
    // >> 39 = décale de 39 bits vers la droite pour isoler les bits 47..39
    // & 0x1FF = masque les 9 bits bas (0x1FF = 0b111111111 = 511)
    // as usize = convertit en index pour accéder au tableau
    let i4 = ((virt >> 39) & 0x1FF) as usize;  // index dans PML4
    let i3 = ((virt >> 30) & 0x1FF) as usize;  // index dans PDPT
    let i2 = ((virt >> 21) & 0x1FF) as usize;  // index dans PD
    let i1 = ((virt >> 12) & 0x1FF) as usize;  // index dans PT

    // ── Niveau 4 : PML4 → PDPT ──────────────────────────
    // Si l'entrée PML4[i4] n'est pas PRESENT, aucune PDPT n'existe pour
    // cette portion de l'espace d'adressage. On en crée une.
    if pml4[i4] & PRESENT == 0 {
        let f = frame::alloc().expect("paging: plus de frames");
        zero_frame(f);                          // met la nouvelle table à zéro
        pml4[i4] = f | PRESENT | WRITABLE;      // inscrit l'adresse + flags
    }
    // Suit le pointeur : extrait l'adresse de la PDPT depuis l'entrée PML4
    let pdpt = table_at(pml4[i4] & ADDR_MASK);

    // ── Niveau 3 : PDPT → PD ────────────────────────────
    if pdpt[i3] & PRESENT == 0 {
        let f = frame::alloc().expect("paging: plus de frames");
        zero_frame(f);
        pdpt[i3] = f | PRESENT | WRITABLE;
    }
    let pd = table_at(pdpt[i3] & ADDR_MASK);

    // ── Niveau 2 : PD → PT ──────────────────────────────
    // ATTENTION : le boot.asm a mappé le 1er Go avec des huge pages 2 Mo.
    // Ces entrées ont le bit 7 (HUGE_PAGE) activé. Elles pointent directement
    // vers 2 Mo de RAM physique, sans passer par une PT.
    // Si on écrasait cette entrée, on perdrait le mapping du kernel → crash !
    if pd[i2] & HUGE_PAGE != 0 {
        return; // zone déjà mappée en 2 Mo par le boot, on ne touche pas
    }

    if pd[i2] & PRESENT == 0 {
        let f = frame::alloc().expect("paging: plus de frames");
        zero_frame(f);
        pd[i2] = f | PRESENT | WRITABLE;
    }
    let pt = table_at(pd[i2] & ADDR_MASK);

    // ── Niveau 1 : PT → Page physique ────────────────────
    // On écrit l'adresse physique du frame + les flags dans PT[i1].
    // C'est la dernière étape : le CPU sait maintenant que
    // `virt` pointe vers `phys`.
    pt[i1] = phys | PRESENT | WRITABLE;

    // Invalide le TLB pour cette adresse : le CPU doit relire les tables
    // au lieu d'utiliser son cache (qui a peut-être l'ancien mapping).
    invlpg(virt);
}

/// Supprime le mapping d'une page virtuelle (la rend inaccessible).
///
/// On descend les 4 niveaux de tables. Si à un niveau l'entrée n'est pas
/// PRESENT, c'est que la page n'était déjà pas mappée → rien à faire.
///
/// On ne touche pas aux huge pages (boot) : les unmapper casserait tout.
///
/// On met l'entrée PT à 0 (tous les flags à 0, dont PRESENT = 0).
/// Résultat : si un programme essaie d'accéder à cette adresse virtuelle,
/// le CPU déclenche un Page Fault (exception #14).
pub fn unmap(virt: u64) {
    let pml4 = table_at(read_cr3() & ADDR_MASK);

    let i4 = ((virt >> 39) & 0x1FF) as usize;
    let i3 = ((virt >> 30) & 0x1FF) as usize;
    let i2 = ((virt >> 21) & 0x1FF) as usize;
    let i1 = ((virt >> 12) & 0x1FF) as usize;

    // À chaque niveau, on vérifie que l'entrée existe avant de descendre
    if pml4[i4] & PRESENT == 0 { return; }
    let pdpt = table_at(pml4[i4] & ADDR_MASK);

    if pdpt[i3] & PRESENT == 0 { return; }
    let pd = table_at(pdpt[i3] & ADDR_MASK);

    // Si c'est une huge page 2 Mo du boot, on refuse de la toucher
    if pd[i2] & PRESENT == 0 || pd[i2] & HUGE_PAGE != 0 { return; }
    let pt = table_at(pd[i2] & ADDR_MASK);

    // Met l'entrée à 0 → PRESENT = 0 → la page n'existe plus
    pt[i1] = 0;
    invlpg(virt);
}

/// Traduit une adresse virtuelle en adresse physique.
///
/// Descend les 4 niveaux de tables en lecture seule.
/// Retourne Some(adresse_physique) si le mapping existe, None sinon.
///
/// Gère les 3 cas de taille de page :
/// - Huge page 1 Go (bit 7 dans PDPT) : offset = bits 0..29 (30 bits)
/// - Huge page 2 Mo (bit 7 dans PD)   : offset = bits 0..20 (21 bits)
/// - Page 4 Ko (normal)                : offset = bits 0..11 (12 bits)
///
/// L'offset est la position DANS la page. On l'ajoute à l'adresse de base
/// du frame pour obtenir l'adresse physique exacte.
pub fn translate(virt: u64) -> Option<u64> {
    let pml4 = table_at(read_cr3() & ADDR_MASK);

    let i4 = ((virt >> 39) & 0x1FF) as usize;
    if pml4[i4] & PRESENT == 0 { return None; }
    let pdpt = table_at(pml4[i4] & ADDR_MASK);

    let i3 = ((virt >> 30) & 0x1FF) as usize;
    if pdpt[i3] & PRESENT == 0 { return None; }
    // Huge page 1 Go : l'adresse physique est dans PDPT, pas besoin de descendre
    // 0x3FFF_FFFF = 30 bits d'offset (position dans la page de 1 Go)
    if pdpt[i3] & HUGE_PAGE != 0 {
        return Some((pdpt[i3] & ADDR_MASK) + (virt & 0x3FFF_FFFF));
    }

    let pd = table_at(pdpt[i3] & ADDR_MASK);
    let i2 = ((virt >> 21) & 0x1FF) as usize;
    if pd[i2] & PRESENT == 0 { return None; }
    // Huge page 2 Mo : l'adresse physique est dans PD
    // 0x1F_FFFF = 21 bits d'offset (position dans la page de 2 Mo)
    if pd[i2] & HUGE_PAGE != 0 {
        return Some((pd[i2] & ADDR_MASK) + (virt & 0x1F_FFFF));
    }

    let pt = table_at(pd[i2] & ADDR_MASK);
    let i1 = ((virt >> 12) & 0x1FF) as usize;
    if pt[i1] & PRESENT == 0 { return None; }

    // Page 4 Ko normale : adresse de base + offset 12 bits
    // 0xFFF = 12 bits d'offset (position dans la page de 4 Ko)
    Some((pt[i1] & ADDR_MASK) + (virt & 0xFFF))
}