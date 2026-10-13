# VFS Path Walk, Open, and Superblock Internals (`fs/namei.c`, `fs/open.c`, `fs/dcache.c`)

## Path Walking Modes: RCU-Walk vs REF-Walk (`fs/namei.c`)

- **RCU-walk (`LOOKUP_RCU`):** Traverses dentries and mounts locklessly under `rcu_read_lock()`, validating `mount_lock` and per-dentry `d_seq` seqlocks (`lookup_fast()`). Returns `-ECHILD` on any seqlock mismatch or when a sleeping filesystem operation is needed.
- **REF-walk:** Holds `d_lockref` dentry references and per-CPU `mnt_count` vfsmount references, acquiring shared `dir->i_rwsem` on dcache misses (`lookup_slow()`).
- **One-way transition (`try_to_unlazy()`):** Transitions an active RCU-walk to REF-walk by acquiring dentry/vfsmount references and validating `d_seq` before dropping `rcu_read_lock()`. If validation fails, `try_to_unlazy()` returns `false` and the caller returns `-ECHILD` so `filename_lookup()` restarts the walk in REF-walk mode (and retries with `LOOKUP_REVAL` on `-ESTALE`).

## File Open, `i_rwsem`, and Superblock `s_umount` Lifecycle

- **`do_dentry_open()` and `file->f_op` (`fs/open.c`):** `O_PATH` files receive `&empty_fops` (not `NULL`); regular opens acquire `fops_get(inode->i_fop)` (failing with `-ENODEV` if `NULL`). `file->private_data` is not freed automatically by the VFS and must be released in `f_op->release()`.
- **State re-validation across lock drops:** Whenever `i_rwsem` or another VFS lock is dropped and re-acquired, re-validate dentry positive/negative state, hashing (`!d_unhashed()`), parent identity, and inode state (`I_FREEING` / `S_DEAD`).
- **Superblock `sb->s_umount`:** Held exclusive (`down_write(&sb->s_umount)`) across `put_super`, `freeze_fs`, `unfreeze_fs`, and `remount_fs`, and held shared (`down_read(&sb->s_umount)`) across `sync_fs`.
