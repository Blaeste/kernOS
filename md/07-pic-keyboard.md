# 07 — PIC, Clavier & Souris PS/2

## PIC 8259 — Programmable Interrupt Controller

Le PIC est le **routeur d'interruptions matérielles**. Le CPU ne voit que les exceptions (0-31).
Le matériel (clavier, timer, souris) passe par le PIC, qui les redirige vers des entrées IDT.

### Architecture double PIC
```
            ┌──────────────┐
            │    CPU        │
            └──────┬───────┘
                   │
            ┌──────┴───────┐
            │   PIC1       │ ← IRQ 0-7  → IDT 32-39
            │   (maître)   │
            └──────┬───────┘
                   │ IRQ2 = cascade
            ┌──────┴───────┐
            │   PIC2       │ ← IRQ 8-15 → IDT 40-47
            │   (esclave)  │
            └──────────────┘
```

**IRQ2 = cascade** — c'est la ligne qui connecte PIC2 à PIC1. Si elle est masquée,
aucune interruption de PIC2 ne passe (souris = IRQ12 = PIC2).

### Ports I/O
| Port | Usage |
|------|-------|
| `0x20` | PIC1 commande |
| `0x21` | PIC1 données (masques) |
| `0xA0` | PIC2 commande |
| `0xA1` | PIC2 données (masques) |

### Séquence d'initialisation
1. `CMD_INIT (0x11)` → démarre la séquence
2. Offset IDT → IRQ0 = entrée 32, IRQ8 = entrée 40
3. Câblage cascade → PIC1 sait que PIC2 est sur IRQ2
4. Mode 8086 → compatibilité x86
5. Masques → active uniquement les IRQ voulus

### Masques (0 = actif, 1 = masqué)
```
PIC1: 0b11111000 = timer(0) + clavier(1) + cascade(2)
PIC2: 0b11101111 = souris(IRQ12 = bit 4)
```

### EOI (End Of Interrupt)
Après chaque IRQ, on doit envoyer `CMD_EOI (0x20)` au PIC.
Pour IRQ 8-15 (PIC2) : envoyer EOI aux deux PICs.
Sinon le PIC croit qu'on traite encore l'interruption.

---

## Clavier PS/2 (IRQ1)

Le clavier envoie des **scancodes** sur le port `0x60`.

| Scancode | Signification |
|----------|--------------|
| `< 0x80` | Touche appuyée |
| `>= 0x80` | Touche relâchée (scancode réel = valeur - 0x80) |

### Handler
```rust
pub extern "C" fn irq_keyboard_inner() {
    let scancode = unsafe { pic::inb(0x60) };  // lire le port AVANT eoi
    unsafe { pic::eoi(1); }                     // libérer le PIC

    if scancode >= 0x80 {
        println!("Relache: {}", scancode - 0x80);
    } else {
        println!("Appui: {}", scancode);
    }
}
```

---

## Souris PS/2 (IRQ12)

La souris PS/2 est **désactivée par défaut**. Il faut l'activer explicitement
via le contrôleur PS/2 (ports 0x64/0x60).

### Initialisation (`mouse.rs`)
```
1. ps2_write_cmd(0xA8)  → active le port souris
2. Lire la config (0x20), activer bit 1 (IRQ12), réécrire (0x60)
3. ps2_write_cmd(0xD4)  → prochain octet destiné à la souris
4. ps2_write_data(0xF4) → "enable data reporting"
```

### Paquets souris (3 octets)
Chaque mouvement/clic envoie **3 IRQ12 consécutifs** :
```
Octet 0 : flags  — boutons + overflow + signes
    bit 0 = bouton gauche
    bit 1 = bouton droit
    bit 2 = bouton milieu
    bit 3 = toujours 1 (sert à la synchronisation)
Octet 1 : delta X (mouvement horizontal, signé)
Octet 2 : delta Y (mouvement vertical, signé)
```

### Synchronisation
Le bit 3 du premier octet est **toujours à 1**. Si on le reçoit à 0,
c'est qu'on est décalé → on ignore l'octet et on attend le vrai début.

```rust
if PACKET_IDX == 0 && (byte & 0x08) == 0 {
    return;  // décalé, on resynchronise
}
```

---

## Stubs naked — Sauvegarde des registres

Le CPU interrompt le code à n'importe quel moment. Si le handler modifie
les registres, le code interrompu plante au retour.

**Solution** : naked function qui sauvegarde tous les registres.

```
push rax, rbx, rcx, rdx, rsi, rdi, rbp, r8-r15   ← sauvegarde
call handler_inner                                  ← traitement
pop  r15, r14, ..., rax                             ← restauration
iretq                                               ← retour (pas ret !)
```

`iretq` restaure aussi `rip`, `cs`, `rflags`, `rsp`, `ss` depuis la stack.

---

## Ordre de boot obligatoire
```rust
interrupt::init();          // 1. charger l'IDT
pic::init();                // 2. init PIC (remapping + masques)
unsafe { mouse::init(); }  // 3. activer la souris PS/2
unsafe { asm!("sti"); }    // 4. activer les interruptions (EN DERNIER)
```

---

## Concepts clés
| Concept | Explication |
|---------|------------|
| Cascade PIC | IRQ2 relie PIC2→PIC1, doit être démasqué pour IRQ 8-15 |
| Port 0x60 | Buffer partagé clavier/souris — lire AVANT eoi |
| Paquet 3 octets | La souris envoie 3 IRQ12 par événement |
| Bit 3 sync | Premier octet a toujours bit 3 = 1, sert à se resynchroniser |
| `naked_asm!` | Fonction sans prologue/épilogue Rust, contrôle total ASM |
| `iretq` | Retour d'interruption (≠ `ret` qui est pour les fonctions normales) |
