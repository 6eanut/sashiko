# Kernel Build System & Compiler Environment (`Makefile`, `scripts/`, `tools/`)

## Compiler Dialect & Flags (`Makefile`)

- **Language standard (`-std=gnu11`):** Both host programs (`KBUILD_USERCFLAGS`) and kernel code (`CC_FLAGS_DIALECT`) use `-std=gnu11`. Do not flag GNU C extensions (statement expressions, `typeof`, zero-length arrays, case ranges).
- **Unsigned `char` (`-funsigned-char`):** Top-level `Makefile` sets `KBUILD_CFLAGS += -funsigned-char` unconditionally across all architectures. Any check testing `char_var < 0` on a plain `char` in kernel space is always false (dead code / broken error check).
- **Strict aliasing (`-fno-strict-aliasing` vs `tools/`):** Kernel code builds with `-fno-strict-aliasing`, so type punning / pointer aliasing in kernel space is defined. Userspace tools under `tools/` default to standard `-fstrict-aliasing` unless overridden in their Makefile.
- **Boolean-to-pointer cast (`(void *)bool_val`):** Casting a C `_Bool` (`bool`) directly to `void *` does not trigger `-Wint-to-pointer-cast` in GCC or Clang; do not report it as a build warning.

## Host Environment, Python 3, and GNU Make Rules

- **Python 3 only:** All `.py` scripts (`scripts/`, `tools/`, `Documentation/`) target Python 3 (`Documentation/process/changes.rst`).
- **POSIX.1-2008 host environment:** Host utilities in `scripts/` and `tools/` target POSIX systems (`fopen(..., "w")` is identical to `"wb"`; `struct stat` nanosecond fields `st_mtim`/`st_atim`/`st_ctim` and standard coreutils like `seq`, `awk`, `sed`, `find`, `xargs`, `sort` are always available).
- **GNU Make empty target before single colon:** A rule `$(var): prerequisite` where `$(var)` expands to empty (`: prerequisite`) is silently ignored by GNU Make as a no-op; it does not fail with `missing target pattern`.
- **RISC-V `vmlinux` relaxation:** `arch/riscv/Makefile` passes `--no-relax` when linking `vmlinux`, so omitting `--emit-relocs` on intermediate `.tmp_vmlinux*` kallsyms passes does not alter symbol offsets.

## Build-Time Invariants & ELF Parsing (`scripts/elf-parse.h`)

- **ELF Parsing (`recordmcount.h`):** `recordmcount.h` is included twice (with/without `RECORD_MCOUNT_64`). MIPS `_mcount` uses a fake symbol at `MIPS_FAKEMCOUNT_OFFSET` which must be filtered.
- **ELF Accessors:** Use `ehdr_shoff()`, `shdr_addr()`, and `sym_addr()` wrappers from `scripts/elf-parse.h` to handle 32/64-bit endianness and structure differences.
- **Section Updates:** When appending sections (e.g., `__mcount_loc`), `sh_link` and `sh_info` in `Elf_Shdr` must be explicitly updated to maintain ELF integrity.
- **Pointer-Sized Types:** When storing pointers in arrays, use `void *` or `uintptr_t`. Using `int` arrays for pointer storage causes stack corruption on 64-bit architectures.
- **Bounds Validation:** Always validate `Elf_Ehdr` and `Elf_Shdr` offsets against file size before dereferencing.

## High-Frequency Bug Patterns & Guards

| Bug Pattern | Guard / Prevention Rule |
| :--- | :--- |
| **Uninitialized Variable** | Initialize all local variables in `scripts/` to `0` or `NULL`. |
| **Missing Kconfig Dep** | Verify all new features have `Kconfig` entries; sort dependencies alphabetically. |
| **Stack Corruption** | Ensure `void *` return values are never stored in `int` arrays. |
| **Missing Cleanup** | Ensure `rm -f` is present for all temporary files in `Makefile` rules. |
| **Feature Detection** | Use `__GLIBC__` or specific feature macros rather than `_GNU_SOURCE`. |
| **Regex Escaping** | Escape `$` in `sed` scripts as `\$` to prevent shell variable expansion. |

## Verification Checklist for Patches
1. **New files:** Ensure addition to `Makefile`/`Kbuild` and `.gitignore`.
2. **Build scripts:** Check `$` escaping and temporary file cleanup.
3. **Structs:** If exposed to BPF/userspace, ensure size/padding for ABI stability.
4. **`char` usage:** Ensure no `< 0` checks are performed on `char` types.
5. **ELF parsing:** Verify `elf_parser` callbacks (e.g., `rela_write_addend`) are implemented for both 32/64-bit variants.
6. **Concurrency:** Ensure temporary files (e.g., `mksysmap` outputs) are unique per-process to prevent race conditions during `make -j`.
7. **Integer Overflows:** Validate additions in buffer size calculations before comparison.
8. **Dependency Scoping:** Ensure build-time tools (e.g., `kernel-doc`) are not executed during external module compilation.
9. **Feature Detection:** Verify that build checks verify both library binaries and development headers.
10. **Recursion:** Ensure object destruction logic (e.g., BPF object release) prevents unbounded recursion.
11. **Tooling:** Ensure `scripts/rust_is_available_bindgen_libclang.h` is used to validate clang versions for Rust support.
12. **ELF Integrity:** Use `rbe()` and `r2be()` for endian-safe reading of ELF headers.
13. **MIPS Ftrace:** Always filter `MIPS_FAKEMCOUNT_OFFSET` when processing `_mcount` symbols.
14. **Symbol Resolution:** Ensure `sym_type()` and `sym_addr()` are used to distinguish between function and data symbols during relocation processing.
15. **Memory Safety:** Validate `shdr_addr()` results against `Elf_Ehdr` bounds to prevent out-of-bounds reads.
16. **Build-time Macros:** Use `__maybe_unused` for helper functions in `scripts/` to avoid compiler warnings in partial builds.
17. **ABI Stability:** Ensure `Elf_Shdr` and `Elf_Sym` fields are accessed via `shdr_` and `sym_` wrappers to maintain 32/64-bit compatibility.
18. **Error Propagation:** Ensure `do_file()` return codes are checked to prevent silent build failures.
19. **Resource Management:** Ensure `new_shnum` calculation correctly accounts for added sections to prevent ELF header corruption.
20. **Tooling Consistency:** Verify that `scripts/` utilities do not rely on non-POSIX shell features.
