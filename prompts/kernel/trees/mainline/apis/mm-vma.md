# VMA Classification, Flags, Locking, and Mapping Helpers (`include/linux/mm.h`, `include/linux/mmap_lock.h`)

## VMA Anonymous vs File-Backed Classification (`vma_is_anonymous`)

- `vma_is_anonymous(vma)` returns `!vma->vm_ops` and is the canonical test for whether a VMA is anonymous.
- `vma_set_anonymous(vma)` and `vma_desc_set_anonymous(desc)` set `vm_ops = NULL` without clearing `vm_file`. Consequently, a VMA can have `vma->vm_file != NULL` while still being anonymous (for example, private mappings of `/dev/zero` via `mmap_zero_prepare()` in `drivers/char/mem.c`).
- Use `vma_is_anonymous(vma)` / `!vma_is_anonymous(vma)`—not `vma->vm_file == NULL` / `vma->vm_file != NULL`—to classify anonymous vs file-backed VMAs in page-fault, rmap, and memory-management dispatch.
- **Valid `vma->vm_file` checks:** Direct checks on `vma->vm_file` are valid when an operation specifically requires the absence of a backing `struct file` (such as `MADV_WIPEONFORK` requiring `MAP_ANONYMOUS`, or `__MADV_SET_ANON_VMA_NAME` checking `vma->vm_file && !vma_is_anon_shmem(vma)` because `/proc/$pid/maps` prints the file path whenever `vm_file` is set).

## VMA Flags Modification API (`vm_flags_*`)

- `vm_flags_set(vma, flags)`: ORs bits into `vma->vm_flags` (never clears existing bits). Using `vm_flags_set()` to replace flags leaves stale bits (such as `VM_WRITE` / `VM_MAYWRITE`) set.
- `vm_flags_reset(vma, flags)`: Replaces `vma->vm_flags` with the exact value `flags` (asserting locks).
- `vm_flags_init(vma, flags)`: Replaces `vma->vm_flags` without lock assertions (for newly allocated VMAs not yet inserted into the tree).
- `vm_flags_clear(vma, flags)` / `vm_flags_mod(vma, set, clear)`: Clear or simultaneously set and clear specific bits. Never clear `VM_ACCOUNT` on a surviving VMA (`do_vmi_munmap()` only uncharges committed memory for VMAs with `VM_ACCOUNT`).

## `mmap_lock` and Per-VMA Locking Helpers

- **`mmap_write_lock` vs `mmap_read_lock`:** Use `mmap_write_lock()` for VMA structural changes (insert/delete/split/merge, changing `vm_flags` or `vm_page_prot`) and `mmap_read_lock()` for VMA lookups, page faults, and read-only walks.
- **Killable lock re-acquisition:** Always check the return value of `mmap_write_lock_killable()` and `mmap_read_lock_killable()` (`-EINTR` on fatal signal), especially in retry loops and lock-upgrade paths (`__get_user_pages_locked()`).
- **`vma_start_write(vma)`:** `mmap_write_lock()` prevents new per-VMA read locks (`vma_start_read()`) via seqcount, but does NOT revoke per-VMA read locks acquired before `mmap_write_lock()`. Call `vma_start_write(vma)` before modifying a VMA or accessing its page tables under `mmap_write_lock()`.
- **`vma_assert_locked(vma)` vs `mmap_assert_locked(mm)`:** `mmap_assert_locked(mm)` fails when only a per-VMA lock is held. Code paths reachable under either `mmap_lock` or a per-VMA lock (such as page-table walk and zap paths) must use `vma_assert_locked(vma)`.

## VMA Interval Tree, `vmalloc` Remap, Memfd, and Overcommit Helpers

- **`vma_interval_tree_foreach()` and `vma_address()`:** `mapping->i_mmap` is indexed by `vm_pgoff` (`pgoff_t` in page-cache units), NOT physical PFN. Never pass a raw PFN to `vma_interval_tree_foreach()` or `vma_address()`.
- **`find_vm_area()` and `remap_vmalloc_range_partial()`:** `find_vm_area(addr)` performs a range-containment lookup (`addr >= va->va_start && addr < va->va_end`), not an exact base-address check. Passing a page-aligned interior pointer `kaddr` within a `VM_USERMAP` `vmalloc` allocation to `remap_vmalloc_range_partial(vma, uaddr, kaddr, pgoff, size)` is valid.
- **Memfd file creation (`memfd_alloc_file()`):** Use `memfd_alloc_file()` to create memfd files rather than calling `shmem_file_setup()` or `hugetlb_file_setup()` directly, which omits `O_LARGEFILE`, `fmode` flags, and LSM security initialization.
- **Committed memory accounting (`security_vm_enough_memory_mm()`):** On success (`0`), `security_vm_enough_memory_mm()` increments `vm_committed_as` via `vm_acct_memory()`. Every subsequent error path must call `vm_unacct_memory(pages)` to avoid leaking committed memory under `vm.overcommit_memory=2`.
- **`(void *)bool` in page-walk callbacks:** Casting a C `bool` (`_Bool`) to `void *` (e.g., `walk_page_range_vma(..., (void *)lock)`) does not trigger `-Wint-to-pointer-cast` in GCC or Clang.
