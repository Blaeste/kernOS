/*
    Command::new("nasm") → lance NASM comme si tu le tapais dans le terminal
    -f elf64 → format de sortie : ELF 64-bit (même format que notre binaire Rust)
    src/boot.asm → fichier source
    -o target/boot.o → fichier objet de sortie (.o = code compilé pas encore linké)
    println!("cargo:rustc-link-arg=...") → dit au linker d'inclure boot.o dans le binaire final
    println!("cargo:rerun-if-changed=...") → recompile boot.asm seulement si il a changé
*/

use std::process::Command;
use std::fs;

fn main() {

    // 0. Compile la crate userspace user/hello
    //
    // src/userspace.rs fait include_bytes! sur le binaire produit ici. Comme
    // user/hello/target/ est gitignoré, sans cette étape un clone frais ne peut
    // pas compiler le kernel du tout.
    //
    // current_dir("user/hello") est OBLIGATOIRE : le "-C link-arg=-Tlinker.ld"
    // de .cargo/config.toml (racine) est hérité par user/hello, et le chemin
    // "linker.ld" est résolu depuis le répertoire courant. Lancé d'ailleurs,
    // le linker prendrait le linker.ld du kernel (base 0x100000) au lieu de
    // celui de hello (base 0x40000000) → ELF chargé à la mauvaise adresse.
    //
    // env_remove : cargo passe son propre environnement aux build scripts.
    // Sans nettoyage, les rustflags et le target-dir du kernel écraseraient
    // la configuration de la crate user.
    build_user_crate();

    // 1. Compile boot.asm
    Command::new("nasm")
        .args(["-f", "elf64", "src/boot.asm", "-o", "target/boot.o"])
        .status()
        .expect("Échec de la compilation de boot.asm avec NASM");

    println!("cargo:rustc-link-arg=target/boot.o");
    println!("cargo:rerun-if-changed=src/boot.asm");

    // 2. Prepare la structure de l'ISO
    fs::create_dir_all("target/isodir/boot/grub").unwrap();

    fs::write("target/isodir/boot/grub/grub.cfg",
        "set timeout=0\nset default=0\nset gfxmode=1920x1080x32\nset gfxpayload=keep\n\nmenuentry \"KernOS\" {\n    multiboot2 /boot/kernos\n    boot\n}\n"
    ).unwrap();

    // 3. Copie le kernel (aprees compilation)
    println!("cargo:rerun-if-changed=target/x86_64-unknown-none/debug/kernos");
}

/// Compile user/hello (et libkern par dépendance) avant le kernel.
fn build_user_crate() {

    // Recompile le kernel dès qu'une source userspace change, sinon
    // include_bytes! garderait l'ancien ELF en cache.
    println!("cargo:rerun-if-changed=user/hello/src");
    println!("cargo:rerun-if-changed=user/hello/Cargo.toml");
    println!("cargo:rerun-if-changed=user/hello/linker.ld");
    println!("cargo:rerun-if-changed=user/libkern/src");
    println!("cargo:rerun-if-changed=user/libkern/Cargo.toml");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());

    let mut cmd = Command::new(cargo);
    cmd.arg("build").current_dir("user/hello");

    // Variables que cargo positionne pour NOTRE build et qui fausseraient
    // la compilation de la crate user si elles fuitaient.
    for var in [
        "CARGO_ENCODED_RUSTFLAGS", "RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",   "CARGO_BUILD_TARGET",
        "CARGO_TARGET_DIR",        "CARGO_BUILD_TARGET_DIR",
        "RUSTC_WRAPPER",           "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_MAKEFLAGS",
    ] {
        cmd.env_remove(var);
    }

    let status = cmd.status().expect("Échec du lancement de cargo pour user/hello");

    if !status.success() {
        panic!("La compilation de user/hello a échoué — le kernel ne peut pas l'embarquer");
    }
}
