# XFS Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for the XFS filesystem subsystem. It is intended for automated patch review and architectural validation.

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **Core Objects:** The primary objects are `xfs_mount` (filesystem instance), `xfs_inode` (file/directory metadata), `xfs_trans` (atomic transaction), and `xfs_buf` (buffer cache).
*   **Object Lifetimes:**
    *   `xfs_inode`: Managed via `xfs_inode_cache`. Lifecycle is governed by `VFS` inode reference counting (`i_count`) and XFS-internal `i_rwsem` and `i_ilock`.
    *   `xfs_attri_log_item` / `xfs_attrd_log_item`: These use `refcount_t` (`attri_refcount`) and `xfs_attri_log_nameval` structures. The `nameval` structure holds the actual attribute data and is freed only when the `refcount` reaches zero, ensuring that log recovery and deferred operations do not suffer from use-after-free.
*   **Invariants:**
    *   Never access `xfs_inode` fields without holding the appropriate `xfs_ilock` (shared or exclusive) or the VFS `i_rwsem`.
    *   `xfs_buf` objects must be released via `xfs_buf_relse()` or `xfs_buf_stale()` to ensure they return to the cache correctly.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Lock Hierarchy:**
    1.  `xfs_mount` (sb_lock)
    2.  `xfs_inode` (i_ilock)
    3.  `xfs_buf` (b_lock)
    *   *Rule:* Never acquire a higher-level lock while holding a lower-level lock in this hierarchy.
*   **Concurrency Rules:**
    *   `xfs_ilock` is a sleepable semaphore. It **must not** be held in atomic contexts (e.g., interrupt handlers or with `preempt_disable()`).
    *   `xfs_buf` locking: Use `xfs_buf_lock()` for exclusive access. If a buffer is needed for read-only metadata, ensure the buffer is pinned or the transaction is active to prevent eviction.
*   **RCU Usage:** `xfs_get_acl` supports RCU-walk mode. When `rcu` is true, the function must not sleep and must use `rcu_read_lock()` to protect the inode's ACL pointer.

## 3. Initialization, Registration & Teardown Ordering

*   **Initialization:** Subsystem components (e.g., `xfs_attri_cache`) are initialized via `kmem_cache_create`. Failure to initialize any cache during module load must trigger a full unwind of all previously allocated caches to prevent memory leaks.
*   **Teardown:**
    *   All deferred work (e.g., `xfs_attr_defer_add`) must be flushed or cancelled before the `xfs_mount` structure is torn down.
    *   `xfs_forget_acl` must be called during inode eviction to ensure no stale ACL references persist in the VFS cache.

## 4. Error Handling, Resource Recovery & Boundary Validation

*   **Error Pointer Sentinels:** Always use `IS_ERR()` to check return values from metadata lookups (e.g., `xfs_iget`). Do not assume `NULL` is the only failure indicator.
*   **Unwind Symmetry:**
    *   If a transaction is started (`xfs_trans_alloc`), it **must** be either committed (`xfs_trans_commit`) or cancelled (`xfs_trans_cancel`). Failure to do so leaks the transaction and associated log reservations.
    *   Every `xfs_ilock` must have a corresponding `xfs_iunlock`.
*   **Empirical Bug Guard:** 100k-fix analysis shows frequent "IS_ERR vs NULL" mismatches. Always verify: `if (IS_ERR(ptr)) return PTR_ERR(ptr);` before checking `if (!ptr)`.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **Use-After-Free (UAF):**
    *   *Pattern:* Accessing an inode after `xfs_irele` or `iput`.
    *   *Guard:* Ensure the inode reference count is held across the entire duration of the operation.
*   **Integer Overflows:**
    *   *Pattern:* Calculating buffer sizes for `xfs_attri_log_nameval` using user-provided lengths.
    *   *Guard:* Use `check_add_overflow()` or `check_mul_overflow()` when calculating allocation sizes for variable-length attribute names/values.
*   **Unbalanced Refcounts:**
    *   *Pattern:* Error paths in `xfs_set_acl` failing to drop the `posix_acl` reference.
    *   *Guard:* Use `posix_acl_release()` on all error exit paths if the ACL was successfully retrieved.
*   **Stale State:**
    *   *Pattern:* Reusing `xfs_da_args` without re-initializing fields.
    *   *Guard:* Always zero-initialize or explicitly set all fields in `xfs_da_args` before passing to `xfs_attr_defer_add`.

## 6. Untrusted Input Boundaries

*   **Validation:** All `xfs_attr` operations must validate the `name` and `value` lengths against `XFS_XATTR_SIZE_MAX`.
*   **Alignment:** Ensure all structures shared with user-space (via `ioctl`) are padded to 8-byte boundaries to prevent information leaks from uninitialized stack memory.
