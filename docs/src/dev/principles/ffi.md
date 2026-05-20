# FFI Authoring

Patina exposes C FFIs for interoperability with C based drivers. This documentation gives guidance on how to write these
FFIs with patina specific guidance. For general guidance, view
[the Rust FFI docs](https://doc.rust-lang.org/nomicon/ffi.html).

## Patina Specific Considerations

### EFIAPI

All FFI functions must be marked with `extern "efiapi"` in order to use the EFIAPI ABI. Failure to include this
will cause undefined behavior.

>**Note:**: The one exception to this, temporarily, is for variadic functions. Rust only contains support for these
through the
[unstable c_variadics feature](https://doc.rust-lang.org/beta/unstable-book/language-features/c-variadic.html). This
feature only supports variadics for `extern "C"`. The UEFI spec has some variadic functions that are required. As such,
these functions use `extern "C"` for now. [This issue](https://github.com/r-efi/r-efi/issues/95) tracks adding support
for `extern "efiapi"`.

### va_list

AARCH64 defines the [AAPCS64 ABI](https://github.com/ARM-software/abi-aa/blob/main/aapcs64/aapcs64.rst) as the single
ABI for AARCH64. However, MSVC (and clang's aarch64-unknown-windows-msvc target to align to it) has broken this ABI in
some places, resulting in an MS ABI for AARCH64 as well.

The [aarch64-unknown-uefi target](https://doc.rust-lang.org/rustc/platform-support/unknown-uefi.html) used by Patina
uses the MS ABI because that is what LLVM supports for building PE/COFF images, which are required for UEFI. Within
Patina and Rust build code, this is immaterial, as long as the target is the same. However, it affects interoperability
with C based code built by gcc and clang's aarch64-linux-gnu target.

In most cases, the MS ABI aligns with the AAPCS64 ABI. However, an exception is the
[va_list type](https://github.com/ARM-software/abi-aa/blob/main/aapcs64/aapcs64.rst#142the-va_list-type). In order to
keep using the aarch64-unknown-uefi target and maintain compatibility with C based code, Patina does not allow using
the va_list type in FFIs. In practice, this has been seen very little.

>**Note:** This does not affect using variadic functions that take `...` as a parameter. Those are supported across
C FFIs. It is only the va_list type itself that cannot be passed by value or by reference.

### Pointer Alignment

Raw pointers received across an `extern "efiapi"` (or `extern "C"`) boundary must be treated as **potentially
unaligned**. C callers are not required to honor Rust's alignment invariants for `*mut T` / `*const T`, and the UEFI
specification often passes out-pointers whose alignment cannot be statically guaranteed (e.g. fields embedded in packed
C structures and addresses computed from byte offsets).

A misaligned read or write through a normal Rust dereference is undefined behavior even on architectures that
tolerate it in hardware. The compiler is also free to assume the pointer is aligned and reorder or coalesce accesses
based on that assumption. So, when working with caller-supplied pointers across an FFI boundary, you must use the
unaligned access intrinsics [`read_unaligned()`] and [`write_unaligned()`] or a safe abstraction like [`zerocopy`].

#### Rules

1. **Read FFI pointers with [`read_unaligned()`].** Do not use methods that assume alignment, such as `*ptr`, `ptr::read`,
   `&*ptr`, `(*ptr).field`, or `(*(ptr as *const T)).clone()` on a caller-supplied pointer.
2. **Write FFI pointers with [`write_unaligned()`].** Do not use `*ptr = value` or `ptr::write`.
3. **Be consistent within a function.** Mixing aligned and unaligned access on the same pointer is a code smell and
   risks UB if the pointer is in fact misaligned.
4. **Null-check before every unaligned access.** `read_unaligned()` and `write_unaligned()` only relax the *alignment*
   requirement. The pointer must still be non-null and valid for the access. A null check should be the first operation
   performed on an untrusted caller-supplied pointer before any other dereference or access, including unaligned ones.
5. **Prefer [`zerocopy`] for structured reads.** When pulling a typed value out of a caller-supplied byte buffer,
   `zerocopy::FromBytes` provides a safe, alignment-agnostic alternative to manual `read_unaligned()` of a casted
   pointer.

[`read_unaligned()`]: https://doc.rust-lang.org/std/primitive.pointer.html#method.read_unaligned()
[`write_unaligned()`]: https://doc.rust-lang.org/std/primitive.pointer.html#method.write_unaligned()
[`zerocopy`]: https://crates.io/crates/zerocopy

#### When direct dereference is acceptable

Direct dereference (`*ptr`, `&*ptr`, `&mut *ptr`) is sound when the pointer's provenance guarantees alignment. In
Patina, this is typically the case for:

- Pointers obtained from `Box::leak`, `Vec`, or other Rust allocators (where alignment is guaranteed by construction).
- The `this` pointer of an internally-produced protocol struct that Patina itself installed via `Box::leak`. The
  layout of `#[repr(C)]` protocol structs is well-defined and Patina-allocated instances are aligned.
- Pointers derived from `&T` / `&mut T` references obtained earlier in the same function.

When in doubt, prefer the unaligned variants as they have very little to no measurable cost on properly aligned
pointers on the targets Patina supports.

#### Anti-pattern examples

```rust,ignore
// WRONG: direct write through a caller-supplied out-pointer.
unsafe { *handle = installed_handle };

// WRONG: aligned read of a caller-supplied integer.
let n = unsafe { *num_bytes };

// WRONG: aligned struct read out of an arbitrary byte buffer.
let header = unsafe { (*(buffer as *const Header)).clone() };

// WRONG: inconsistent as the same pointer is read aligned and is written unaligned.
let n = unsafe { *num_bytes };
// ...
unsafe { num_bytes.write_unaligned()(actual) };
```

Replace each with the corresponding `read_unaligned()` / `write_unaligned()` form (and, for the struct case, a
`core::ptr::read_unaligned()::<Header>(buffer.cast())` or a `zerocopy::FromBytes` parse).
