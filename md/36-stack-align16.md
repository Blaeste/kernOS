# 36 — Alignement 16 octets des piles (fix panic Linux)

## Symptôme

Panic kernel sur Linux, mais pas sur Windows, quand on exécute la
commande `ring3` (syscall depuis ring 3) ou `syscalltest`.

## Cause

Le compilateur Rust sur Linux (LLVM) génère des instructions SSE2/AVX
(`MOVAPS`, `MOVDQA`, etc.) pour les copies de structures et le formatting
(`println!`, `core::fmt`). Ces instructions exigent un RSP **aligné sur
16 octets** — sinon le CPU déclenche un `#GP` (General Protection Fault).

### Séquence d'un syscall depuis ring 3

```
int 0x80 depuis ring 3
  → CPU lit TSS.rsp0 = sommet de KERNEL_INT_STACK
  → CPU switch RSP = TSS.rsp0
  → CPU pousse SS, RSP, RFLAGS, CS, RIP   (5 × 8 = 40 octets)
  → syscall_stub pousse 15 regs            (15 × 8 = 120 octets)
  → call syscall_inner                     ← RSP doit être ≡ 0 (mod 16)
```

RSP avant `call` = `rsp0 - 160`. Pour que `call` soit aligné :
**`rsp0 ≡ 0 (mod 16)`**, donc `KERNEL_INT_STACK_top ≡ 0 (mod 16)`.

### Pourquoi ça marchait sur Windows

Sur Windows, le linker COFF/PE place les statics à des adresses
naturellement alignées à 16 octets par convention de section.
Sur Linux (ELF), `[u8; N]` a l'alignement déclaré = **1** → le linker
peut placer la pile à n'importe quelle adresse.

## Fix

Ajouter `#[repr(align(16))]` sur toutes les piles statiques :

### `gdt.rs` — pile kernel d'interruption

```rust
#[repr(align(16))]
struct IntStack([u8; 8192]);
static mut KERNEL_INT_STACK: IntStack = IntStack([0; 8192]);

// usage :
let int_stack_top = addr_of_mut!(KERNEL_INT_STACK.0) as u64 + 8192;
```

`8192 = 512 × 16` → `stack_top = base_16aligned + 512×16 = 16-byte aligned`. ✓

### `userspace.rs` — pile ring 3

```rust
#[repr(align(16))]
struct UserStack([u8; 8192]);
static mut USER_STACK: UserStack = UserStack([0; 8192]);

// usage :
let stack_top = addr_of_mut!(USER_STACK.0) as u64 + 8192;
```

### `scheduler.rs` — piles des tâches kernel

```rust
#[repr(C, align(16))]
pub struct Task {
    pub stack:  [u8; STACK_SIZE],  // offset 0 → 16-byte aligné
    pub rsp:    u64,
    pub active: bool,
}
```

`stack` en premier (offset 0) : `&task.stack[0]` = adresse de la Task
= 16-byte alignée. Puis `stack_top = base + 8192` = 16-byte alignée. ✓

## Règle générale

> Toute pile utilisée comme RSP lors d'un `call` en kernel
> **doit être alignée à 16 octets**. Utiliser `#[repr(align(16))]`
> sur les tableaux `[u8; N]` statiques ou les structs qui les contiennent.

### `mem/allocator.rs` — heap

```rust
#[repr(align(16))]
struct Heap([u8; HEAP_SIZE]);
static mut HEAP: Heap = Heap([0; HEAP_SIZE]);

// init() :
let head = core::ptr::addr_of_mut!(HEAP.0) as *mut FreeBlock;
```

`FreeBlock` = 2 × u64 = 16 octets. L'initialisation `(*head).size = N;
(*head).next = null;` peut être fusionnée par LLVM en un `MOVAPS` (16 octets).
Si `HEAP` est à une adresse non multiple de 16 → `#GP` dès `allocator::init()`,
soit au tout début du kernel — avant même d'afficher quoi que ce soit.

### `io/framebuffer.rs` — text_buf et BACK_BUF

```rust
// FbWriter : text_buf en premier (offset 0 = toujours aligné)
#[repr(C, align(16))]
pub struct FbWriter {
    pub text_buf: [[u8; TEXT_MAX_COLS]; TEXT_MAX_ROWS],  // offset 0
    pub row: u32, ...
}

// BACK_BUF 8 Mo
#[repr(align(16))]
struct BackBuf([u32; 2_073_600]);
static mut BACK_BUF: BackBuf = BackBuf([0; 2_073_600]);
```

`clear_screen()` fait `self.text_buf = [[b' '; 100]; 200]` = 20 Ko.
Sans `align(16)`, `text_buf` était à l'offset 24 dans `FbWriter`
(6 × u32 = 24 octets, 24 mod 16 = 8) → non aligné → `#GP` dès le boot
si le compilateur Linux utilise `MOVAPS` pour le memset.

`BACK_BUF` : `copy_nonoverlapping` de chaque ligne de pixels peut générer
du SSE vectorisé → même problème si le buffer commence à une adresse 4n.

## Fichiers modifiés

- `src/mem/allocator.rs` — `Heap` + `addr_of_mut!(HEAP.0)`
- `src/gdt.rs` — `IntStack` + `addr_of_mut!(KERNEL_INT_STACK.0)`
- `src/userspace.rs` — `UserStack` + `addr_of_mut!(USER_STACK.0)`
- `src/task/scheduler.rs` — `Task` `align(16)`, champ `stack` en premier
- `src/io/framebuffer.rs` — `FbWriter` `align(16)` + `text_buf` en premier, `BackBuf` wrapper
