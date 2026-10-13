# Objtool Control-Flow Validation (`tools/objtool/`)

## Instruction Classification Semantics (`INSN_BUG` vs `INSN_TRAP`)

In `tools/objtool/arch/*/decode.c` and `tools/objtool/check.c`, `INSN_BUG` and `INSN_TRAP` have distinct control-flow effects:
- **`INSN_BUG`:** Sets `dead_end = true` in `decode_instructions()`, marking subsequent sequential instructions unreachable, suppressing unreachable-code warnings in `ignore_unreachable_insn()`, and matching indirect-call stop patterns in `validate_retpoline()`.
- **`INSN_TRAP`:** Treated as ignorable padding in `ignore_unreachable_insn()` (like `INSN_NOP`) without terminating control flow, and required after `ret` and indirect jumps in `validate_sls()` for Straight-Line Speculation (SLS) validation.

| Architecture | `INSN_BUG` | `INSN_TRAP` |
| :--- | :--- | :--- |
| x86 | `ud2`, `ud1`, `udb` | `int3` |
| LoongArch | `amswap.w $zero, $ra, $zero`; `break 0x1` | `break 0x0` |

## Compiler-Generated Trap Instructions

Compiler passes such as `-fisolate-erroneous-paths-dereference` (enabled at `-O2`), `-fsanitize=unreachable`, and `-fsanitize=undefined` emit machine trap/break instructions independently of kernel `BUG()` macros. Reclassifying a trap instruction in `tools/objtool/arch/*/decode.c` may require pairing with compiler flags in `arch/*/Makefile` (such as `-fno-isolate-erroneous-paths-dereference` and `-mno-check-zero-division` in `arch/loongarch/Makefile`) so compiler-emitted traps do not trigger spurious objtool warnings.

## Single-Invocation CLI Lifecycle and Threading

`objtool` is a short-lived build-time CLI tool that processes an object file and exits immediately, relying on the OS to reclaim process memory:
- **Do NOT report:**
  - Memory leaks on OOM or error-exit paths (such as `realloc()` overwriting a pointer on `NULL`, early error returns in `decode_file()` / `check()` skipping `free()`, or unfreed `insn_chunk` / hash allocations at exit).
  - Cleanup-path bugs reachable only if a startup heap allocation (`malloc` / `calloc`) fails under OOM.
  - Concurrent writes to a non-bitfield `bool` flag (such as `elf->changed` or `sec->_changed`) across worker threads when all threads only ever write the constant `true` (`1`) and the flag is read only after `pthread_join()`.
