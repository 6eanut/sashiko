# libbpf Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for `tools/lib/bpf/`. It is intended for automated patch review and architectural validation.

## 1. `errno` Wrapper Contract (`tools/lib/bpf/libbpf_internal.h`)

*   **Public `LIBBPF_API` Functions Must Set `errno`**: Every public libbpf function (annotated with `LIBBPF_API` in `libbpf.h`, `bpf.h`, `btf.h`, `libbpf_legacy.h` or exported in `libbpf.map`) must set `errno` on every error return path right at the `return` statement using one of the three internal wrappers:
    *   `return libbpf_err(ret);` — for integer-returning public APIs (when `ret < 0`, sets `errno = -ret` and returns `ret`).
    *   `return libbpf_err_ptr(err);` — for pointer-returning public APIs failing with a negative errno code `err` (sets `errno = -err` and returns `NULL`). Public APIs must never return `ERR_PTR(...)` to callers.
    *   `return libbpf_ptr(ptr);` — for pointer-returning public APIs wrapping an internal helper that returns `ERR_PTR(err)` (if `IS_ERR(ptr)`, sets `errno = -PTR_ERR(ptr)` and returns `NULL`; otherwise returns `ptr`).
*   **Internal / Static Helpers**: Internal non-exported functions in `tools/lib/bpf/` use kernel-style negative error codes or `ERR_PTR()` directly and must **not** call `libbpf_err*()` wrappers.

## 2. Object Lifetimes and Memory Safety

*   **Symmetric Unwind**: Every `bpf_object__open` must have a corresponding `bpf_object__close`. Failure to do so leaks file descriptors and memory.
*   **Publication Boundary**: An object is "published" only after successful `bpf_object__load`. Do not attempt to attach or modify programs before this state is reached.
*   **Transient State**: Objects in a "transient" state (e.g., during `bpf_object__load`) must not be exposed to external iterators.
*   **Struct Extensions**: All user-provided buffers (e.g., `bpf_prog_load_opts`) must be validated for size and alignment. Use `bpf_prog_load_opts__last_field` to ensure compatibility with future struct extensions.

## 3. Synchronization and Concurrency

*   **Thread Safety**: `libbpf` is not thread-safe for concurrent modifications of the same `bpf_object`. Users must serialize access to `bpf_object` handles.
*   **Atomic Contexts**: `libbpf` interfaces with kernel BPF programs that may run in atomic contexts (e.g., `BPF_PROG_TYPE_KPROBE`). Ensure helper functions or kfuncs invoked by BPF programs are verified for sleepable/non-sleepable context compatibility.
*   **RCU Protection**: When accessing kernel-side RCU-protected lists, ensure `rcu_read_lock` is held. Missing `rcu_read_lock` is a common source of use-after-free vulnerabilities.

## 4. High-Frequency Bug Patterns & Review Invariants

*   **Sentinel Checks**: Always distinguish between `NULL` and `ERR_PTR` in internal logic. `IS_ERR` vs `NULL` check mismatches are prohibited.
*   **Integer Integrity**: 
    *   Use 64-bit types for buffer size calculations to prevent overflow.
    *   Validate that `max_entries` for maps does not exceed `UINT32_MAX`.
    *   Arithmetic in bounds checks must be protected against underflow. Use `unsigned` arithmetic; verify indices are clamped before use.
*   **Verifier State Invariants**:
    *   **Sign-Extension**: 32-bit keys must not be sign-extended to 64-bit during map lookups.
    *   **Recursion**: Object destruction logic must be iterative to prevent kernel stack exhaustion.
*   **False-Positive Guard**: During review, prioritize:
    1.  `libbpf_err` usage at the final `return` statement.
    2.  Missing `rcu_read_lock` if accessing kernel-side RCU-protected lists.
    3.  Unbalanced `refcount_dec` on error paths.
    4.  Lack of `bpf_prog_load_opts` field validation.

## 5. Untrusted Input and Callbacks

*   **Input Clamping**: All user-provided offsets and lengths must be clamped to the actual allocated size of the destination buffer.
*   **Format Strings**: All `bpf_printk` or logging calls must use literal format strings to prevent injection.
*   **Callback Identity**: If multiple callbacks share a resource, ensure they have distinct lock identities to prevent circular locking dependencies.
*   **Memory Initialization**: Recycled memory (e.g., per-CPU hash elements) must be fully zeroed or re-initialized before being exposed to a new context to prevent stale data leakage.

## 6. Architectural Invariants for v7.3-rc5

*   **BTF Relocations**: `__builtin_preserve_field_info` must be used with valid `bpf_field_info_kind` to ensure correct BTF relocation processing.
*   **Endianness**: Use `bpf_htons`, `bpf_htonl`, etc., for cross-platform byte order consistency. Do not rely on libc `__BYTE_ORDER` as it does not reflect the BPF target endianness.
*   **Prog Load Retries**: `bpf_prog_load_opts` allows for `attempts` configuration. Ensure that `EAGAIN` is handled gracefully by the loader logic.
*   **Log Buffers**: Use `BPF_LOG_BUF_SIZE` as a baseline for verifier log allocations, ensuring that output log sizes are tracked correctly via `bpf_prog_load_opts` or `bpf_btf_load_opts`.
