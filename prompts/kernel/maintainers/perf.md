# Perf Tools Maintainer Notes (`tools/perf/`)

## Cross-Platform Analysis vs `tools/perf/arch/`

- **Keep decoding and analysis out of `tools/perf/arch/`:** Code placed under `tools/perf/arch/` is only compiled into host binaries for that architecture, breaking cross-platform `perf report` / `perf script` analysis of `perf.data` files recorded on other architectures.
- `tools/perf/arch/` must only contain logic strictly tied to live host execution (such as native PMU probing or host register access). For architecture-dependent decoding or analysis, place the code in generic `tools/perf/util/` and dispatch dynamically on the ELF machine constant (`e_machine`) from `struct perf_env`, `perf_session`, `machine`, `thread`, or `evsel`.

## `ERR_PTR` Avoidance in Userspace `perf`

- **Do not introduce `ERR_PTR()` in new `tools/perf/` code:** Unlike kernel code, userspace `perf` code predominantly uses `NULL` checks, so `ERR_PTR()` returns frequently lead to unguarded `ERR_PTR` dereferences.
- Return `NULL` from pointer-returning functions on error, or return a negative errno (`int`) and pass the allocated pointer via an output parameter (`struct foo **out`). When calling legacy helpers that still return `ERR_PTR()`, check with `IS_ERR()` / `PTR_ERR()`.

## Explicit POSIX Header Inclusions (`musl` Compatibility)

- `tools/perf/` is built against both `glibc` and `musl`. Do not rely on transitive header inclusions from `glibc`; explicitly `#include` the POSIX header for every libc symbol used (`<unistd.h>`, `<stdio.h>`, `<stdlib.h>`, `<string.h>`, `<limits.h>`, etc.).
- In `.h` files, prefer forward declarations (`struct evlist;`, `struct evsel;`) over full header includes when only pointer types are referenced.
