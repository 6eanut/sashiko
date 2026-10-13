# VFS Path Walk, Open, and Superblock Internals (`fs/namei.c`, `fs/open.c`, `fs/dcache.c`)

## Path Walking Modes: RCU-Walk vs REF-Walk (`fs/namei.c`)

- **RCU-walk (`LOOKUP_RCU`):** Traverses dentries and mounts locklessly under `rcu_read_lock()`, validating `mount_lock` and per-dentry `d_seq` seqlocks (`lookup_fast()`). Returns `-ECHILD` on any seqlock mismatch or when a sleeping filesystem operation is needed.
- **REF-walk:** Holds `d_lockref` dentry references and per-CPU `mnt_count` vfsmount references, acquiring shared `dir->i_rwsem` on dcache misses (`lookup_slow()`).
- **One-way transition (`try_to_unlazy()`):** Transitions an active RCU-walk to REF-walk by acquiring dentry/vfsmount references and validating `d_seq` before dropping `rcu_read_lock()`. If validation fails, `try_to_unlazy()` returns `false` and the caller returns `-ECHILD` so `filename_lookup()` restarts the walk in REF-walk mode (and retries with `LOOKUP_REVAL` on `-ESTALE`).

## File Open, `i_rwsem`, and Superblock `s_umount` Lifecycle

- **`do_dentry_open()` and `file->f_op` (`fs/open.c`):** `O_PATH` files receive `&empty_fops` (not `NULL`); regular opens acquire `fops_get(inode->i_fop)` (failing with `-ENODEV` if `NULL`). `file->private_data` is not freed automatically by the VFS and must be released in `f_op->release()`.
- **State re-validation across lock drops:** Whenever `i_rwsem` or another VFS lock is dropped and re-acquired, re-validate dentry positive/negative state, hashing (`!d_unhashed()`), parent identity, and inode state (`I_FREEING` / `S_DEAD`).
- **Superblock `sb->s_umount`:** Held exclusive (`down_write(&sb->s_umount)`) across `put_super`, `freeze_fs`, `unfreeze_fs`, and `remount_fs`, and held shared (`down_read(&sb->s_umount)`) across `sync_fs`.

## Core Invariants and Synchronization

- **Dcache/Icache Relationship:** The dcache is the master of the icache. An inode exists if and only if at least one dentry points to it, or it is currently open. `iput()` is triggered by dentry eviction or explicit release.
- **`file` Object Lifecycle:** `struct file` is `SLAB_TYPESAFE_BY_RCU`. When accessing files via RCU, callers must handle spurious refcount bumps. Always re-validate the file state after a successful `atomic_long_inc_not_zero()`.
- **Lock Hierarchy:**
    1. `inode->i_lock` (protects `i_dentry`, `d_alias`)
    2. `dentry->d_lock` (protects `d_flags`, `d_name`, `d_count`, `d_unhashed()`)
    3. `sb->s_dentry_lru_lock`
    4. `dcache_hash_bucket` lock
    5. `s_roots_lock`
- **Superblock Readiness:** Use `super_lock()` to wait for `SB_BORN` or `SB_DYING`. Never assume a superblock is ready until `SB_BORN` is set.

## High-Frequency Bug Patterns & False-Positive Guards

- **Sleeping in Atomic:** Never perform `kmalloc(..., GFP_KERNEL)` or `mutex_lock()` inside `rcu_read_lock()` or while holding `d_lock`.
- **Double Unlock/Double Free:** Ensure `f_op->release()` is the sole owner of `file->private_data` cleanup. Do not free it in the error path of `open()` if the file was successfully installed in the descriptor table.
- **Integer Overflows:** Use `check_add_overflow()` or `check_mul_overflow()` for buffer size calculations in `vfs_read` or `vfs_write`.
- **`d_seq` Mismatch:** A mismatch in `d_seq` during RCU-walk is a signal to retry, not an error.
- **`O_PATH`:** Files opened with `O_PATH` have `&empty_fops`. Check `file->f_mode & FMODE_PATH` before invoking `f_op` methods.
- **ID-Mapping:** When creating files (e.g., `O_TMPFILE`), verify that the caller's credentials map correctly into the filesystem's `mnt_idmap` to prevent privilege escalation.

## Architectural Review: Addressing Subsystem Invariants

- **Error Unwinding:** Every `vfs_*` operation must ensure that if an error occurs after a successful `get_write_access()` or `mnt_want_write()`, the corresponding `put_write_access()` or `mnt_drop_write()` is invoked.
- **Locking Contexts:** Blocking operations (e.g., `inode_lock()`, `mutex_lock()`) are strictly forbidden in `LOOKUP_RCU` mode. If a path walk requires a lock, it must transition to REF-walk via `try_to_unlazy()`.
- **Object Lifetime:** `struct dentry` and `struct inode` are protected by RCU. When traversing lists, ensure the RCU read-side critical section covers the entire traversal.
- **Metadata Integrity:** When modifying metadata (e.g., `vfs_truncate`), ensure the `i_rwsem` is held exclusively to prevent concurrent modification of `i_size` or other inode attributes.
- **Boundary Contracts:** Filesystems must not assume that `dentry->d_inode` is stable unless the dentry is hashed and the caller holds a reference or the `d_lock`. Always check `d_unhashed()` if the dentry was retrieved via RCU.
- **Initialization Symmetry:** Superblock initialization must be atomic with respect to `SB_BORN`. Use `super_lock()` to ensure that concurrent lookups do not access a partially initialized superblock.
- **User-Space Input:** All user-provided offsets and lengths in `vfs_fallocate` or `vfs_truncate` must be validated against `inode->i_sb->s_maxbytes` to prevent integer overflows or invalid filesystem state.
- **Atomic Contexts:** Ensure `f_pos_lock` is only used for files requiring `FMODE_ATOMIC_POS`. Do not assume `f_pos_lock` is available for all file types as it resides in a union.
- **Backing File Security:** When using `struct backing_file`, ensure `backing_file_set_security()` is called before the file is exposed to user-space to maintain security context integrity.
- **Superblock Waiters:** `super_lock()` must be paired with `super_unlock()` or `super_unlock_excl()` depending on the `excl` flag to ensure correct `s_umount` state management.
- **RCU-Safe Traversal:** When iterating over `d_alias` or `i_dentry`, ensure the `inode->i_lock` is held or the traversal is performed within an RCU read-side critical section to prevent use-after-free if the dentry is evicted.
- **Namespace Stability:** Always ensure `mnt_idmap` is valid for the lifetime of the operation; if the mount is unmounted during the operation, the VFS handles the teardown, but internal filesystem logic must not cache the idmap pointer across potential sleep points.
- **Transaction Aborts:** If a filesystem transaction (e.g., `ext4_journal_start`) is initiated, it must be paired with a corresponding `journal_stop` on all exit paths, including error returns from `vfs_*` helpers.
