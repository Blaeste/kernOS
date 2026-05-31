/*
    ENTRY(_start32) → le point d'entrée du programme est la fonction _start32
    SECTIONS { } → on définit comment organiser la mémoire
    . = 0x100000; → charge le kernel à l'adresse 1 MB en mémoire (convention standard en x86, la zone en dessous est réservée au BIOS/firmware)
    .multiboot ->
    .text → section du code (les instructions assembleur)
    .rodata → read-only data (strings, constantes)
    .data → données modifiables (variables globales initialisées)
    .bss → données non-initialisées (variables globales à zéro)
    *(.text*) → "prends tous les morceaux .text de tous les fichiers objets"
*/

ENTRY(_start32)

SECTIONS {
    . = 0x100000;

    .multiboot : {
        KEEP(*(.multiboot))
    }

    .text : {
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data*)
    }

    .bss : {
        *(.bss*)
    }

    . = ALIGN(4096);
    kernel_end = .;
}