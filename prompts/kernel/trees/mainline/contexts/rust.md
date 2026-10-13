# Kernel Rust Toolchain, Bindings, and Pin-Init (`rust/`)

## Build Environment, `CONFIG_*` Combinations, and FFI Types

- **Build and lint baseline:** Assume Rust patches compile and pass `clippy` on default configurations, and that unstable Rust features are enabled (`RUSTC_BOOTSTRAP=1`). Still check for conditional compilation errors or dead/unreferenced items under valid `CONFIG_*` combinations not covered by default builds.
- **`ffi::c_ulong` mapping:** In the Linux kernel, `unsigned long` always has the same width as `uintptr_t` and `size_t`, so `ffi::c_ulong` is mapped to `usize`.
- **`size_t`:** Represented as `usize` in Rust to match the architecture-dependent width of C's `size_t`.
- **`uintptr_t`:** Represented as `usize` in Rust, ensuring pointer-sized integer compatibility across 32-bit and 64-bit architectures.

## C Bindings and Helpers (`rust/helpers/`, `rust/bindings/bindings_helper.h`)

- **`__rust_helper` and `rust_helper_` prefix:** C helper functions in `rust/helpers/` wrap C inline functions or macros for Rust FFI. Every helper must be named with the `rust_helper_` prefix (which bindgen strips when exposing the function in `bindings::`) and annotated with `__rust_helper`.
- **`RUST_CONST_HELPER_`:** Complex C macro constants exposed in `rust/bindings/bindings_helper.h` use `const` definitions prefixed with `RUST_CONST_HELPER_` (stripped in `bindings::`).
- **Verifying `bindings::*` calls:** When auditing a `bindings::*` call inside an `unsafe` block, verify the preconditions, locking, and ownership rules against the underlying C definition rather than assuming userspace libc semantics.

## `build_assert!()` and `#[inline(always)]`

- Any function calling `build_assert!()` whose condition depends on function parameters **must** be annotated with `#[inline(always)]` so the compiler performs inlining and constant folding before evaluating the `build_assert!()`.
- **`build_error`:** The `rust/build_error.rs` module provides `build_error` to trigger compile-time panics. If a `build_assert!` condition cannot be evaluated at compile time, the compiler will fail to optimize away the call, resulting in a linker error.

## Pin Initialization (`pin_init!` / `try_pin_init!`)

- `try_pin_init!(Type { ... })` (fallible) and `pin_init!(Type { ... })` (infallible) initialize pinned structs in place:
  - Fields requiring in-place pinned initialization use `field <- init_expr`.
  - By-value fields use `field: value_expr`.
  - Previously initialized fields can be referenced by name in subsequent field initializers, and `_: { ... }` blocks can run arbitrary statements between fields.
- **Safety:** Ensure error paths in `try_pin_init!` correctly drop partially initialized objects to prevent memory leaks.

## Synchronization and Concurrency

- **Locking Contracts:** Rust abstractions for kernel locks must enforce the kernel's lock hierarchy.
- **Atomic Contexts:** Blocking operations (e.g., `alloc`, `sleep`) are strictly prohibited inside `SpinLock` or `RawSpinLock` critical sections.
- **Thread Safety:** Implement `Send` and `Sync` only if the underlying C structure is thread-safe.
- **Lockless Data:** Read-side critical sections must be bounded by the appropriate RCU read-side lock. Use `Opaque` or `UnsafeCell` for shared mutable state, ensuring access is mediated by safe wrappers enforcing kernel memory ordering.

## Initialization, Registration, and Teardown

- **Premature Publication:** Objects must be fully initialized and reference counts set to 1 before registration with C-side subsystems (e.g., `device_register`).
- **Symmetric Unwind:** Every `register` call must have a corresponding `unregister` or `put` call. Use `Drop` implementations to ensure resources are released in reverse order of allocation.
- **Quiescing:** During teardown, ensure all pending asynchronous work (e.g., `work_struct`, `timer_list`) is cancelled and handlers have finished before freeing memory.

## Error Handling and Boundary Validation

- **Error Propagation:** Translate internal Rust errors into standard kernel error codes (e.g., `-EINVAL`, `-ENOMEM`).
- **Pointer Sentinels:** Check `IS_ERR()` for pointers received from C before dereferencing. Wrap pointers in `Result` immediately upon entry.
- **Integer Overflows:** Use `checked_add`, `checked_mul`, etc., for buffer sizes or offsets derived from external inputs.
- **EOF Semantics:** `read` implementations must return `0` for EOF, not an error code.
- **Type Invariants:** Ensure that type conversions (e.g., from `c_int` to a bounded integer type) satisfy all internal invariants of the destination type to prevent undefined behavior.
- **Trait Sealing:** Traits that rely on specific internal invariants must be sealed to prevent external implementations from breaking safety guarantees.
- **Compiler Intrinsics:** Avoid `f32`, `f64`, `i128`, and `u128` types as they trigger panics in `compiler_builtins.rs` due to lack of kernel-space support. Use `core` primitives compatible with the target architecture.
