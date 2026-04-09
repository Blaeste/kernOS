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
