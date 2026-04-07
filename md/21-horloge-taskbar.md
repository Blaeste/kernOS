# 21 — Horloge dans la taskbar

## Résultat

Affichage de l'uptime `MM:SS` centré verticalement à droite de la taskbar,
mis à jour automatiquement chaque seconde.

```
[ Terminal ]  [ Fichiers ]                              00:42
```

## Implémentation

### Affichage — `taskbar.rs`

Calcul de l'uptime depuis les ticks du timer (100 Hz = 100 ticks/s) :

```rust
let ticks = crate::drivers::timer::get_ticks();
let secs  = ticks / 100;
let mins  = secs / 60;
let s     = secs % 60;
```

Formatage manuel en `MM:SS` sans allocation :

```rust
let mut buf = [0u8; 8];
buf[0] = b'0' + (mins / 10) as u8;
buf[1] = b'0' + (mins % 10) as u8;
buf[2] = b':';
buf[3] = b'0' + (s / 10) as u8;
buf[4] = b'0' + (s % 10) as u8;
let time_str = to_str_or(&buf[..5], "00:00");
```

Position : coin bas-droit, centré verticalement :

```rust
let clock_y = sh - TASKBAR_H + (TASKBAR_H - 16) / 2;
```

### Mise à jour automatique — `timer.rs`

Chaque seconde (100 ticks), l'IRQ timer redessine la taskbar et swap :

```rust
if *ptr % 100 == 0 {
    if *core::ptr::addr_of!(crate::io::framebuffer::FB_ACTIVE) {
        crate::ui::taskbar::draw();
        crate::io::framebuffer::swap_buffers();
    }
}
```

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/ui/taskbar.rs` | Affichage `MM:SS` à droite, centré verticalement |
| `src/drivers/timer.rs` | Redraw taskbar chaque seconde dans `tick()` |
