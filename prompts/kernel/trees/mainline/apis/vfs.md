# VFS Inode, Dentry, Permission, and `file_operations` API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for the VFS subsystem, focusing on object lifecycle, locking, and error recovery.

---

## 1. Inode Locking Hierarchy and `i_rwsem` Subclasses (`include/linux/fs.h`)

| Subclass | Constant | Usage |
| :--- | :--- | :--- |
| `0` | `I_MUTEX_NORMAL` | Target object of the current VFS operation (`inode_lock(inode)`) |
| `1` | `I_MUTEX_PARENT` | Parent directory (`inode_lock_nested(dir, I_MUTEX_PARENT)`) |
| `2` | `I_MUTEX_CHILD` | Child/target subdirectory changing parent during rename |
| `3` | `I_MUTEX_XATTR` | Extended attribute (`xattr`) operations |
| `4` | `I_MUTEX_NONDIR2` | Second non-directory inode (`lock_two_nondirectories()`) |
| `5` | `I_MUTEX_PARENT2` | Descendant parent directory in cross-directory `lock_rename()` |

*   **Parent before child:** When locking a directory to look up, create (`vfs_create`, `vfs_mkdir`, `vfs_mknod`, `vfs_symlink`, `vfs_link`), unlink (`vfs_unlink`, `vfs_rmdir`), or rename entries inside it, use `inode_lock_nested(dir, I_MUTEX_PARENT)` rather than plain `inode_lock(dir)` (`I_MUTEX_NORMAL`).
*   **Sleeping Locks:** `inode_lock()` and `lock_rename()` are sleeping locks; they cannot be called from atomic context (interrupts, `spin_lock` held).

---

## 2. Dentry States, Instantiation, and TOCTOU Revalidation (`include/linux/dcache.h`)

*   **Negative vs positive dentries:** Use `d_is_negative(dentry)` / `d_is_positive(dentry)` (which inspect `DCACHE_MISS_TYPE` flags); `d_really_is_negative()` / `d_really_is_positive()` inspect raw `dentry->d_inode` and are only for a filesystem inspecting its own underlying dentries.
*   **`d_instantiate()`, `d_add()`, and `d_instantiate_new()`:**
    *   `d_instantiate(dentry, inode)` requires the dentry to be unhashed on `d_alias` (`BUG_ON(!hlist_unhashed(&entry->d_u.d_alias))`), not `d_in_lookup()`, and requires the caller to hold a reference on `inode`. It does **not** hash the dentry; use `d_add(dentry, inode)` to both instantiate and hash.
    *   `d_instantiate_new(dentry, inode)` combines `d_instantiate()` with `unlock_new_inode()` and requires `inode` to be non-NULL with `I_NEW` set (from `iget_locked()`, `iget5_locked()`, or `insert_inode_locked()`—never from `new_inode()` alone).
*   **TOCTOU dentry validation:** When a dentry was obtained before acquiring its parent's `i_rwsem`, revalidation under the lock must check **both** parent stability and dcache hashing:
    ```c
    if (dentry->d_parent != expected_parent || d_unhashed(dentry))
        return -ESTALE;
    ```
    Checking only `dentry->d_parent == expected_parent` misses concurrent unlink/rmdir races that unhash the dentry while leaving `d_parent` pointing to the old parent.

---

## 3. Permission Checks and `file_operations` Replacement

*   **`inode_permission(idmap, inode, mask)` and `file_permission(file, mask)`:** Central VFS permission gates (checking read-only superblock, `S_IMMUTABLE`, idmap, POSIX ACLs/mode, device cgroup, and LSM hooks).
*   **`replace_fops(file, new_fops)` (`include/linux/fs.h`):** Never overwrite `file->f_op` by raw pointer assignment after open. Acquire a module reference on the new operations table via `fops_get(new_fops)` and swap via `replace_fops(file, fops)` (which calls `fops_put()` on the previous `file->f_op`).

---

## 4. High-Frequency Bug Patterns & Review Invariants

*   **Reference Counting:** If a function returns an error, ensure all `dget`/`ihold` calls are matched by `dput`/`iput`. When passing an object to an asynchronous worker, the caller **must** increment the reference count before scheduling.
*   **Sysfs/Procfs Stores:** Store functions must return the number of bytes processed. Returning `0` signals EOF/error to userspace and may trigger infinite retry loops.
*   **Teardown Invariants:** Ensure all background work (timers, workqueues) is cancelled via `cancel_work_sync()` before freeing the associated object.
*   **Namespace Awareness:** When displaying identifiers (e.g., in `procfs`), always translate them to the reader's namespace using `from_kuid_munged()` or similar helpers.
*   **Buffer Sizing:** Use `seq_file` interfaces for `debugfs` or `procfs` output to handle partial reads and buffer sizing automatically. Avoid manual `copy_to_user` with fixed-size buffers.
*   **Integer/Type Integrity:** Validate all user-provided lengths against `PAGE_SIZE` or specific subsystem limits before using them in arithmetic to prevent overflows.
