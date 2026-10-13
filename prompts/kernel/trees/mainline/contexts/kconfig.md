# Kconfig Code Context (`scripts/kconfig/`)

## 1. Symbol Existence and Dependency Inheritance

*   **Symbol Existence**: Every symbol referenced in `select FOO` or `depends on FOO` must match an existing `config FOO` definition in the tree (watch for prefix typos such as `QCM_` vs `QCS_` or `IMX8M_` vs `IMX8MM_`).
*   **`select` Dependency Inheritance (`sym_warn_unmet_dep` in `scripts/kconfig/symbol.c`)**:
    *   `select B` forces symbol `B` on without enforcing `B`'s `depends on` expression.
    *   Whenever `config A` uses `select B`, `config A` must declare `depends on` constraints at least as restrictive as `config B`'s `depends on` (e.g., if `B` has `depends on ARM64 || COMPILE_TEST`, `A` must also have `depends on ARM64 || COMPILE_TEST`).
*   **`COMPILE_TEST` with Architecture-Specific `select`**:
    *   When a driver uses `depends on ARCH_FOO || COMPILE_TEST`, unconditional `select ARCH_SPECIFIC_SYM` breaks `COMPILE_TEST` builds on other architectures if `ARCH_SPECIFIC_SYM` depends on `ARCH_FOO`.
    *   Either gate the selection (`select ARCH_SPECIFIC_SYM if ARCH_FOO`) or use `depends on ARCH_SPECIFIC_SYM`.
*   **Cross-Config Family Consistency**: When a patch adds multiple related Kconfig entries (e.g., clock controllers for the same SoC family), verify that their `depends on` and `select` clauses are consistent across the group.

## 2. Subsystem Architecture & Core Data Structures

*   **`struct symbol`**: Tracks `type` (bool, tristate, string, hex, int), current calculated value (`curr`), and user-provided values (`def[S_DEF_USER]`).
    *   `dir_dep`: Direct dependencies (`depends on`).
    *   `rev_dep`: Reverse dependencies (`select`).
    *   `implied`: Weak reverse dependencies (`imply`).
*   **`struct expr`**: Represents the logic tree. Tristate logic follows `EXPR_OR` (max) and `EXPR_AND` (min) macros in `expr.h`.
*   **`struct menu`**: Represents the hierarchical structure. `menu_finalize()` propagates dependencies from parent menus/ifs to child symbols.

## 3. Dependency Resolution & `select` Contracts

*   **Tristate Propagation**: A `tristate` symbol selected by a `bool` symbol is forced to 'y'. Ensure C code handles this via `IS_REACHABLE()` or `IS_ENABLED()`.
*   **Missing `select` for Libraries**: If a driver uses `regmap`, `clk`, or `gpio` APIs, the Kconfig entry must `select` the corresponding subsystem (e.g., `select REGMAP_I2C`).
*   **Module vs. Built-in**: If `config A` is `y` and `config B` is `m`, and `A` calls `B`'s functions, `B` must be exported via `EXPORT_SYMBOL`. Kconfig dependencies must prevent `A=y` and `B=m` if link-time requirements are not met.

## 4. Initialization and Parsing

*   **Symbol Visibility**: A symbol's visibility is calculated based on its prompt's dependencies. If a symbol lacks a prompt, its visibility is 'n'.
*   **`menu_finalize()`**: The dependency graph is incomplete until this is called. Do not evaluate symbol values or perform consistency checks before the menu tree is finalized.
*   **Error Unwind**: The parser uses `zconf_initscan()` and `zconf_nextfile()`. Ensure custom scripts interacting with `scripts/kconfig/` handle non-zero exit codes.

## 5. Scripting & Tooling Invariants

*   **POSIX Compliance**: Scripts must use POSIX-compliant shell syntax. Use `=` instead of `==` in `if` statements.
*   **Atomic File Operations**: When using `awk` or `sed` to process Kconfig files, use a temporary file and `mv` to ensure atomicity. Never use the same file as both input and output.
*   **Naming**: Do not prefix symbol names with `CONFIG_` inside Kconfig files; this is handled by the build system.

## 6. Verification Checklist

1.  **Dependency Completeness**: Does the patch `select` all required library dependencies?
2.  **`COMPILE_TEST` Safety**: Is hardware-specific logic gated by `depends on ARCH_FOO || COMPILE_TEST`?
3.  **Symbol Pruning**: Are `select` statements and `config` entries removed when features are deleted?
4.  **Redundancy**: Are there duplicate `depends on` clauses that can be simplified?
5.  **Integrity**: Run `make oldconfig` and `make allyesconfig` after modifying Kconfig files to verify dependency integrity.
6.  **Makefile Mapping**: Does the `Makefile` correctly map the Kconfig tristate symbol to the object files required for module compilation?
7.  **Pointer Validation**: Are all pointers validated before being passed to formatting functions (e.g., `str_printf`) to prevent NULL dereferences?
8.  **Compiler Versioning**: Are compiler version checks based on the actual release version where a fix was merged?
9.  **Static Linking**: Does the build system ensure that dependent libraries are linked in the correct order for static builds?
10. **Documentation Accuracy**: Do the Kconfig help strings accurately reflect the valid input range and current implementation status?
