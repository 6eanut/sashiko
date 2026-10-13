# Objtool Control-Flow Validation (`tools/objtool/`)

## 1. Instruction Classification Semantics (`INSN_BUG` vs `INSN_TRAP`)

In `tools/objtool/arch/*/decode.c` and `tools/objtool/check.c`, `INSN_BUG` and `INSN_TRAP` have distinct control-flow effects:
- **`INSN_BUG`:** Sets `dead_end = true` in `decode_instructions()`, marking subsequent sequential instructions unreachable, suppressing unreachable-code warnings in `ignore_unreachable_insn()`, and matching indirect-call stop patterns in `validate_retpoline()`.
- **`INSN_TRAP`:** Treated as ignorable padding in `ignore_unreachable_insn()` (like `INSN_NOP`) without terminating control flow, and required after `ret` and indirect jumps in `validate_sls()` for Straight-Line Speculation (SLS) validation.

| Architecture | `INSN_BUG` | `INSN_TRAP` |
| :--- | :--- | :--- |
| x86 | `ud2`, `ud1`, `udb` | `int3` |
| LoongArch | `amswap.w $zero, $ra, $zero`; `break 0x1` | `break 0x0` |

## 2. Compiler-Generated Trap Instructions

Compiler passes such as `-fisolate-erroneous-paths-dereference` (enabled at `-O2`), `-fsanitize=unreachable`, and `-fsanitize=undefined` emit machine trap/break instructions independently of kernel `BUG()` macros. Reclassifying a trap instruction in `tools/objtool/arch/*/decode.c` may require pairing with compiler flags in `arch/*/Makefile` (such as `-fno-isolate-erroneous-paths-dereference` and `-mno-check-zero-division` in `arch/loongarch/Makefile`) so compiler-emitted traps do not trigger spurious objtool warnings.

## 3. Single-Invocation CLI Lifecycle and Threading

`objtool` is a short-lived build-time CLI tool that processes an object file and exits immediately, relying on the OS to reclaim process memory:
- **Do NOT report:**
  - Memory leaks on OOM or error-exit paths (such as `realloc()` overwriting a pointer on `NULL`, early error returns in `decode_file()` / `check()` skipping `free()`, or unfreed `insn_chunk` / hash allocations at exit).
  - Cleanup-path bugs reachable only if a startup heap allocation (`malloc` / `calloc`) fails under OOM.
  - Concurrent writes to a non-bitfield `bool` flag (such as `elf->changed` or `sec->_changed`) across worker threads when all threads only ever write the constant `true` (`1`) and the flag is read only after `pthread_join()`.

## 4. Core Data Structures and Invariants

- **`struct instruction`:** The fundamental unit of analysis. It tracks `type` (e.g., `INSN_CALL`, `INSN_BUG`), `dead_end` status, and `stack_ops`.
- **Symbol Resolution:** Must be deterministic. Verify the full key (name + scope/type) when resolving symbols to prevent hash collisions.
- **Noreturn Annotations:** `tools/objtool/noreturns.h` is the source of truth for CFG termination. Any new function that does not return (e.g., `rust_begin_unwind`, `panic`) must be added to this list.
- **Empty Sections:** Always check `sec->len == 0` or `sec->symbols == NULL` before iterating. Do not infer boundaries based on size; track the presence of the next entry explicitly.
- **Checksum Stability:** Checksums must be calculated on stable content, never on volatile metadata like jump offsets or section addresses that change during relocation.
- **Relocation Handling:** Ignore relocation types not intended for processing. Distinguish between module-specific exports and global vmlinux exports.
- **Format Strings:** All format strings passed to `warn()` or `error()` must be literal constants to prevent format injection.
- **Alignment:** Data added to binary sections must explicitly respect the alignment constraints of the target section to avoid corrupted string data and linker errors.

## 5. Architectural Invariants and Build-Time Safety

- **Deterministic Resolution:** Symbol lookup must verify the full key (name + scope) to avoid collisions. If a hash collision occurs, the lookup must verify the full key to ensure correctness.
- **Recursive Traversal:** When performing deep traversals (e.g., CFG analysis), minimize stack usage to prevent overflow.
- **Section Boundaries:** Do not rely on size-based inference for section boundaries. Explicitly track the presence of the next entry to avoid dragging in unrelated data.
- **Compiler Variations:** Symbol identification logic must account for variations in naming conventions across different compilers (e.g., GCC vs. Clang).
- **Discarded Sections:** References to discarded sections (e.g., `.exit.text`) must be properly excluded from build-time metadata generation to prevent linker errors.
- **Placeholder Symbols:** Internal placeholder symbols (e.g., tombstone symbols) must be marked to prevent them from being treated as exported symbols by the module loader.
- **Module Normalization:** Module identifiers must be normalized to their runtime names before being used for symbol resolution.
- **Build-Time Dependency:** The build system must verify both the presence of library binaries and the corresponding development headers to ensure `objtool` can link against required features.
- **Robust Metadata:** Avoid fuzzy heuristics for symbol extraction; rely on explicit ELF metadata to prevent misidentification of compiler-generated labels.
- **Context-Aware Metadata:** Derive build-time metadata from the context of the reference (e.g., the object being patched) rather than the definition of the target to ensure relocation accuracy.
- **Zero-Length Handling:** Explicitly skip relocations that cannot be resolved due to empty target sections to prevent null pointer dereferences.
- **Processing Phases:** Ensure all required processing phases are executed even if optional sections are absent, preventing early-exit bugs that skip critical logic.
- **Clean Targets:** All generated build artifacts must be included in the project's `clean` targets to prevent stale files from breaking incremental builds.
