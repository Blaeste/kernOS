# 05 — Pipeline de build

## Vue d'ensemble
```
src/boot.asm  ──NASM──►  target/boot.o  ─┐
src/main.rs   ──rustc──► target/kernos.o ─┤──linker──► target/.../kernos (ELF)
                                          │                    │
linker.ld ────────────────────────────────┘            cp vers isodir/boot/
                                                               │
                                                    grub-mkrescue (WSL)
                                                               │
                                                    target/kernos.iso
                                                               │
                                                    QEMU -cdrom kernos.iso
```

## `build.rs`
Exécuté automatiquement par Cargo avant chaque compilation.
```rust
// Compile l'ASM
Command::new("nasm")
    .args(["-f", "elf64", "src/boot.asm", "-o", "target/boot.o"])
    .status().expect("Échec NASM");

// Dit au linker d'inclure boot.o
println!("cargo:rustc-link-arg=target/boot.o");
```

## `run.ps1`
```powershell
cargo build                                              # compile Rust + ASM
cp target/x86_64-unknown-none/debug/kernos target/isodir/boot/kernos
wsl grub-mkrescue -o target/kernos.iso target/isodir    # crée l'ISO via WSL
& "C:\Program Files\qemu\qemu-system-x86_64.exe" -cdrom target/kernos.iso -boot d
```

## Pourquoi GRUB ?
- QEMU avec `-kernel` refuse les ELF 64-bit sans note PVH spéciale
- GRUB comprend Multiboot2 et charge correctement les kernels 64-bit
- GRUB tourne dans WSL (Linux) via `grub-mkrescue`

## Concepts clés
| Concept | Explication |
|---|---|
| `build.rs` | Script Cargo exécuté avant la compilation |
| `-f elf64` | Format de sortie NASM : ELF 64-bit |
| `.o` | Fichier objet — code compilé mais pas encore linké |
| ELF | Format binaire standard Linux/bare-metal |
| ISO | Image de disque bootable contenant GRUB + notre kernel |
