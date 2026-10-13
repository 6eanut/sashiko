# MM Folio Lifecycle and Page Cache Code Context (`mm/filemap.c`, `mm/truncate.c`, `mm/swap.c`)

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
