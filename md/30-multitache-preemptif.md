# 30 — Multitâche préemptif

## Objectif

Rendre le multitâche fonctionnel : on peut lancer des tâches concurrentes depuis le shell.

## Architecture existante

Le context switch était déjà implémenté (chapitres précédents) :

- **`irq_timer_stub`** (interrupt.rs) — naked function assembleur, sauvegarde/restaure 15 registres
- **`schedule(old_rsp)`** (scheduler.rs) — round-robin, retourne le RSP de la prochaine tâche
- **`spawn(entry)`** — prépare une pile avec 20 valeurs (15 regs + 5 iretq)

## Ajouts

### task_exit()

Quand une tâche se termine, elle doit appeler `task_exit()` :

```rust
pub fn task_exit() {
    TASKS[CURRENT].active = false;
    asm!("int 0x20");  // force un reschedule via IRQ timer
    loop {}
}
```

`int 0x20` déclenche manuellement l'IRQ timer (vecteur 32 = 0x20) → le scheduler passe à la prochaine tâche active. La tâche courante est désactivée → elle ne sera plus jamais schedulée.

### Commande shell `spawn`

Lance une tâche de test :

```rust
fn test_task() {
    println!("[Tache 1] Salut depuis une autre tache !");
    for i in 0..5 {
        println!("[Tache 1] tick {}", i);
        // attente active ~1s
    }
    println!("[Tache 1] Termine !");
    task_exit();
}
```

### Flux complet

```
> spawn
  ↓
spawn(test_task) → slot 1, pile 8Ko, active = true
  ↓
Timer tick → IRQ0 → irq_timer_stub
  ↓
schedule(): tâche 0 (shell) → tâche 1 (test_task)
  ↓
test_task s'exécute, affiche ses messages
  ↓
task_exit() → active = false → int 0x20
  ↓
schedule(): tâche 1 inactive → retour tâche 0 (shell)
```

## Limitations actuelles

- Stack fixe 8 Ko (suffisant pour l'instant)
- Round-robin sans priorité
- Pas d'isolation mémoire entre tâches
- `println!` partagé (pas de mutex) — fonctionne car single-core

## Test

```
> spawn
Tache lancee !
> [Tache 1] Salut depuis une autre tache !
[Tache 1] tick 0
[Tache 1] tick 1
...
[Tache 1] Termine !
```
