# MM Large Folios, THP, Hugetlb, and Memory Failure Context (Linux v7.3-rc5)

## Large Folio Mapping, `i_size`, Swapin, and Deferred Split

*   **`i_size` Boundary Enforcement (`mm/filemap.c`, `mm/memory.c`, `mm/truncate.c`)**:
    *   For non-shmem file-backed mappings (`!shmem_mapping(mapping)`), never install PTEs past `DIV_ROUND_UP(i_size_read(mapping->host), PAGE_SIZE)` and never install PMD mappings unless `file_end >= folio_next_index(folio)` (preserving POSIX `SIGBUS` past `i_size`).
    *   On truncate across a large folio, if splitting fails, `try_folio_split_or_unmap()` must fully unmap the folio so subsequent faults map it with `i_size`-clamped PTEs.
*   **mTHP Swapin Conflict Check (`non_swapcache_batch`)**: Before large-folio swapin (`can_swapin_thp()`, `shmem_swap_alloc_folio()`), verify `non_swapcache_batch(entry, nr_pages) == nr_pages` and fall back to order 0 if any slot in the range already has `SWAP_HAS_CACHE` set, avoiding unbounded `-EEXIST` retry loops in `swapcache_prepare()`.
*   **Mapcount Snapshot & Deferred Split**: Reading multiple large-folio mapcount fields (`_large_mapcount`, `_entire_mapcount`, `_nr_pages_mapped`) consistently requires `folio_lock_large_mapcount()` (as in `__wp_can_reuse_large_anon_folio()`). Partially unmapping a large folio must queue it via `deferred_split_folio()`.

## Hugetlb Locking, Accounting, and PMD Sharing (`mm/hugetlb.c`)

*   **Fault Lock Ordering**: `hugetlb_fault_mutex_table[hash]` -> `vma_lock` -> `i_mmap_rwsem` -> `folio_lock`.
*   **Fault Context & `need_wait_lock`**: Because `hugetlb_wp()` drops `hugetlb_fault_mutex` and `vma_lock` mid-fault, never call sleeping `filemap_lock_folio()` while holding `hugetlb_fault_mutex` on paths reaching `hugetlb_wp()`; use `folio_trylock()` and wait after dropping locks (`need_wait_lock`).
*   **Pool and Subpool Accounting**:
    *   Update `nr_huge_pages`, `free_huge_pages`, `surplus_huge_pages`, and `resv_huge_pages` atomically under a single `hugetlb_lock` hold.
    *   Pass `surplus_huge_pages_node[nid] > 0` (not hardcoded `false`) as `adjust_surplus` to `remove_hugetlb_folio()` / `add_hugetlb_folio()`.
    *   On error after `hugepage_subpool_get_pages()`, pass `chg - gbl_reserve` (not `chg`) to `hugepage_subpool_put_pages()` and pass its return value to `hugetlb_acct_memory()`.
    *   Propagate allocation-origin flags such as `HPG_cma` when demoting hugetlb folios (`demote_free_hugetlb_folios()`), and zero + mark uptodate under `hugetlb_fault_mutex_table[hash]` before `hugetlb_add_to_page_cache()`.
*   **PMD Table Unsharing (`huge_pmd_unshare`)**: Requires `i_mmap_rwsem` in write mode and must free unshared page-table pages via `tlb_remove_table()` (never direct `free_page()`) to synchronize with lockless GUP-fast.

## Memory Failure (`mm/memory-failure.c`)

*   **`memory_failure()` Return Codes**: `0` (recovered), `-EHWPOISON` (already poisoned), `-EOPNOTSUPP` (filtered), or other negative errno (recovery failed, kill process); internal helpers must return `0` on successful recovery, not `-EFAULT`.
*   **Unmapping and Accounting**: `unmap_poisoned_folio()` does not pass `TTU_SPLIT_HUGE_PMD` to `try_to_unmap()`, so callers must split large non-hugetlb folios (`try_to_split_thp_page()`) before calling it and keep `PG_hwpoison` (`PF_ANY`) and `PG_has_hwpoisoned` (`PF_SECOND`) synchronized across splits. Always update `num_poisoned_pages` and per-node `update_per_node_mf_stats()` together (`action_result()`).

## High-Frequency Bug Patterns & Guards

*   **Atomic PTE/PMD Updates**: Use architecture-specific helpers (e.g., `huge_ptep_get_and_clear`) for huge pages; standard PTE helpers misinterpret PFN encoding.
*   **Stale State**: Always re-read page table entries under the lock after any sleep or lock-drop sequence.
*   **Dirty Bit Propagation**: Ensure `folio_mark_dirty()` is called if the PMD was dirty before `zap_pmd_range`.
*   **Debug Macros**: Never place functional code (e.g., counter increments) inside `WARN_ON` or debug macros; these are stripped in production builds.
*   **Address Arithmetic**: Use `folio_page(folio, offset / PAGE_SIZE)` for safe page-level access. Never perform manual pointer arithmetic on `struct page` arrays without verifying alignment.
*   **Initialization Invariants**: Ensure all `struct page` flags are initialized before a folio is exposed to the page allocator or `hugetlb` lists.
*   **Error Path Symmetry**: Every `hugepage_subpool_get_pages()` must be matched by `hugepage_subpool_put_pages()` on all error paths, including early returns in `hugetlbfs` inode allocation.
*   **Locking Hierarchy**: Never acquire `hugetlb_lock` while holding `i_mmap_rwsem` in a way that violates the established lock order; always drop `i_mmap_rwsem` before acquiring `hugetlb_lock` if the path requires it.
*   **Memory Failure Recovery**: When handling `memory_failure()`, ensure that `num_poisoned_pages` is updated atomically relative to the page state transition to prevent counter drift.
*   **TLB Synchronization**: Always perform `tlb_remove_table()` for page table pages to ensure that lockless GUP-fast walkers do not access freed memory.
*   **Atomic Operations**: Use `atomic_inc_not_zero()` and `atomic_set_release()` for folio reference management to prevent race conditions during concurrent `mm_get_huge_zero_folio()` calls.
*   **State Machine Invariants**: Verify `hstate` parameters and `hugetlb_cmdline` parsing before global state validation to prevent zone contiguity check failures.
*   **PTE/Swap Distinction**: Ensure `pte_none()` and `pte_present()` checks are performed before accessing swap entries to prevent corruption of the swap payload.
*   **Subpool Lifecycle**: Use `hugepage_put_subpool()` to manage reference counts on subpools; never manually free subpool memory if references remain.
*   **Page Table Walkers**: Always identify and skip special non-present entries (e.g., device-private PMDs) during `madvise` or `zap` operations to avoid invalid memory access.
