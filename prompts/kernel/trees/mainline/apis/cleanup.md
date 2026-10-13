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

Functions must not mix `goto` cleanup labels with `__free()` or `guard()` declarations, as jumping across or combining manual unwind labels with automatic scope destructors causes double-frees, premature lock releases, or skipped cleanups.
