# Étape 15 — Multitâche (Scheduler Round-Robin)

## Concept

Le multitâche permet d'exécuter plusieurs tâches "en même temps". En réalité, le CPU alterne entre les tâches si vite (100 fois/sec) que ça semble simultané.

## Comment ça marche

1. Chaque tâche a sa propre **pile** (8 Ko) et son **RSP** (stack pointer) sauvegardé
2. Le **timer PIT** (100 Hz) interrompt le CPU toutes les 10 ms
3. Le **scheduler** sauvegarde le RSP de la tâche courante, choisit la suivante (round-robin), et charge son RSP
4. Le CPU reprend la nouvelle tâche comme si rien ne s'était passé

## Le Context Switch (dans irq_timer_stub)

```
1. push 15 registres        → sauvegarde sur la pile de la tâche courante
2. mov rdi, rsp             → passe RSP en argument
3. call irq_timer_inner     → tick + EOI + schedule()
4. mov rsp, rax             → CHANGE DE PILE (le switch !)
5. pop 15 registres         → restaure depuis la NOUVELLE pile
6. iretq                    → reprend la nouvelle tâche
```

La ligne `mov rsp, rax` est LE moment du switch : on passe de la pile d'une tâche à celle d'une autre.

## Fichier : `src/task/mod.rs`

### Structure

```rust
pub struct Task {
    pub rsp:    u64,              // pointeur de pile sauvegardé
    pub stack:  [u8; STACK_SIZE], // pile privée (8 Ko)
    pub active: bool,             // slot utilisé ou libre
}
```

### Fonctions

| Fonction | Rôle |
|---|---|
| `spawn(entry)` | Crée une tâche : trouve un slot libre, prépare la pile, l'active |
| `schedule(old_rsp) → u64` | Round-robin : sauvegarde RSP courant, retourne RSP de la suivante |

### Préparation de la pile (spawn)

On "triche" en empilant 20 valeurs comme si la tâche avait déjà été interrompue :
- 15 registres à 0 (seront restaurés par les `pop`)
- RIP = adresse de la fonction entry
- CS = 0x08 (segment code 64-bit)
- RFLAGS = 0x202 (interruptions activées)
- RSP = sommet de la pile
- SS = 0x00 (segment données)

Quand le scheduler choisit cette tâche, il pop les registres + iretq → la tâche démarre.

## Commande shell `tasks`

```
> tasks
== Taches ==
 [0] active        ← kernel/shell
 [1] active        ← tâche de test 1
 [2] active <-     ← tâche de test 2 (courante)
```

## Modifications dans interrupt.rs

- `irq_timer_inner(rsp: u64) -> u64` : reçoit le RSP, appelle `schedule()`, retourne le nouveau RSP
- `irq_timer_stub` : ajout de `mov rdi, rsp` avant call et `mov rsp, rax` après

## Points techniques

- **Round-robin** : `(current + 1) % MAX_TASKS` — chaque tâche a un temps égal
- **RFLAGS 0x202** : bit IF = 1 = interruptions activées. Sans ça, la nouvelle tâche bloquerait le timer
- **CS 0x08** : sélecteur du segment de code dans la GDT (défini dans boot.asm)
- **iretq** : dépile RIP, CS, RFLAGS, RSP, SS — c'est comme ça que le CPU "reprend" une tâche
