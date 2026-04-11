; ============================================================
; MULTIBOOT HEADER — identification du kernel pour QEMU
; ============================================================
; section .multiboot → place ce contenu dans notre section .multiboot (définie dans linker.ld)
; align 8 → aligne à 8 octets (obligation Multiboot)
; dd = define doubleword = écrire un nombre de 32 bits
section .multiboot
align 8
header_start:
    dd 0xE85250D6                                                		; magic Multiboot2
    dd 0                                                        		; architecture i386
    dd header_end - header_start                                		; taille du header
    dd 0x100000000 - (0xE85250D6 + 0 + (header_end - header_start)) 	; checksum

	; --- Tag framebuffer --- demande un écran graphique au lieu du mode texte
	align 8																; alignement obligatoire pour chaque tag
	dw 5																; type = 5 -> framebuffer request
	dw 0																; flags = 0 -> optionnel (GRUB boote quand même si impossible)
	dd 20																; taille du tag = 20 octets (header 8 + 3 x u32)
	dd 1920																; largeur voulue : 800 pixels
	dd 1080																; hauteur voulue : 600 pixels
	dd 32																; profondeur : 32 bits par pixel (R8 G8 B8 A8)

	; --- Tag fin ---
	align 8
    dw 0  																; type = end tag
    dw 0  																; flags
    dd 8 																; size
header_end:

