# Kconfig Code Context (`scripts/kconfig/`)

## Symbol Existence and Dependency Inheritance

* **Symbol Existence**: Every symbol referenced in `select FOO` or `depends on FOO` must match an existing `config FOO` definition in the tree (watch for prefix typos such as `QCM_` vs `QCS_` or `IMX8M_` vs `IMX8MM_`).
* **`select` Dependency Inheritance (`sym_warn_unmet_dep` in `scripts/kconfig/symbol.c`)**:
  * `select B` forces symbol `B` on without enforcing `B`'s `depends on` expression.
  * Whenever `config A` uses `select B`, `config A` must declare `depends on` constraints at least as restrictive as `config B`'s `depends on` (e.g., if `B` has `depends on ARM64 || COMPILE_TEST`, `A` must also have `depends on ARM64 || COMPILE_TEST`).
* **`COMPILE_TEST` with Architecture-Specific `select`**:
  * When a driver uses `depends on ARCH_FOO || COMPILE_TEST`, unconditional `select ARCH_SPECIFIC_SYM` breaks `COMPILE_TEST` builds on other architectures if `ARCH_SPECIFIC_SYM` depends on `ARCH_FOO`.
  * Either gate the selection (`select ARCH_SPECIFIC_SYM if ARCH_FOO`) or use `depends on ARCH_SPECIFIC_SYM`.
* **Cross-Config Family Consistency**: When a patch adds multiple related Kconfig entries (e.g., clock controllers for the same SoC family), verify that their `depends on` and `select` clauses are consistent across the group.
