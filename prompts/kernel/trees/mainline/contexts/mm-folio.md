# MM Folio Lifecycle and Page Cache Code Context (Linux v7.3-rc5)

## Speculative Lookup and PFN-Scanning Invariants

* **Lockless Page-Cache RCU Lookup (`filemap_get_entry`)**: Compound-dependent properties (`folio_mapcount()`, `folio_nr_pages()`, `folio_order()`) and head-page flags (`folio_test_lru()`) require stabilizing the folio first: `folio_try_get(folio)` followed by `xas_reload(&xas)` to verify the slot still points to `folio`.
* **Speculative PFN Scanning (`page_folio`)**:
  * Check termination bounds before calling `pfn_to_page(pfn)` (under `CONFIG_SPARSEMEM && !CONFIG_SPARSEMEM_VMEMMAP`), and use `pfn_to_online_page()` rather than `pfn_valid()` when accessing page metadata.
  * Filter out non-folio driver compound pages (`vm_insert_page()` with `order > 0` sets `PG_head` without initializing `folio->mapping` or LRU) via `PageLRU()`, `HWPoisonHandlable()`, or mapping checks before calling folio operations.
  * Stabilize speculative `folio = page_folio(page)` with `folio_try_get(folio)` and re-verify `page_folio(page) == folio` before reading folio flags or size (`const_folio_flags()`).
  * Advance PFN loops by `folio_nr_pages(folio) - folio_page_idx(folio, page) - 1` (or `folio_size(folio)`) for non-idempotent per-folio operations so large folios are not processed `folio_nr_pages()` times.
  * Never call `page_folio()` / `compound_head()` on a single-element stack copy of `struct page` (`page_fixed_fake_head()` reads `page[1].compound_head` under `CONFIG_HUGETLB_PAGE_OPTIMIZE_VMEMMAP`; see `snapshot_page()` in `mm/util.c`).
  * Use `for_each_page_ext()` / `page_ext_iter_next()` across large folios (under `CONFIG_SPARSEMEM`, `page_ext` is per-section, not contiguous).

## Reclaim, Eviction, Workingset, and Security Invariants

* **Lazyfree Folio Reclaim (`!folio_test_swapbacked()`, `mm/rmap.c`, `mm/huge_memory.c`)**:
  * In `try_to_unmap_one()` and `__discard_anon_folio_pmd_locked()`, enforce `smp_mb()` between PTE clear and refcount read and `smp_rmb()` between refcount and dirty read.
  * Check dirty status first: call `folio_set_swapbacked()` **only** when the folio is genuinely dirty (and not `VM_DROPPABLE`). If `ref_count != 1 + map_count` (e.g., speculative `folio_try_get()`), remap and abort *without* calling `folio_set_swapbacked()`.
* **Eviction Dirty/Writeback Check (`mapping_evict_folio`, `mm/truncate.c`)**: Must check `folio_test_dirty(folio) || folio_test_writeback(folio)` *after* acquiring `folio_lock(folio)` before removing from page cache.
* **Workingset Shadow Nodes (`mapping_set_update`)**: Any `XA_STATE` mutating `mapping->i_pages` outside `mm/filemap.c` (e.g., `mm/khugepaged.c`, `mm/shmem.c`) must call `mapping_set_update(&xas, mapping)` first so shadow `xa_node`s are tracked on the memcg `list_lru`.
* **Tail Page Overlay & `ZONE_DEVICE`**: Rearranging `struct folio` tail-page fields requires updating both `free_tail_page_prepare()` (`mm/page_alloc.c`) and `__dump_folio()` (`mm/debug.c`). `ZONE_DEVICE` page initialization bypasses `prep_new_page()` and must clear stale `compound_head`/`_nr_pages` before `prep_compound_page()`.
* **Page-Cache Side-Channel Gate**: Any syscall, ioctl, or procfs/sysfs interface exposing per-file page-cache residency/dirty/writeback state must require write permission on the file (`FMODE_WRITE`, `inode_owner_or_capable()`, or `file_permission(f, MAY_WRITE)`, matching `can_do_mincore()` / `can_do_cachestat()`).

## Synchronization and Concurrency Invariants

* **Lock Hierarchy**:
  1. `i_mmap_rwsem` (truncate) -> `private_lock` -> `swap_lock` -> `i_pages` lock.
  2. `i_rwsem` -> `invalidate_lock` -> `i_mmap_rwsem`.
  3. `mmap_lock` -> `invalidate_lock` -> `folio_lock`.
* **Atomic Context**: Memory allocation in `mm/compaction.c` or `mm/filemap.c` must avoid `GFP_KERNEL` if holding a spinlock or in interrupt context. Use `GFP_ATOMIC`.
* **Publication Barriers**: Use `smp_wmb()` or atomic release semantics when publishing shared objects (e.g., `anon_vma`) to ensure initialization visibility.
* **Lock Re-validation**: If a lock is dropped for blocking I/O, re-validate `folio->mapping` and `folio_test_uptodate()` upon re-acquisition.

## Error Handling and Boundary Validation

* **Pointer Safety**: Distinguish `NULL` (not found) from `ERR_PTR` (internal error). `filemap_get_entry` paths must check `IS_ERR()` before dereferencing.
* **Arithmetic**: Use `size_mul()` or `check_add_overflow()` for `nr_pages` or `pgoff_t` calculations to prevent integer overflow.
* **PTE Helpers**: Use architecture-specific accessors (e.g., `pte_offset_map_lock`) instead of direct pointer arithmetic. Standard PTE helpers are invalid for `hugetlb` entries.
* **Loop State**: Re-initialize local variables (`nr_pages`, `pfn`) per iteration to prevent stale value reuse.

## Architectural and Subsystem Contracts

* **Memory Allocation Contexts**: Allocation paths must strictly avoid sleeping in non-preemptible/interrupt contexts. Use `GFP_ATOMIC` for spinlock-protected regions.
* **Resource Unwinding**: Error paths must release resources in exact reverse order of acquisition. Reference counts must be decremented on all error exits to prevent leaks.
* **Asynchronous Teardown**: All background workers, timers, and delayed work must be synchronously cancelled and drained before the associated memory object is freed.
* **Hardware State**: Use architecture-specific accessors for page table walks. Cache maintenance must cover the entire range; verify alignment for DMA-mapped regions.
* **Input Validation**: All user-provided offsets and lengths must be clamped and checked for integer overflow before being used in memory indexing or allocation size calculations.
* **State Machine Invariants**: Ensure global resource counters are updated consistently across all paths, including fast-path and error-path exits. Reset state machine flags to baseline before retrying failed operations.
* **Reentrancy & Callbacks**: Cross-layer callbacks must be context-aware; if a callback is invoked from an atomic context, it must not trigger blocking I/O or sleep.
* **PTE Batching**: When modifying page tables, batch IPI broadcasts and TLB flushes to minimize overhead, ensuring consistency across all affected CPUs.
* **Huge Page Handling**: Always use `PageHuge()` checks before applying standard page-cache logic to avoid misinterpreting `hugetlb` metadata.
* **Memory Barriers**: Ensure `smp_rmb()` and `smp_wmb()` are used correctly when accessing shared flags to prevent stale reads in concurrent page-fault or reclaim paths.
