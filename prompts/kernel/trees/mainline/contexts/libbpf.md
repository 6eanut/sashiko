# libbpf Public API Error Handling Context (`tools/lib/bpf/`)

## `errno` Wrapper Contract (`tools/lib/bpf/libbpf_internal.h`)

* **Public `LIBBPF_API` Functions Must Set `errno`**: Every public libbpf function (annotated with `LIBBPF_API` in `libbpf.h`, `bpf.h`, `btf.h`, `libbpf_legacy.h` or exported in `libbpf.map`) must set `errno` on every error return path right at the `return` statement using one of the three internal wrappers:
  * `return libbpf_err(ret);` — for integer-returning public APIs (when `ret < 0`, sets `errno = -ret` and returns `ret`).
  * `return libbpf_err_ptr(err);` — for pointer-returning public APIs failing with a negative errno code `err` (sets `errno = -err` and returns `NULL`). Public APIs must never return `ERR_PTR(...)` to callers.
  * `return libbpf_ptr(ptr);` — for pointer-returning public APIs wrapping an internal helper that returns `ERR_PTR(err)` (if `IS_ERR(ptr)`, sets `errno = -PTR_ERR(ptr)` and returns `NULL`; otherwise returns `ptr`).
* **Internal / Static Helpers**: Internal non-exported functions in `tools/lib/bpf/` use kernel-style negative error codes or `ERR_PTR()` directly and must **not** call `libbpf_err*()` wrappers.
