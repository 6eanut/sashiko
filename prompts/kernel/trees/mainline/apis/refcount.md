# Linux Kernel API Guide: `refcount_t` and `kref` (v7.3-rc5)

This guide provides authoritative rules for managing object lifetimes using the Linux kernel's reference counting primitives. These APIs are designed to prevent use-after-free (UAF) and double-free vulnerabilities through saturation semantics and strict lifecycle enforcement.

---

## 1. Core API Overview & Ownership
The kernel provides two primary abstractions: `refcount_t` (a hardened `atomic_t` variant) and `kref` (a wrapper for `refcount_t` that enforces a release callback).

### Ownership Rules
*   **Initialization:** Every object must be initialized via `refcount_set()` or `kref_init()` before being made visible to any global list, RCU-protected structure, or asynchronous worker.
*   **Lookup/Reference:** When a lookup yields a shared object, the caller **must** increment the reference count (e.g., `kref_get()` or `refcount_inc()`) before dropping the lock that protected the lookup.
*   **Asynchronous Handoff:** If an object is passed to an asynchronous worker (e.g., `work_struct`), the caller **must** increment the reference count before scheduling the work. The worker is then responsible for calling the corresponding `_put` function.
*   **Lifecycle Invariant:** An object must never be accessed after the final `refcount_dec_and_test()` or `kref_put()` returns `true` (indicating the object is being destroyed).

---

## 2. Calling Context, Locking & Synchronization
### Atomic Context Constraints
*   `refcount_inc()` and `refcount_dec()` are atomic and safe to call in any context, including hard IRQ and NMI.
*   `refcount_dec_and_mutex_lock()` and `kref_put_mutex()` are **strictly forbidden** in atomic context (e.g., hard IRQ, `in_atomic()`), as they acquire a `mutex`.
*   `refcount_dec_and_lock()` and `kref_put_lock()` are safe in atomic context provided the spinlock is not a sleeping lock (e.g., `raw_spinlock_t` or standard `spinlock_t` in non-RT kernels).

### Memory Ordering
*   `refcount_t` operations provide release memory ordering. This ensures that all prior stores to the object's data are visible to the thread that eventually performs the final `dec_and_test` and subsequent `kfree()`.

---

## 3. Return Values, Error Sentinels & Unwind Symmetry
### Error Path Discipline
*   **Unbalanced Refcounts:** Failing to decrement a reference count on an error path is a critical UAF/leak vector. If a function increments a refcount and subsequently fails a validation check, it **must** decrement the count before returning the error code.
*   **Partial Initialization:** If an object is partially initialized and fails, the cleanup path must be symmetric to the initialization path. If `kref_init` was called, the `release` function must be capable of handling the object in its partially-initialized state.

### Return Value Conventions
*   `kref_put()`: Returns `1` if the object was released, `0` otherwise. Never assume the object is still valid if `0` is returned; another thread may have concurrently dropped the final reference.
*   `kref_get_unless_zero()`: Returns `1` on success, `0` if the object is already being destroyed. This is the mandatory pattern for RCU-protected lookups.

---

## 4. High-Frequency Bug Patterns & Review Invariants

### Critical Review Invariants
1.  **The "Double-Check" Rule:** When using `kref_get_unless_zero()`, you must check the return value. If it returns `0`, the object is dead; do not attempt to access it.
2.  **The "Local Cache" Rule:** If you must access object metadata after dropping a reference, **cache the data locally** before calling `_put`. Accessing `obj->field` after `refcount_dec_and_test()` is a UAF.
3.  **The "Publication" Rule:** Never add an object to a global list or make it visible to other threads until the `refcount_t` is fully initialized.
4.  **The "Iteration" Rule:** When iterating over a list of refcounted objects, you must hold a reference to the *current* element before processing it. If the list is modified during iteration, use `list_for_each_entry_safe` and ensure the reference is held.

### Common Bug Patterns
*   **UAF on Teardown:** Failing to pin a parent object while a child object is being created or processed.
*   **Missing RCU Protection:** Dereferencing a pointer obtained via `rcu_dereference` without holding a reference or an `rcu_read_lock`.
*   **Unbalanced State Bits:** Setting a "busy" or "locked" bit to serialize work but failing to clear it on error paths, leading to permanent deadlocks.

---

## 5. Summary Table of API Usage

| Function | Context | Lock Held? | Purpose |
| :--- | :--- | :--- | :--- |
| `refcount_inc` | Any | No | Increment reference |
| `refcount_dec_and_test` | Any | No | Decrement and check if 0 |
| `refcount_dec_and_lock` | Any | No | Decrement; acquire spinlock if 0 |
| `kref_put_mutex` | Process | No | Decrement; acquire mutex if 0 |
| `kref_get_unless_zero` | Any | No | Safe lookup increment |

**Reviewer Note:** When reviewing a shared object whose struct embeds `refcount_t` or `struct kref` and has already been published, verify that any `kfree()` or `kmem_cache_free()` of that refcounted object occurs only after `refcount_dec_and_test()` returns `true` or inside the `kref_put()` release callback.
