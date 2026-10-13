# Kernel Build & Scripts Maintainer Notes

**MAINTAINERS Sections:** `KERNEL BUILD + files below scripts/ (unless maintained elsewhere)`

## Short-Lived Build Utilities (`scripts/`, `tools/objtool/`)

Programs under `scripts/` (such as `kallsyms`, `modpost`, `sorttable`, `depcheck`, `fixdep`) and `tools/objtool/` are short-lived command-line build utilities that run once per invocation and exit immediately.

- **Do NOT report as bugs:**
  - Memory leaks, `realloc()` pointer loss on OOM, or unclosed `FILE *` / file descriptors on fatal error paths or process exit where the OS immediately reclaims resources.
  - Hypothetical malformed CLI inputs that Kbuild Makefiles never pass (such as passing a literal `".cmd"` filename when Kbuild only passes `.<target>.cmd`).
  - Race conditions or skipped sub-makes that only occur under unsupported multi-goal command lines (e.g., `make -j modules_prepare tar-pkg` or `make -j vmlinux rustdoc`) when single goals work and `$(filter ...,$(MAKECMDGOALS))` is used as a multi-goal heuristic.
