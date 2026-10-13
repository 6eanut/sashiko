# Perf Tools Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for the `tools/perf/` subsystem, ensuring memory safety, correct synchronization, and robust error handling in the Linux v7.3-rc5 codebase.

## 1. Core Data Structures & Object Lifetimes

### Reference-Counted Handle Wrappers (`DECLARE_RC_STRUCT`)
- Under `REFCNT_CHECKING` (enabled with ASAN/LSAN), reference-counted objects (`struct thread`, `struct maps`, `struct map`, `struct dso`, etc.) are wrapped in intermediary pointer handles via `DECLARE_RC_STRUCT`.
- **Ownership Contract:** Every handle acquired via `*_new()` or `*_get()` (e.g., `thread__get()`, `maps__get()`, `dso__get()`) must be paired with its matching `*_put()` (`thread__put()`, `maps__put()`, `dso__put()`).
- **Destruction Invariant:** Never dereference a handle after passing it to `*_put()`. Never copy reference-counted pointers without `*_get()`.
- **Publication Rule:** An object must be fully initialized and its reference count incremented before it is linked into a global list or made visible to other threads to prevent use-after-free (UAF) during concurrent access.

### `struct perf_tool` Callbacks and Pipe Mode
- **Callback registration (`struct perf_tool`):** Unregistered event callbacks are silently ignored. Any tool registering `.mmap` must also register `.mmap2` (and vice versa).
- **Pipe mode vs regular `perf.data` files:** Pipe-mode streams cannot seek and deliver header attributes (`perf_event_header_attr`) and features as synthesized events rather than file header sections. Tools supporting pipe mode must register attribute and feature callbacks so `evlist`/`evsel` and `struct perf_env` are populated, and all code reading `perf_env` fields must verify they have been initialized (as older `perf.data` files or pipe streams may omit newer feature sections).

## 2. Synchronization and Concurrency

- **RCU Usage:** All `rcu_dereference` calls must be enclosed within an `rcu_read_lock()` / `rcu_read_unlock()` critical section. Updates to RCU-protected pointers must use appropriate atomic assignment helpers.
- **List Iteration:** Never remove an element from a list while iterating over it without using safe iteration macros (e.g., `list_for_each_entry_safe`).
- **Reentrancy:** Buffer drain functions (e.g., PEBS processing) must be protected against re-entrancy. Use appropriate locking or state flags if functions are triggered from both hardware interrupt and user-space processing contexts.
- **Locking Hierarchy:** When multiple locks are required, maintain a strict acquisition order to prevent circular dependencies. If a lock must be acquired from both thread and IRQ contexts, ensure local interrupts are masked.

## 3. Initialization, Registration & Teardown

- **Directory Iteration (`openat` / `fdopendir` / `closedir`):** When iterating `/proc` directories (such as `/proc/<pid>/fd` or `/proc/<pid>/fdinfo`) with `openat()` and `fdopendir()`, note that once `fdopendir(fd)` succeeds, the `DIR *` owns `fd` (closing it via `closedir()`), whereas if `fdopendir()` fails, `close(fd)` must be called directly. Ensure early callback error returns clean up all open descriptors and `DIR *` streams at every nesting level.
- **Symmetric Unwind:** Every resource acquisition (memory allocation, `openat`, `fdopendir`) must have a corresponding cleanup path. Use `goto` labels for centralized, symmetric error unwinding.

## 4. Error Handling & Resource Recovery

- **IS_ERR vs NULL:** Distinguish between `NULL` (no object) and `ERR_PTR()` (error code). Do not use `if (!ptr)` to check for an error pointer; use `IS_ERR(ptr)`.
- **Integer Overflows:** Check for integer overflows before performing allocations for ring buffers or descriptor arrays. Account for metadata, headers, and alignment padding.
- **Input Validation:** Validate all user-provided offsets and lengths against current dynamic limits. Copy parameters into local memory before validation to prevent TOCTOU (Time-of-Check to Time-of-Use) races.

## 5. Build Feature Detection

- **Feature Probes:** Feature probes under `tools/build/feature/test-*.c` are evaluated by `tools/perf/Makefile.config` to set `-DHAVE_*_SUPPORT` and `CONFIG_*` defines.
- **Guard Invariant:** Guard optional library usage with `#ifdef HAVE_*_SUPPORT` (or `CONFIG_*` build rules).
- **Fallback Stubs:** Provide matching inline fallback stubs in headers (returning `-ENOTSUPP`, `-ENOSYS`, or `NULL`) when the feature is disabled to ensure build-time and runtime compatibility.

## 6. Hardware/Protocol Invariants

- **Affinity:** Respect `enum perf_affinity` (SYS, NODE, CPU) when configuring event scheduling to avoid cross-node latency or invalid CPU access.
- **Register Snapshots:** Populate CPU flags and register values from the hardware-provided snapshot rather than using default constants to ensure state accuracy.
- **Syscall Interface:** Use `sys_perf_event_open` for direct interaction with the kernel PMU interface, ensuring `struct perf_event_attr` is fully initialized to zero before setting specific fields.
- **Context Validation:** Ensure all pointers passed to callbacks (e.g., scheduler task callbacks) are validated against the specific PMU context (CPU-wide vs. task-specific) to prevent NULL dereferences.
- **State Consistency:** Metadata describing buffered data structures must be kept in sync with the actual data when structures are saved and later reprocessed; failure to do so leads to data corruption in tracers like `ftrace`.
