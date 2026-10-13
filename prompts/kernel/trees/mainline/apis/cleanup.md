# Scope-Based Cleanup & Guard API (`include/linux/cleanup.h`)

## `DEFINE_FREE` Guard Compatibility (`NULL` vs `ERR_PTR`)

When a variable is annotated with `__free(name)`, the cleanup wrapper runs at every scope exit. Verify that the `DEFINE_FREE` guard expression handles every value the variable can hold at any exit path:

- `__free(kfree)` (`include/linux/slab.h`): defined with `if (!IS_ERR_OR_NULL(_T)) kfree(_T)` — safe for both `NULL` and `ERR_PTR`.
- `__free(kfree_sensitive)` (`include/linux/slab.h`): defined with `if (_T) kfree_sensitive(_T)` — safe for `NULL` only, **crashes on `ERR_PTR`**.
- Custom `DEFINE_FREE(name, type, ...)`: if the initializer or any reassignment can produce an `ERR_PTR`, a wrapper that only checks `if (_T)` will pass the `ERR_PTR` (non-NULL) to the destructor on early return.

## LIFO Teardown Ordering (`guard()` vs `__free()`)

Scope-based cleanup attributes execute in **reverse order of declaration (LIFO)**:

- Any `guard()` lock required during a resource's destruction must be declared **before** the `__free()` variable so the destructor runs while the lock is still held.
- Declaring `struct obj *p __free(...) = NULL;` before `guard(mutex)(&lock);` and assigning `p` inside the locked region causes `p`'s destructor to run **after** the mutex is unlocked at scope exit.

## `guard()` vs `scoped_guard()` Lifetime

- `guard(lock_type)(&lock)` holds the lock until the end of the **enclosing lexical block** (not just the statement, and only until `}` if declared inside an `if`/`while` block). Accessing protected data after the block closes is a data race.
- `scoped_guard(lock_type, &lock)` holds the lock strictly for the immediately following statement or compound block.

## Ownership Transfer (`no_free_ptr()`, `return_ptr()`, `retain_and_null_ptr()`)

When transferring ownership of a `__free()` pointer to a caller or a consuming callee, cleanup must be inhibited to prevent double-free:

- `return_ptr(p)`: returns `p` and nulls out `p` (`return no_free_ptr(p)`).
- `no_free_ptr(p)`: `__must_check` helper that returns `p` and sets `p = NULL`.
- `retain_and_null_ptr(p)`: sets `p = NULL` without returning a value (use after a callee consumes `p` on success).

## Do Not Mix `goto` Unwind with `__free()` / `guard()`

Functions must not mix `goto` cleanup labels with `__free()` or `guard()` declarations. Jumping across or combining manual unwind labels with automatic scope destructors causes double-frees, premature lock releases, or skipped cleanups.

## High-Frequency Bug Patterns & Review Invariants

- **Initialization Requirement:** All variables annotated with `__free()` must be initialized at the point of declaration (e.g., `struct obj *p __free(kfree) = NULL;`). Failure to initialize leads to the destructor running on uninitialized stack memory if an early exit occurs.
- **Dangling Cleanup:** Never jump over a variable declaration with a `__free()` attribute using `goto`. This skips the initialization, leading to the destructor running on garbage stack data.
- **Mixing Manual and Automatic:** If converting a `goto`-based function to `__free()`, remove all manual `kfree()` or `put()` calls in the error paths.
- **Pointer vs. Address:** Apply `__free()` to the pointer variable itself. Applying it to the address of a pointer (e.g., `struct obj **p __free(...)`) passes the stack address to the destructor, causing memory corruption.
- **Atomic Context:** Do not use `guard()` or `scoped_guard()` with sleeping locks (e.g., `mutex_lock`) in atomic contexts. Use `spin_lock` variants if synchronization is required in atomic contexts.
- **Synchronous Cancellation:** When tearing down, ensure all asynchronous work (e.g., `work_struct`) is cancelled synchronously before the `__free()` destructor runs, preventing use-after-free if the worker accesses the object.
- **Reference Counting:** When using `__free()` for reference-counted objects (e.g., `pci_dev_put`), ensure the reference was successfully acquired. If the acquisition function returns an `ERR_PTR`, the `DEFINE_FREE` logic must handle it to avoid passing invalid pointers to the destructor.
- **Lock Hierarchy:** When using multiple `guard()` instances, maintain a strict, global lock acquisition order to prevent circular dependencies.
- **Conditional Locks:** Use `ACQUIRE()` and `ACQUIRE_ERR()` for conditional locking scenarios where the lock might not be acquired; ensure the subsequent code path checks the lock status before accessing protected resources.
- **Scope Granularity:** Prefer `scoped_guard()` for short critical sections to minimize lock hold time and reduce the risk of contention or deadlocks.
- **Control Flow Integrity:** Ensure `break` or `continue` statements do not bypass the scope of a `scoped_guard()`, as this may lead to premature lock release or unbalanced state.
- **Idempotency:** Ensure that cleanup functions defined via `DEFINE_FREE` are idempotent or that the guard logic correctly checks for null/invalid states to prevent double-free scenarios.
- **Resource Lifecycle:** When managing device-managed resources, ensure that manual cleanup is not performed on objects already registered with the `devm_` framework, as this will cause a double-free upon driver unbind.
- **Memory Ordering:** When publishing objects to shared structures, ensure appropriate memory barriers are used if the object is accessed via RCU or other lockless mechanisms.
- **Error Propagation:** Always propagate error codes correctly; do not return `NULL` when an `ERR_PTR` is expected, as this may cause the cleanup logic to skip necessary error handling.
