# Kernel Rust Toolchain, Bindings, and Pin-Init (`rust/`)

## Build Environment, `CONFIG_*` Combinations, and FFI Types

- **Build and lint baseline:** Assume Rust patches compile and pass `clippy` on default configurations, and that unstable Rust features are enabled (`RUSTC_BOOTSTRAP=1`). Still check for conditional compilation errors or dead/unreferenced items under valid `CONFIG_*` combinations not covered by default builds.
- **`ffi::c_ulong` mapping:** In the Linux kernel, `unsigned long` always has the same width as `uintptr_t` and `size_t`, so `ffi::c_ulong` is mapped to `usize`.

## C Bindings and Helpers (`rust/helpers/`, `rust/bindings/bindings_helper.h`)

- **`__rust_helper` and `rust_helper_` prefix:** C helper functions in `rust/helpers/` wrap C inline functions or macros for Rust FFI. Every helper must be named with the `rust_helper_` prefix (which bindgen strips when exposing the function in `bindings::`) and annotated with `__rust_helper`.
- **`RUST_CONST_HELPER_`:** Complex C macro constants exposed in `rust/bindings/bindings_helper.h` use `const` definitions prefixed with `RUST_CONST_HELPER_` (stripped in `bindings::`).
- **Verifying `bindings::*` calls:** When auditing a `bindings::*` call inside an `unsafe` block, verify the preconditions, locking, and ownership rules against the underlying C definition rather than assuming userspace libc semantics.

## `build_assert!()` and `#[inline(always)]`

- Any function calling `build_assert!()` whose condition depends on function parameters **must** be annotated with `#[inline(always)]` so the compiler performs inlining and constant folding before evaluating the `build_assert!()`.

## Pin Initialization (`pin_init!` / `try_pin_init!`)

- `try_pin_init!(Type { ... })` (fallible) and `pin_init!(Type { ... })` (infallible) initialize pinned structs in place:
  - Fields requiring in-place pinned initialization use `field <- init_expr`.
  - By-value fields use `field: value_expr`.
  - Previously initialized fields can be referenced by name in subsequent field initializers, and `_: { ... }` blocks can run arbitrary statements between fields.
