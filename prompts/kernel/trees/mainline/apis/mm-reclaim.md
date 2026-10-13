# Writeback, Swap, Memcg, and Folio Migration API (Linux v7.3-rc5)

## 1. Writeback Tags and Helpers (`include/linux/writeback.h`)
- **Page cache XArray tags:** `PAGECACHE_TAG_DIRTY` (`XA_MARK_0`), `PAGECACHE_TAG_WRITEBACK` (`XA_MARK_1`), `PAGECACHE_TAG_TOWRITE` (`XA_MARK_2`).
- **`folio_start_writeback()` vs `__folio_start_writeback()`:** `folio_start_writeback(folio)` expands to `__folio_start_writeback(folio, false)`, which sets `PAGECACHE_TAG_WRITEBACK`, clears `PAGECACHE_TAG_DIRTY` (if the folio dirty flag is clear), and clears `PAGECACHE_TAG_TOWRITE`. To preserve `PAGECACHE_TAG_TOWRITE`, call `__folio_start_writeback(folio, true)`.
- **`wbc_to_tag(wbc)`:** Returns `PAGECACHE_TAG_TOWRITE` for `WB_SYNC_ALL` or `tagged_writepages` mode (after `tag_pages_for_writeback()` copies `DIRTY` to `TOWRITE` to prevent livelock from newly dirtied pages), and `PAGECACHE_TAG_DIRTY` otherwise.

## 2. Swap Cache, Device Lifetime, and Folio Mapping (`include/linux/swap.h`)
- **`folio_test_swapbacked()` vs `folio_test_swapcache()`:**
  - Shmem folios can reside in either the page cache or the swap cache (never both simultaneously).
  - Use `folio_test_swapcache(folio)`—never `folio_test_swapbacked(folio)`—to test whether a folio is currently in the swap cache or to branch between single-entry page-cache XArray operations and `N`-entry swap-cache XArray operations (see `__folio_migrate_mapping()` in `mm/migrate.c`).
- **`FOLIO_MAPPING_ANON`:** A bitmask used in `folio->mapping` to identify anonymous folios. When set, the `mapping` field points to an `anon_vma` structure rather than an `address_space`.
- **`address_space` vs `anon_vma`:** The `folio->mapping` union field points to an `address_space` for file-backed folios or an `anon_vma` for anonymous folios. Always check `folio_test_anon(folio)` before dereferencing this pointer.
- **Swap Device Lifetime:** `swapoff()` kills `si->users` (`percpu_ref_kill()`) and waits for `synchronize_rcu()` before freeing `struct swap_info_struct`.
  - `get_swap_device(entry)` validates the swap entry and acquires a reference on `si->users` (returning `NULL` if swapoff is in progress); pair every successful call with `put_swap_device(si)`.
  - `__swap_entry_to_info(entry)` does NOT acquire a reference and is only safe under `rcu_read_lock()` or when the caller already holds a device reference.
  - `__read_swap_cache_async()` calls `__swap_entry_to_info()` without pinning the device; callers must hold a device reference for the entry. In VMA swap readahead (`swap_vma_readahead()`), adjacent PTEs may reference different swap devices than the target fault entry and must each be pinned with `get_swap_device()`.

## 3. Memcg Charge, Migration, and Lookup API (`include/linux/memcontrol.h`)
- **Charge lifecycle:** Pair `mem_cgroup_charge()` with `mem_cgroup_uncharge()` on the free path. Call `folio_unqueue_deferred_split()` before uncharging so deferred-split list removal does not access freed memcg state.
- **Migration charge transfer (`mem_cgroup_migrate()`):** Transfers charge from the old folio to the new folio and clears the source folio's `memcg_data`. Because the source folio no longer has a valid memcg, release it with `folio_put()`, NOT `folio_putback_lru()` (which asserts a valid memcg).
- **Offline memcgs and `folio_memcg()`:** `folio_memcg()` returns `NULL` for uncharged folios and may return an offline memcg until reparenting completes. Paths that charge/record/uncharge against an online ancestor resolved via `mem_cgroup_id_get_online()` must use that resolved memcg consistently rather than re-reading `folio_memcg()`.
- **`mem_cgroup_from_id()` RCU lifetime:** `mem_cgroup_from_id()` returns an RCU-protected pointer valid only under `rcu_read_lock()`; call `mem_cgroup_tryget()` before `rcu_read_unlock()` to persist the reference.

## 4. Folio Migration Sleeping Constraints (`include/linux/migrate.h`)
- **Sleeping Contexts:** `folio_mc_copy()` (`mm/util.c`) calls `cond_resched()` between pages, which sleeps when copying large folios. Consequently, `migrate_folio()`, `filemap_migrate_folio()`, and `__migrate_folio()` are sleeping functions for large folios.
- **Locking Invariant:** Filesystem `address_space_operations->migrate_folio` callbacks must never hold a spinlock across `filemap_migrate_folio()` / `migrate_folio()` / `__migrate_folio()` (use non-blocking state flags such as `BH_Migrate` in `__buffer_migrate_folio()` instead).

## 5. Concurrency, Invariants, and Safety
- **Lock Hierarchy:** Acquire locks in the order: `lruvec->lru_lock` -> `mapping->i_pages` (XArray lock) -> `memcg->objcg_lock`.
- **Publication:** Never expose a folio to the LRU or page cache until all fields (including `memcg_data` and `mapping`) are fully initialized.
- **Partial Failure:** If `migrate_pages()` fails, it returns the number of pages that could not be migrated. Callers must ensure that any pages remaining on the list are handled via `putback_movable_pages()` to prevent memory leaks.
- **Atomic Context:** If a function is called from `kswapd` or an interrupt, ensure it does not call `mem_cgroup_charge()` directly, as this may trigger a blocking cgroup notification.
- **Memory Ordering:** Use `smp_wmb()` or `smp_store_release()` when publishing a folio to the page cache to ensure that initialization of the folio structure is visible to other CPUs before the XArray entry is visible.
- **False Sharing:** Frequently updated fields (like `lruvec_stats`) are separated from read-mostly fields in `struct mem_cgroup_per_node` to prevent false sharing. Avoid adding new fields to the middle of these structures without explicit padding.
- **Error Handling:** Always check for `IS_ERR()` on pointers returned from lookup functions. If a resource acquisition (like `get_swap_device`) fails, ensure the error path does not attempt to call the corresponding release function (`put_swap_device`).
- **Cleanup:** Always call the appropriate cleanup function when breaking out of a resource-acquiring iteration loop to prevent reference leaks.
- **Idempotency:** The page type API does not allow repeated non-atomic operations; ensure state transitions are strictly managed to prevent crashes when setting flags.
