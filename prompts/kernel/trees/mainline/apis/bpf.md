# BPF Kfunc & Skeleton API Rules

## `__bpf_kfunc` Scalar/Enum Array Indexing (`include/linux/btf.h`)

- Non-`__const` scalar or `enum` parameters passed to a `__bpf_kfunc` are unconstrained `SCALAR_VALUE` registers at runtime. Always bounds-check them (including negative values for signed integers/enums) before indexing kernel arrays.
- Parameters with the `__const` suffix are enforced by the verifier to be compile-time constants (`tnum_is_const()`).

## BPF Skeleton Lifecycle (`bpftool gen skeleton`)

- After `<skel>__open_and_load()` returns a valid pointer (verified via `ASSERT_OK_PTR(skel, ...)`), **all** skeleton program pointers (`skel->progs.<name>`) and map pointers (`skel->maps.<name>`) are guaranteed to be loaded and to hold valid file descriptors (`>= 0`).
- **Do NOT flag missing FD checks** on `bpf_program__fd(skel->progs.<name>)` or `bpf_map__fd(skel->maps.<name>)` after a successful `<skel>__open_and_load()`.
- Manual lookup APIs (`bpf_object__find_program_by_name()`, `bpf_object__find_map_by_name()`) can return `NULL` and still require pointer/FD checks.