; ============================================================
; MEMOIRE — pile + tables de pages (non initialisées)
; ============================================================
; section .bss = mémoire non-initialisée, mise à zéro au démarrage.
; align 4096 → aligne à 4096 octets (taille d'une page mémoire — obligatoire pour les tables de pages)
; p4_table/p3_table/p2_table → tables de pagination (on en parle juste après)
; resb 4096 = reserve bytes = réserve 4096 octets vides
; stack_bottom / stack_top → notre pile de 16 Ko (16384 octets). On pointe esp vers stack_top car la pile grandit vers le bas en x86
section .bss
align 4096

p4_table: resb 4096						; table de pages de niveau 4 (P4) — la plus haute
p3_table: resb 4096						; table de pages de niveau 3 (P3)
p2_table: resb 16384				    ; table de pages de niveau 2 (P2) — la plus basse, pointe vers les pages physiques

stack_bottom:							; adresse de la base de la pile (non utilisée, mais bonne pratique de la définir)
	resb 16384							; réserve 16 Ko pour la pile
stack_top:								; adresse du sommet de la pile (esp doit être initialisé à cette adresse)

global stack_top

; ============================================================
; CODE 32-BIT — point d'entrée de QEMU
; ============================================================
section .text
bits 32

global _start32 						; rend _start32 visible pour le linker (comme pub en Rust)

_start32:
	mov esp, stack_top 					; initialise le stack pointer. Sans stack, aucune fonction ne peut être appelée
	mov edi, ebx						; sauvegarde l'adresse Multiboot2 info (passée par GRUB dans ebx)
	call setup_page_tables				; configure les tables de pages pour le long mode (64-bit)

	mov eax, cr4						; active le mode PAE (Physical Address Extension), nécessaire pour le long mode
	or eax, 1 << 5						; bit 5 = PAE
	mov cr4, eax						; cr4 = cr4 | (1 << 5) pour activer le mode PAE

	mov eax, p4_table					; charge l'adresse de la table P4 dans cr3 pour activer la pagination
	mov cr3, eax						; cr3 = adresse de la table de pages de niveau 4

	mov ecx, 0xC0000080					; numéro du MSR IA32_EFER (pas une adresse — c'est un identifiant de registre interne du CPU)
	rdmsr								; lit le MSR IA32_EFER dans edx:eax (64 bits)
	or eax, 1 << 8						; bit 8 = LME (Long Mode Enable) — active le long mode
	wrmsr								; écrit le nouveau MSR IA32_EFER avec le long mode activé

	mov eax, cr0						; active la protection (bit 0 = PE) et le long mode (bit 31 = PG)
	or eax, 1 << 31						; bit 31 = PG (Paging) — active la pagination, nécessaire pour le long mode
	mov cr0, eax						; cr0 = cr0 | (1 << 31) pour activer la pagination

	lgdt [gdt64.pointer]				; charge la GDT (Global Descriptor Table) pour le long mode. La GDT est nécessaire même en long mode, mais elle est très simple (un seul segment couvrant tout l'espace mémoire)
	jmp gdt64.code:_start64				; saute à _start64 en utilisant le segment de code défini dans la GDT (bits 43, 44, 47, 53 = code segment 64-bit)

; Le mode 64-bit exige la pagination activée.
; On crée un mapping simple (identity map) : l'adresse virtuelle = l'adresse physique.
; 3 niveaux de tables : P4 → P3 → P2
setup_page_tables:
	; P4[0] -> P3
	mov eax, p3_table					; adresse de la table P3
	or eax, 0b111						; bits 0 et 1 = present + writable
	mov [p4_table], eax					; la table P4 pointe vers la table P3

	; P3[0] -> P2+0, P3[1] -> P2+4096, P3[2] -> P2+8192, P3[3] -> P2+12288
	mov ecx, 0							; compteur de boucle (0, 1, 2, 3)
.p3loop:
	mov eax, p2_table					; adresse de la table P2
	mov edx, ecx						; copie ecx dans edx pour le calcul d'offset
	shl edx, 12							; edx = ecx x 4096 (offset de chaque table P2)
	add eax, edx
	or eax, 0b111						; bits 0 et 1 = present + writable
	mov [p3_table + ecx * 8], eax		; la table P3 pointe vers la table P2
	inc ecx								; passe à la table P2 suivante
	cmp ecx, 4							; 4 tables P2 = 4 Go
	jne .p3loop

	; Remplit les 2048 entrées P2 (4 x 512)
	mov ecx, 0							; index de la page à mapper (0 = adresse physique 0, 1 = adresse physique 2 Mo, etc.)
.loop:									; La boucle mappe 2048 × 2 Mo = 4 Go de RAM en identity map
	mov eax, 0x200000					; 2 Mo = taille d'une huge page (bit 7 de la table de pages)
	mul ecx								; eax = adresse physique de la page à mapper
	or eax, 0b10000111					; ajoute le bit 7 = huge page (pages de 2 Mo au lieu de 4 Ko — plus simple pour démarrer)
	mov [p2_table + ecx * 8], eax		; la table P2 pointe vers la page physique
	inc ecx								; passe à la page suivante
	cmp ecx, 2048						; 2048 × 2 Mo = 4 Go de RAM
	jne .loop							; tant que ce n'est pas fini, continue la boucle
	ret									; retourne à _start32 pour continuer le processus de démarrage en long mode

; ============================================================
; GDT 64-BIT — table de descripteurs de segments
; ============================================================
section .rodata							; la GDT est en lecture seule, elle ne doit pas être modifiée après le démarrage
align 8									; aligne à 8 octets pour respecter les exigences du processeur pour les descripteurs de segments
gdt64:									; Global Descriptor Table pour le long mode. En long mode, les segments sont très simples : un segment de code 64-bit et un segment nul (obligatoire)
	dq 0								; descripteur de segment nul (obligatoire, le processeur considère le segment 0 comme invalide)
.code: equ $ - gdt64					; offset = 8 (adresse du descripteur dans la GDT, utilisé comme sélecteur dans le jmp)
	dq (1<<43)|(1<<44)|(1<<47)|(1<<53)	; bits 43, 44, 47, 53 = code segment 64-bit
.pointer:								; descripteur de segment pour lgdt : limite = taille de la GDT - 1, base = adresse de la GDT
	dw $ - gdt64 - 1					; limite = taille de la GDT - 1 (16 octets - 1 = 15)
	dq gdt64							; base = adresse de la GDT

; ============================================================
; CODE 64-BIT — on est enfin en long mode
; ============================================================
section .text							; le code 64-bit doit être dans une section .text séparée du code 32-bit pour éviter les problèmes d'offsets et de segments
bits 64									; indique que le code qui suit est du code 64-bit, ce qui permet d'utiliser les registres 64-bit (rax, rbx, etc.) et les instructions du long mode

_start64:								; point d'entrée en long mode
	mov ax, 0							; segment nul pour les segments de données (on n'utilise pas de segmentation en long mode, mais il faut quand même initialiser les registres de segment)
	mov ss, ax							; segment nul pour la pile
	mov ds, ax							; segment nul pour les données
	mov es, ax							; segment nul pour les données
	mov fs, ax							; segment nul pour les données
	mov gs, ax							; segment nul pour les données

	extern _start						; indique que _start est défini dans un autre fichier (main.rs en Rust)
	; edi contient déja l'adresse multiboot (sauvegardée en 32-bit)
	; En x86_64 System V ABI, rdi = 1er argument de focntion
	call _start							; appelle la fonction _start du kernel pour continuer le processus de démarrage

	hlt									; si _start retourne, on arrête le processeur (normalement, _start ne devrait jamais retourner)
