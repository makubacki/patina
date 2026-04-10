# Memory-Mapped I/O (MMIO)

This page explains the soundness problems with naive MMIO access in Rust and why Patina uses the
[`safe-mmio`](https://crates.io/crates/safe-mmio) crate.

## The Problem with References to MMIO Space

In C, casting an address to a pointer and performing volatile reads/writes is the standard way to access device
registers. In Rust, it might seem natural to create a reference (`&T` or `&mut T`) to a `#[repr(C)]` struct
laid out over the register block. However, **creating a Rust reference to MMIO space is unsound**.

The Rust compiler is free to insert additional reads through any `&T` reference at any time. For normal memory this
is harmless, but for MMIO registers an extra read can clear an interrupt status, pop a byte from a FIFO, or trigger
other device side-effects. More details are [documented](https://github.com/rust-embedded/volatile-register/issues/10)
and [tracked by the Rust Embedded Working Group](https://github.com/rust-embedded/wg/issues/791).

Because this is part of Rust's reference semantics, MMIO must be accessed exclusively through raw pointers combined
with volatile operations.

## Why `safe-mmio`

Patina uses the [`safe-mmio`](https://github.com/google/safe-mmio) crate because it directly addresses the following
concerns:

1. **No references to MMIO space.** `UniqueMmioPointer<T>` is a unique owned pointer to the registers of some MMIO
   device. It never creates a `&T` to device memory, eliminating potential for undefined behavior.

2. **Read side-effect distinction.** The crate separates read-with-side-effects (`ReadOnly` and `ReadWrite`
   requires `&mut UniqueMmioPointer`) from pure reads (`ReadPure` and `ReadPureWrite` allows `&UniqueMmioPointer`
   or `&SharedMmioPointer`). This encodes at the type level whether a read is safe to perform.

3. **Correct codegen on AArch64.** On AArch64, `read_volatile`/`write_volatile` can emit instructions that cannot be
   virtualized. `safe-mmio` works around this by using inline assembly to emit the correct MMIO instructions on
   AArch64.

### Field Wrapper Types

The below table is provided for quick Patina developer reference. See the
[safe-mmio documentation](https://docs.rs/safe-mmio/latest/safe_mmio) for full API details.

| Wrapper                                                                                           | Read | Write | Read Requires `&mut` | Notes                             |
|---------------------------------------------------------------------------------------------------|------|-------|----------------------|-----------------------------------|
| [`ReadPure<T>`](https://docs.rs/safe-mmio/latest/safe_mmio/fields/struct.ReadPure.html)           | yes  | no    | no                   | Pure read, no device side-effects |
| [`ReadOnly<T>`](https://docs.rs/safe-mmio/latest/safe_mmio/fields/struct.ReadOnly.html)           | yes  | no    | yes                  | Read has device side-effects      |
| [`WriteOnly<T>`](https://docs.rs/safe-mmio/latest/safe_mmio/fields/struct.WriteOnly.html)         | no   | yes   | n/a                  | Write-only register               |
| [`ReadWrite<T>`](https://docs.rs/safe-mmio/latest/safe_mmio/fields/struct.ReadWrite.html)         | yes  | yes   | yes                  | Read has side-effects             |
| [`ReadPureWrite<T>`](https://docs.rs/safe-mmio/latest/safe_mmio/fields/struct.ReadPureWrite.html) | yes  | yes   | no (for read)        | Read is pure, write allowed       |

See [safe-mmio usage instructions](https://github.com/google/safe-mmio?tab=readme-ov-file#usage) for examples of how
to define register blocks and access registers.

## Further Reading

- [`safe-mmio` README](https://github.com/google/safe-mmio) — full API overview and comparison with other MMIO crates
- [`safe-mmio` docs.rs](https://docs.rs/safe-mmio) — API reference
