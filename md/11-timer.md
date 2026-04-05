# 11 — Timer PIT

## PIT (Programmable Interval Timer)

Le PIT est un chip qui génère des interruptions à intervalle régulier (IRQ0).
Indispensable pour :
- Mesurer le temps écoulé
- Le futur scheduler (multitâche = changer de processus à chaque tick)

### Fréquence
Le PIT a une horloge interne à **1 193 182 Hz**. On envoie un **diviseur**
pour choisir la fréquence des interruptions :

```
fréquence = 1_193_182 / diviseur
diviseur  = 1_193_182 / fréquence_voulue
```

Avec `freq = 100` → diviseur = 11931 → **une interruption toutes les 10ms**.

---

## Ports I/O

| Port | Usage |
|------|-------|
| `0x40` | Channel 0 données (diviseur) |
| `0x43` | Registre commande |

### Commande `0x36`
```
0x36 = 0011_0110
  bits 6-7 = 00  → channel 0
  bits 4-5 = 11  → accès lo/hi byte (envoyer le diviseur en 2 fois)
  bits 1-3 = 011 → mode 3 (square wave generator)
  bit 0    = 0   → binaire (pas BCD)
```

---

## Code (`timer.rs`)

```rust
pub fn init(freq: u32) {
    let divisor = PIT_FREQ / freq;
    // Envoie commande puis diviseur (low byte, high byte)
}

pub fn tick() {
    // Incrémente le compteur global TICKS
}

pub fn get_ticks() -> u64 {
    // Retourne le nombre de ticks depuis le boot
}
```

### Calcul du temps
```rust
let seconds = get_ticks() / 100;  // 100 ticks = 1 seconde
```

---

## Intégration

### `interrupt.rs` — handler timer
```rust
pub extern "C" fn irq_timer_inner() {
    timer::tick();        // compter
    unsafe { pic::eoi(0); }  // libérer le PIC
}
```

### `shell.rs` — commande uptime
```rust
} else if eq(cmd, b"uptime") {
    let ticks = crate::timer::get_ticks();
    let seconds = ticks / 100;
    crate::println!("Uptime: {} secondes ({} ticks)", seconds, ticks);
}
```

---

## Ordre de boot
```rust
interrupt::init();      // 1. IDT
pic::init();            // 2. PIC
timer::init(100);       // 3. PIT à 100 Hz  ← nouveau
mouse::init();          // 4. Souris PS/2
asm!("sti");            // 5. Activer les interruptions
```

---

## Concepts clés
| Concept | Explication |
|---------|------------|
| PIT | Chip matériel qui génère des interruptions périodiques |
| Diviseur | `base_freq / diviseur = fréquence_IRQ` |
| Tick | Une unité de temps = une interruption timer |
| `addr_of!` vs `addr_of_mut!` | Lecture = `addr_of!`, écriture = `addr_of_mut!` |
| Mode 3 | Square wave — génère un signal régulier |
