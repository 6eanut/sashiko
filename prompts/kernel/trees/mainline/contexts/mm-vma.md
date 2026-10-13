# MM VMA Internals (`mm/vma.c`, `mm/mmap.c`, `mm/mmap_lock.c`)

## `SLAB_TYPESAFE_BY_RCU` and VMA Recycling (`mm/mmap_lock.c`)

The VMA slab cache is created with `SLAB_TYPESAFE_BY_RCU` (`vma_state_init()` in `mm/vma_init.c`), so a freed VMA can be reallocated to a different `mm_struct` during an RCU read-side critical section:
- In `lock_vma_under_rcu()`, `lock_next_vma()`, and `vma_start_read()`, after `vma_start_read()` increments `vma->vm_refcnt`, if `vma->vm_mm != mm` (the VMA was recycled to a foreign `mm`), the caller must stabilize the foreign `vma->vm_mm` with `mmgrab()` before calling `vma_refcount_put()` (which dereferences `vma->vm_mm` for `rcuwait_wake_up()`) and release it with `mmdrop()` afterward.
- **Maple state RCU lifetime:** `struct ma_state` caches RCU-protected node pointers. Whenever `rcu_read_unlock()` is called (including internally on `vma_start_read()` failure), invalidate the state with `mas_set()` or `mas_reset()` before reusing it.
- **VMA lock refcount balance (`__vma_enter_locked()`):** `__vma_enter_locked()` adds `VMA_LOCK_OFFSET` to `vma->vm_refcnt` before waiting for readers; when waiting with `TASK_KILLABLE` / `TASK_INTERRUPTIBLE`, the `-EINTR` error path must subtract `VMA_LOCK_OFFSET` back.

## VMA Split, Merge, and Modify Invariants (`mm/vma.c`)

- **Split/merge critical section:** `__split_vma()`, `commit_merge()`, and `vma_shrink()` execute page-table structural changes (`vma_adjust_trans_huge()`, `hugetlb_split()`) strictly inside the critical section bracketed by `vma_start_write(vma)` + `vma_prepare()` and `vma_complete()` (which holds file `i_mmap_lock_write` and `anon_vma` write locks).
- **`vm_ops->may_split()`:** Runs before VMA and rmap locks are acquired; it must only validate whether splitting is permitted and must never modify page tables or shared state.
- **Per-VMA lock exclusion before PMD/PTE checks:** Under `mmap_write_lock()`, calling `check_pmd_still_valid()` or `find_pmd_or_thp_or_none()` before `vma_start_write(vma)` is a race because a concurrent per-VMA-locked `MADV_DONTNEED` can invoke `try_to_free_pte()` -> `pmd_clear()` + `free_pte()` between the check and subsequent use.
- **VMA merge/modify return values and pointer invalidation:**
  - `vma_merge_new_range()`, `vma_merge_existing_range()`, and `vma_modify*()` may free the original VMA on success; callers must use the returned VMA pointer.
  - `vma_modify*()` (e.g., `vma_modify_flags()`) returns `ERR_PTR(-ENOMEM)` on split/merge allocation failure; callers in VMA loops must check `IS_ERR()` (or set `give_up_on_oom`) before dereferencing.
  - `vma_merge_extend()`, `vma_merge_new_range()`, and `copy_vma()` return `NULL` on failure; do not overwrite a struct member (such as `vrm->vma`) before checking for `NULL`, and note that `vmg->start`/`end`/`pgoff` may be mutated on failure (`vmg_nomem()`).
- **Merge `anon_vma` asymmetry:** Merging an unfaulted VMA with a faulted VMA requires `dup_anon_vma()`. Property checks on `anon_vma` during merge (`is_mergeable_anon_vma()`, `vma_is_fork_child()`) must inspect whichever VMA actually has an `anon_vma`, not unconditionally the destination VMA.
- **Pre-merge flags and `skip_vma_uprobe`:** Flags not in `VM_IGNORE_MERGE` must be included in the proposed `vm_flags` passed to `vma_merge_new_range()` rather than added post-merge via `vm_flags_set()`. Callers that overwrite or move page tables immediately after a merge must set `skip_vma_uprobe` in `struct vma_merge_struct` so `vma_complete()` does not install uprobe PTEs that get orphaned.
- **Fork flag divergence (`dup_mmap()`):** `dup_mmap()` clears `__VM_UFFD_FLAGS` and `VM_LOCKED_MASK` on the child VMA; fork-time checks such as `vma_needs_copy()` must inspect the destination VMA when testing flags that differ between parent and child.
- **Zero-address VMAs:** `vma->vm_start` can legitimately be `0`; do not use an address variable as a boolean "is set" flag.

## File Reference Ownership and `.mmap_prepare` Lifecycle (`mm/vma.c`)

- **Swapped `vm_file` cleanup via `put_map(&map)`:** When `f_op->mmap_prepare` or `f_op->mmap` replaces `vm_file`, the replacement file already carries its own reference (`map->file_doesnt_need_get`). In `__mmap_new_file_vma()`, `if (map_same_file(map)) fput(map->vm_file);` intentionally skips `fput` when `vm_file` was swapped because `__mmap_region()`'s `abort_munmap:` path releases `map->vm_file` via `put_map(&map)` after `unmap_region(&map)`.
- **No VMA during `call_mmap_prepare()`:** `.mmap_prepare` (`vfs_mmap_prepare()`) operates on `struct vm_area_desc` before any `struct vm_area_struct` is allocated; `vm_ops->close()` cannot and must not be called if `call_mmap_prepare()` or `mmap_prepare_validate()` fails.
- **`__compat_vma_mmap()` non-NULL `vm_file`:** `__compat_vma_mmap()` is only called from a file's `f_op->mmap` path where `vma->vm_file` is non-NULL, and `.mmap_prepare` cannot clear `desc->vm_file` to `NULL`.
- **Restoring VMA bounds before `unmap_region()`:** In `mmap_file()`, restoring `vma->vm_start`, `vma->vm_end`, and `vma->vm_pgoff` on validation failure ensures `unmap_region(&map)` only unmaps the originally reserved range.
- **Driver contract `WARN_ON_ONCE` / `VM_WARN_ON_ONCE`:** Warnings in `mmap_validate_vma_flags()`, `mmap_action`, or `discontig_kernel_map_page*()` enforce internal kernel driver contracts and are not user-triggerable bugs unless a driver violates the contract.

## External `mm_struct` Iteration and Static `mm_struct` Sizing

- **`check_stable_address_space(mm)`:** Needed when walking an external `mm`'s maple tree from the global `mmlist` (e.g., `unuse_mm()` in `mm/swapfile.c`) where a failed `dup_mmap()` may leave `XA_ZERO_ENTRY` markers and `MMF_UNSTABLE`. Not needed when a subsystem walks its own list of established mappings (such as `kernel/bpf/arena.c:zap_pages()`).
- **`MM_STRUCT_FLEXIBLE_ARRAY_INIT`:** `struct mm_struct` ends with a flexible array packing cpumask and `mm_cid` regions; static definitions (`init_mm`, `efi_mm`) must use `MM_STRUCT_FLEXIBLE_ARRAY_INIT`.
