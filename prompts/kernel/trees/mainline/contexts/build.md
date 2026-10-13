# Kernel Build System & Compiler Environment (`Makefile`, `scripts/`, `tools/`)

## Compiler Dialect & Flags (`Makefile`)

- **Language standard (`-std=gnu11`):** Both host programs (`KBUILD_USERCFLAGS`) and kernel code (`CC_FLAGS_DIALECT`) use `-std=gnu11`. Do not flag GNU C extensions (statement expressions, `typeof`, zero-length arrays, case ranges).
- **Unsigned `char` (`-funsigned-char`):** Top-level `Makefile` sets `KBUILD_CFLAGS += -funsigned-char` unconditionally across all architectures. Any check testing `char_var < 0` on a plain `char` in kernel space is always false (dead code / broken error check).
- **Strict aliasing (`-fno-strict-aliasing` vs `tools/`):** Kernel code builds with `-fno-strict-aliasing`, so type punning / pointer aliasing in kernel space is defined. Userspace tools under `tools/` default to standard `-fstrict-aliasing` unless overridden in their Makefile.
- **Boolean-to-pointer cast (`(void *)bool_val`):** Casting a C `_Bool` (`bool`) directly to `void *` does not trigger `-Wint-to-pointer-cast` in GCC or Clang; do not report it as a build warning.

## Host Environment, Python 3, and GNU Make Rules

- **Python 3 only:** All `.py` scripts (`scripts/`, `tools/`, `Documentation/`) target Python 3 (`Documentation/process/changes.rst`).
- **POSIX.1-2008 host environment:** Host utilities in `scripts/` and `tools/` target POSIX systems (`fopen(..., "w")` is identical to `"wb"`; `struct stat` nanosecond fields `st_mtim`/`st_atim`/`st_ctim` and standard coreutils like `seq`, `awk`, `sed`, `find`, `xargs`, `sort` are always available).
- **GNU Make empty target before single colon:** A rule `$(var): prerequisite` where `$(var)` expands to empty (`: prerequisite`) is silently ignored by GNU Make as a no-op; it does not fail with `missing target pattern` (which only applies to two-colon static pattern rules).
- **RISC-V `vmlinux` relaxation:** `arch/riscv/Makefile` passes `--no-relax` when linking `vmlinux`, so omitting `--emit-relocs` on intermediate `.tmp_vmlinux*` kallsyms passes does not alter symbol offsets.
