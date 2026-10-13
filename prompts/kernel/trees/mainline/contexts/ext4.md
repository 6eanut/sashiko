# Ext4 and JBD2 Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative architectural invariants and bug-prevention rules for the Ext4 filesystem and its journaling layer (JBD2).

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **JBD2 Handle Lifecycle:** A `handle_t` represents a single atomic transaction update. It must be started via `jbd2_journal_start()` and stopped via `jbd2_journal_stop()`.
    *   **Invariant:** Every `jbd2_journal_start()` must have a corresponding `jbd2_journal_stop()` on all paths, including error exits. Failure to do so leads to transaction leaks and filesystem deadlock.
*   **Inode Info:** `struct ext4_inode_info` is the core in-memory representation. It contains the `i_data` extent tree root.
*   **Extent Tree:** The tree is managed via `struct ext4_ext_path`.
    *   **Lifecycle:** Paths are allocated during lookup/modification and must be freed via `ext4_ext_drop_refs()` to prevent memory leaks and dangling buffer head references.
*   **Object Ownership:** The VFS layer owns the `inode` and `dentry` objects. Ext4 manages the private `ext4_inode_info` embedded within the `inode`. Destruction of the `inode` triggers `ext4_evict_inode()`, which must ensure all JBD2 handles are closed and metadata is flushed.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Lock Hierarchy:**
    1.  `s_umount` (VFS level)
    2.  `i_rwsem` (Inode level)
    3.  `j_checkpoint_mutex` (JBD2 checkpointing)
    4.  `j_list_lock` (JBD2 list protection)
    *   **Rule:** Never acquire `j_checkpoint_mutex` while holding `i_rwsem` if the reverse order is possible elsewhere.
*   **Atomic vs. Blocking:**
    *   `jbd2_journal_start()` can sleep. It must **never** be called from an atomic context (e.g., interrupt handlers or with spinlocks held).
    *   `j_list_lock` is a spinlock. Operations inside this critical section must be non-blocking.
*   **Read-Copy-Update (RCU):** Used for ACLs and certain lookup paths. Read-side critical sections are bounded by `rcu_read_lock()`/`rcu_read_unlock()`. Updates must use `synchronize_rcu()` or `call_rcu()` to ensure safe reclamation.

## 3. Initialization, Registration & Teardown Ordering

*   **Initialization:** Ext4 initialization (`ext4_fill_super`) follows a strict sequence:
    1.  Read superblock.
    2.  Initialize JBD2 journal.
    3.  Mount orphan list cleanup.
    4.  Enable features.
    *   **Bug Pattern:** Setting feature flags after initialization can lead to inconsistent state. Always validate feature compatibility before setting flags.
*   **Teardown:** The `kill_sb` path must ensure all pending asynchronous work is flushed before the journal is destroyed.

## 4. Error Handling, Resource Recovery & Boundary Validation

*   **Error Pointer Sentinels:** Functions returning `struct *` (e.g., `ext4_get_acl`) must be checked using `IS_ERR()`.
    *   **Rule:** Never check `if (ptr == NULL)` for functions that return `ERR_PTR`.
*   **Transaction Credits:**
    *   **Invariant:** Always reserve sufficient credits via `jbd2_journal_start()` for the *worst-case* scenario of an operation (e.g., extent tree split). Underestimating credits causes `jbd2_journal_restart()` failures or transaction overflows.
*   **Symmetric Unwind:** Use the "goto out" pattern for error paths. Ensure every `ext4_journal_start` is balanced by `ext4_journal_stop`.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **The "Double-Free" Guard:** When an error occurs during `ext4_ext_insert_extent`, ensure the `ext4_ext_path` is freed exactly once.
*   **The "Stale Pointer" Guard:** After `jbd2_journal_stop()`, the handle is invalid. Do not access `handle->h_transaction` post-stop.
*   **The "Integer Overflow" Guard:** When calculating `ext4_acl_size` or buffer offsets, use `size_t` and check for overflows before allocation.
*   **The "State Machine" Guard:**
    *   **Fastcommit Replay:** Ensure the filesystem is in a "replaying" state before allowing metadata modifications.
    *   **Read-Only Mounts:** Orphan cleanup must be skipped or handled gracefully on read-only mounts to avoid spurious warnings.
*   **Diagnostic Gating:** Use `ext4_debug` or `jbd2_debug` for transient issues. Ensure error messages are rate-limited to prevent log flooding during disk corruption events.

## 6. Critical Invariants for v7.3-rc5

*   **Extent Tree Depth:** `EXT4_MAX_EXTENT_DEPTH` is 5. Any traversal logic must handle this limit to prevent stack overflows.
*   **Buffer Head Ownership:** When a buffer is attached to a transaction, JBD2 assumes ownership. Do not release the buffer head manually until the transaction is committed or aborted.
*   **Journal Space:** `__jbd2_log_wait_for_space` must be called under `j_state_lock`. It is the primary mechanism to prevent journal exhaustion.
