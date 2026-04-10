# Hardware Access

Patina components access hardware through several mechanisms, each with Rust-specific safety considerations that
differ from traditional C firmware. This section covers the supported access methods, the crates Patina uses for
each, and the pitfalls to avoid.

- [Memory-Mapped I/O (MMIO)](hardware_access/mmio.md)
