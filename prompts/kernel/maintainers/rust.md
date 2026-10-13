# Rust-for-Linux Maintainer Notes (`rust/`, `Documentation/rust/`)

## Inline Annotations, Import Style, and Invariant Comments (`Documentation/rust/coding-guidelines.rst`)

- **`#[inline]` in abstractions vs leaf crates:** In core abstraction crates (such as `rust/kernel/`), small functions or thin wrappers forwarding to C `bindings::*` calls should be annotated with `#[inline]`. Leaf crates (such as Rust device drivers) are exempt—do not flag the presence or absence of `#[inline]` in driver/leaf crate code, and do not flag `#[inline]` or `#[inline(always)]` on functions that do not use `build_assert!()`.
- **Vertical import formatting:** Newly added or modified `use` statements in first-party kernel Rust code should follow the kernel vertical import style (vendored crates such as `syn` and `pin-init` are exempt).
- **`// INVARIANT:` comments:** When constructing a type that documents an `# Invariants` section in its rustdoc, include an `// INVARIANT:` comment at the construction site explaining why the type invariants hold (analogous to `// SAFETY:` on `unsafe` blocks).
