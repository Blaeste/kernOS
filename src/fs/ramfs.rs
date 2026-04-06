/// Système de fichiers en RAM (RAMfs)

use crate::klib::string::{eq, to_str, to_str_or};

/// Nom max d'un fichier (31 chars + null)
const NAME_MAX: usize = 32;

// Taille max du contenu d'un fichier
const DATA_MAX: usize = 512;

/// Nombre max de fichiers
const MAX_FILES: usize = 16;

/// Un ficheir en RAM
struct File {
    name:   [u8; NAME_MAX],     // Nom du fichier
    data:   [u8; DATA_MAX],     // contenu
    size:   usize,              // taille réelle des données
    used:   bool,               // slot occupé ou libre
}

/// Tableau de tous les fichiers
static mut FILES: [File; MAX_FILES] = [const { File {
    name:   [0; NAME_MAX],
    data:   [0; DATA_MAX],
    size:   0,
    used:   false,
} }; MAX_FILES];

/// Accès en lecture au tableau de fichiers
fn files() -> &'static [File; MAX_FILES] {
    unsafe { &*core::ptr::addr_of!(FILES) }
}

/// Accès en écriture au tableau de fichiers
fn files_mut() -> &'static mut [File; MAX_FILES] {
    unsafe { &mut *core::ptr::addr_of_mut!(FILES) }
}

/// Crée un fichier avec un nom et du contenu.
/// Retourne true si OK, false si plus de place ou nom déjà pris.
pub fn create(name: &[u8], data: &[u8]) -> bool {
    // Verifie que le nom n'existe pas déja
    for i in 0..MAX_FILES {
        let f = &files()[i];
        if f.used && eq(&f.name[..name.len()], name) {
            return false;
        }
    }

    // Cherche un slot libre
    for i in 0..MAX_FILES {
        let f = &mut files_mut()[i];
        if !f.used {
            // Copie le nom
            for j in 0..name.len() {
                f.name[j] = name[j];
            }
            f.name[name.len()] = 0;

            // Copie les données (max DATA_MAX octets)
            let len = if data.len() > DATA_MAX { DATA_MAX } else { data.len() };
            for j in 0..len {
                f.data[j] = data[j];
            }
            f.size = len;
            f.used = true;
            return true;
        }
    } 
    false
}

/// Affiche la liste des fichiers (nom + taille)
pub fn ls() {
    let mut count = 0;
    for i in 0..MAX_FILES {
        let f = &files()[i];
        if f.used {
            let name = to_str(&f.name[..name_len(&f.name)]);
            crate::println!(" {} ({} octets)", name, f.size);
            count += 1;
        }
    }
    if count == 0 {
        crate::println!(" (vide)");
    }
}

/// Retourne la longeur d'un nom (jusqu'au null terminator)
fn name_len(name: &[u8]) -> usize {
    let mut i = 0;
    while i < name.len() && name[i] != 0 {
        i += 1;
    }
    i
}

/// Affiche le contenu d'un fichier par son nom
pub fn cat(name: &[u8]) {
    for i in 0..MAX_FILES {
        let f = &files()[i];
        if f.used && eq(&f.name[..name.len()], name) {
            let content = to_str_or(&f.data[..f.size], "(binaire)");
            crate::println!("{}", content);
            return;
        }
    }
    crate::println!("fichier introuvables: {}", to_str(name));
}

/// Supprime un fichier par son nom
pub fn rm(name: &[u8]) {
    for i in 0..MAX_FILES {
        let f = &mut files_mut()[i];
        if f.used && eq(&f.name[..name.len()], name) {
            f.used = false;
            crate::println!("supprime: {}", to_str(name));
            return;
        }
    }
    crate::println!("fichier introuvable: {}", to_str(name));
}