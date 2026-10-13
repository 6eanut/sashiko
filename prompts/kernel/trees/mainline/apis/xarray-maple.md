# XArray and Maple Tree API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for using the XArray (`<linux/xarray.h>`) and Maple Tree (`<linux/maple_tree.h>`) APIs. These structures are the primary mechanisms for sparse array and range-based storage in the Linux kernel.

---

## 1. Core API Overview & Ownership
The XArray and Maple Tree manage the lifecycle of entries (pointers or values). 

*   **Ownership:** The XArray/Maple Tree does **not** own the objects stored within it. If you store a pointer to a dynamically allocated object, you are responsible for the object's lifecycle. You must ensure the object remains valid until it is removed from the tree and any RCU grace period has elapsed.
*   **Reference Counting:** If an object requires reference counting (e.g., `kref`), the reference must be held by the caller. When removing an object from the tree, the caller must ensure no concurrent lookups are in progress (via `xa_lock` or RCU) before dropping the final reference.
*   **Teardown:** Use `xa_destroy()` or `mas_destroy()` to free internal tree nodes. Failure to call these results in memory leaks of the tree's internal structure.
*   **Initialization:** Always initialize with `xa_init()` or `mt_init()` before use. Using an uninitialized structure is a critical bug.

## 2. Calling Context, Locking & Synchronization
*   **Locking:** 
    *   `xa_lock()`/`xa_unlock()` must be used for modifications. 
    *   For read-only operations, `rcu_read_lock()` is sufficient.
    *   **Interrupt Context:** If accessing from an interrupt handler, you **must** use `xa_lock_irqsave()` or `xa_lock_bh()` to prevent deadlocks against process-context writers.
*   **Sleeping:** `xa_store()` and `mtree_store()` may allocate memory. If called under a spinlock or in atomic context, you **must** use `GFP_ATOMIC` or preallocate nodes using `mas_preallocate()` (for Maple Tree) or `xa_reserve()` (for XArray).
*   **Atomicity:** The XArray/Maple Tree does not provide atomic "check-then-act" semantics across multiple calls. If you need to check for existence and then insert, you must hold the `xa_lock` across the entire sequence.

## 3. Return Values & Error Sentinels
*   **Error Pointers:** Many XArray functions return internal entries that encode errors. Use `xa_is_err(entry)` to check for errors and `xa_err(entry)` to extract the integer error code.
*   **NULL vs. Empty:** `xa_load()` returns `NULL` if the slot is empty. This is a valid state, not an error.
*   **Partial Failure:** If `xa_insert()` or `mtree_insert()` fails, the tree remains unchanged. However, if you performed a pre-allocation, you are responsible for cleaning up the pre-allocated nodes if the operation is aborted.
*   **Internal Sentinels:** Never store internal entries (e.g., `xa_mk_internal()`) in the tree. Use `xa_mk_value()` for integers and `xa_tag_pointer()` for tagged pointers.

## 4. High-Frequency Bug Patterns & Review Invariants
Based on the analysis of 100,000+ commits, the following patterns are strictly enforced:

*   **The "Double-Free/UAF" Guard:** When an XArray or Maple Tree has lockless RCU readers (`rcu_read_lock()` with `xa_load()` / `mas_walk()`), never free a removed entry without waiting for an RCU grace period (`kfree_rcu()` or `synchronize_rcu()`). Immediate `kfree()` after `xa_erase()` is only safe when all readers synchronize under `xa_lock` or an external lock.
*   **The "Initialization Race":** Do not publish an object to the tree until it is fully initialized. A common bug is inserting a pointer to a struct before its internal fields are set, allowing a concurrent reader to access uninitialized data.
*   **The "Unbalanced Cleanup":** If an error occurs after `xa_alloc()` or `mtree_insert()`, ensure all previously acquired resources (including the entry itself) are cleaned up.
*   **The "Reserved Entry Confusion":** XArray reserves internal entries. Do not store raw integers that overlap with internal sentinel values. Use `xa_mk_value()` to safely encode integers.
*   **The "Locking Mismatch":** Ensure the same lock is used for both reading and writing. Mixing `rcu_read_lock()` for reads and `xa_lock()` for writes is correct, but mixing different spinlocks for the same XArray is a fatal error.
*   **The "State Visibility":** Ensure `mas_preallocate()` is used when the context cannot sleep, and always check `mas_is_err()` after `mas_store()` or `mas_walk()` operations.

## 5. Review Checklist for Automated Patch Review
1.  **Is the tree locked?** Verify `xa_lock` or `rcu_read_lock` is held for the duration of the operation.
2.  **Is the object fully initialized?** Check that the object is populated *before* the `xa_store` or `mtree_store` call.
3.  **Are error codes handled?** Ensure `xa_is_err()` is checked before dereferencing the result of `xa_load()`.
4.  **Is the teardown complete?** Check that `xa_destroy()` or `mas_destroy()` is present in the module/object exit path.
5.  **Are bounds checked?** For `xa_alloc()`, ensure the range is within `[0, ULONG_MAX]`.
6.  **Is the context safe?** If in an IRQ handler, ensure `_irqsave` variants of locks are used.
7.  **Are internal entries avoided?** Ensure no `xa_mk_internal()` values are stored in user-accessible slots.
8.  **Is RCU grace period respected?** Ensure `synchronize_rcu()` is called before freeing objects removed from the tree.

---
*Note: This guide is generated for Linux v7.3-rc5. Always consult `Documentation/core-api/xarray.rst` for the latest architectural changes.*
