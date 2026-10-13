# Perf Tools Internals (`tools/perf/`)

## `struct perf_tool` Callbacks, Pipe Mode, and `perf_env`

- **Callback registration (`struct perf_tool`):** Unregistered event callbacks are silently ignored. Any tool registering `.mmap` must also register `.mmap2` (and vice versa).
- **Pipe mode vs regular `perf.data` files:** Pipe-mode streams cannot seek and deliver header attributes (`perf_event_header_attr`) and features as synthesized events rather than file header sections. Tools supporting pipe mode must register attribute and feature callbacks so `evlist`/`evsel` and `struct perf_env` are populated, and all code reading `perf_env` fields must verify they have been initialized (as older `perf.data` files or pipe streams may omit newer feature sections).

## Build Feature Detection (`Makefile.config` and `HAVE_*_SUPPORT`)

- Feature probes under `tools/build/feature/test-*.c` are evaluated by `tools/perf/Makefile.config` to set `-DHAVE_*_SUPPORT` and `CONFIG_*` defines.
- Guard optional library usage with `#ifdef HAVE_*_SUPPORT` (or `CONFIG_*` build rules) and provide matching inline fallback stubs in headers (returning `-ENOTSUPP`, `-ENOSYS`, or `NULL`) when the feature is disabled.

## Reference-Counted Handle Wrappers (`DECLARE_RC_STRUCT` / `REFCNT_CHECKING`)

- Under `REFCNT_CHECKING` (enabled with ASAN/LSAN), reference-counted objects (`struct thread`, `struct maps`, `struct map`, `struct dso`, etc.) are wrapped in intermediary pointer handles via `DECLARE_RC_STRUCT`.
- Every handle acquired via `*_new()` or `*_get()` (e.g., `thread__get()`, `maps__get()`, `dso__get()`) must be paired with its matching `*_put()` (`thread__put()`, `maps__put()`, `dso__put()`), which frees the intermediary handle wrapper—never dereference a handle after passing it to `*_put()`, and never copy reference-counted pointers without `*_get()`.

## Directory Iteration Resource Cleanup (`openat` / `fdopendir` / `closedir`)

- When iterating `/proc` directories (such as `/proc/<pid>/fd` or `/proc/<pid>/fdinfo`) with `openat()` and `fdopendir()`, note that once `fdopendir(fd)` succeeds, the `DIR *` owns `fd` (closing it via `closedir()`), whereas if `fdopendir()` fails, `close(fd)` must be called directly. Ensure early callback error returns clean up all open descriptors and `DIR *` streams at every nesting level.
