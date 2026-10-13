# Writeback, Swap, Memcg, and Folio Migration API (`include/linux/writeback.h`, `include/linux/swap.h`, `include/linux/memcontrol.h`, `include/linux/migrate.h`)

## Writeback Tags and Helpers (`include/linux/fs.h`, `include/linux/writeback.h`)

- **Page cache XArray tags:** `PAGECACHE_TAG_DIRTY` (`XA_MARK_0`), `PAGECACHE_TAG_WRITEBACK` (`XA_MARK_1`), `PAGECACHE_TAG_TOWRITE` (`XA_MARK_2`).
- **`folio_start_writeback()` vs `__folio_start_writeback()`:** `folio_start_writeback(folio)` expands to `__folio_start_writeback(folio, false)`, which sets `PAGECACHE_TAG_WRITEBACK`, clears `PAGECACHE_TAG_DIRTY` (if the folio dirty flag is clear), and clears `PAGECACHE_TAG_TOWRITE`. To preserve `PAGECACHE_TAG_TOWRITE`, call `__folio_start_writeback(folio, true)`.
- **`wbc_to_tag(wbc)`:** Returns `PAGECACHE_TAG_TOWRITE` for `WB_SYNC_ALL` or `tagged_writepages` mode (after `tag_pages_for_writeback()` copies `DIRTY` to `TOWRITE` to prevent livelock from newly dirtied pages), and `PAGECACHE_TAG_DIRTY` otherwise.

## `folio_test_swapbacked()` vs `folio_test_swapcache()`

Shmem folios can reside in either the page cache or the swap cache (never both simultaneously):

| State | `folio_test_swapbacked()` | `folio_test_swapcache()` | `folio->mapping` | XArray storage |
| :--- | :--- | :--- | :--- | :--- |
| Shmem in page cache | `true` | `false` | shmem `address_space` | `mapping->i_pages` (1 multi-order entry) |
| Shmem in swap cache | `true` | `true` | `NULL` | swap address space (`N` individual entries) |
| Anon in swap cache | `true` | `true` | `anon_vma` (`FOLIO_MAPPING_ANON`) | swap address space (`N` individual entries) |

- Use `folio_test_swapcache(folio)`—never `folio_test_swapbacked(folio)`—to test whether a folio is currently in the swap cache or to branch between single-entry page-cache XArray operations and `N`-entry swap-cache XArray operations (see `__folio_migrate_mapping()` in `mm/migrate.c`).

## Swap Device Lifetime (`get_swap_device` / `put_swap_device`)

`swapoff()` kills `si->users` (`percpu_ref_kill()`) and waits for `synchronize_rcu()` before freeing `struct swap_info_struct`.
- `get_swap_device(entry)` validates the swap entry and acquires a reference on `si->users` (returning `NULL` if swapoff is in progress); pair every successful call with `put_swap_device(si)`.
- `__swap_entry_to_info(entry)` does NOT acquire a reference and is only safe under `rcu_read_lock()` or when the caller already holds a device reference.
- `__read_swap_cache_async()` calls `__swap_entry_to_info()` without pinning the device; callers must hold a device reference for the entry. In VMA swap readahead (`swap_vma_readahead()`), adjacent PTEs may reference different swap devices than the target fault entry and must each be pinned with `get_swap_device()`.

## Memcg Charge, Migration, and Lookup API

- **Charge lifecycle:** Pair `mem_cgroup_charge()` with `mem_cgroup_uncharge()` on the free path. Call `folio_unqueue_deferred_split()` before uncharging so deferred-split list removal does not access freed memcg state.
- **Migration charge transfer (`mem_cgroup_migrate()`):** Transfers charge from the old folio to the new folio and clears the source folio's `memcg_data`. Because the source folio no longer has a valid memcg, release it with `folio_put()`, NOT `folio_putback_lru()` (which asserts a valid memcg).
- **Offline memcgs and `folio_memcg()`:** `folio_memcg()` returns `NULL` for uncharged folios and may return an offline memcg until reparenting completes. Paths that charge/record/uncharge against an online ancestor resolved via `mem_cgroup_id_get_online()` must use that resolved memcg consistently rather than re-reading `folio_memcg()`.
- **`mem_cgroup_from_id()` RCU lifetime:** `mem_cgroup_from_id()` returns an RCU-protected pointer valid only under `rcu_read_lock()`; call `mem_cgroup_tryget()` before `rcu_read_unlock()` to persist the reference.

## Folio Migration Sleeping Constraints

`folio_mc_copy()` (`mm/util.c`) calls `cond_resched()` between pages, which sleeps when copying large folios. Consequently, `migrate_folio()`, `filemap_migrate_folio()`, and `__migrate_folio()` are sleeping functions for large folios.
- Filesystem `address_space_operations->migrate_folio` callbacks must never hold a spinlock across `filemap_migrate_folio()` / `migrate_folio()` / `__migrate_folio()` (use non-blocking state flags such as `BH_Migrate` in `__buffer_migrate_folio()` instead).
