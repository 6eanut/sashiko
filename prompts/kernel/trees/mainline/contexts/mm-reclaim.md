# MM Reclaim, Swap, and Migration Internals (`mm/vmscan.c`, `mm/swap_state.c`, `mm/migrate.c`, `mm/memcontrol.c`, `mm/page-writeback.c`, `mm/shmem.c`)

## Cgroup Writeback Domain Abstraction (`dtc_dom`)

`balance_dirty_pages()` (`mm/page-writeback.c`) selects between global (`gdtc`) and memcg (`mdtc`) `dirty_throttle_control` structures. Any helper or tracepoint receiving `struct dirty_throttle_control *dtc` must use `dtc_dom(dtc)` rather than referencing `global_wb_domain` directly, unless it specifically computes global limits (`global_dirty_limits()`).

## Swap Cache Residency and ABA Entry Reuse

- **Removal precondition (`folio_free_swap()` in `mm/swapfile.c`):** Removes a folio from the swap cache only when `folio_swapcache_freeable(folio)` (in swapcache, not under writeback, swap device not suspended) and `!folio_swapped(folio)`.
- **Large folio swapin conflicts (`shmem_swapin_folio()` in `mm/shmem.c`):** Multi-size THP swapin fails with `-EEXIST` when any subpage's swap slot is already populated in the swap cache by a racing smaller-order swapin; callers must fall back to order-0 or retry.
- **Swap entry ABA reuse (`move_swap_pte()` in `mm/userfaultfd.c`):** Recycled `swp_entry_t` values can be reassigned to a new folio while a caller waits for a lock (`pte_same()` only checks the PTE value, not folio identity). After locking a folio returned by `swap_cache_get_folio()`, re-verify `folio_test_swapcache(folio)` and `folio->swap.val`. If an earlier swap-cache lookup returned `NULL`, check `SWAP_HAS_CACHE` in `si->swap_map[swp_offset(entry)]` after acquiring the PTL.

## Dual Reclaim Paths and MGLRU State (`mm/vmscan.c`)

- **Classic LRU vs MGLRU parity:** `mm/vmscan.c` maintains two runtime-selectable reclaim pipelines that both call `shrink_folio_list()` but perform separate post-reclaim accounting:
  - Inactive/eviction: `shrink_inactive_list()` <-> `evict_folios()`
  - Active/aging: `shrink_active_list()` <-> `scan_folios()`
  Changes to vmstat counters, memcg events, or tracepoints in one path must be mirrored in its counterpart.
- **MGLRU generation and tier bits (`LRU_GEN_MASK` / `LRU_REFS_FLAGS`):** When moving a folio to a new MGLRU generation (`folio_update_gen()`, `folio_inc_gen()`), clear `LRU_REFS_FLAGS` (`LRU_REFS_MASK | BIT(PG_referenced)`) alongside `LRU_GEN_MASK` via `old_flags & ~(LRU_GEN_MASK | LRU_REFS_FLAGS)` so stale access tiers do not carry over. (`lru_gen_add_folio()` sets initial generation and clears `LRU_GEN_MASK | BIT(PG_active)`.)

## Per-CPU Memcg Stock Drain (`mm/memcontrol.c`)

Per-CPU memcg charge stocks cache a `struct mem_cgroup *` pointer and hold a reference pinning the memcg until the stock is drained. Draining per-CPU stocks on cgroup destruction flushes batched charges and drops that reference; omitting a drain delays freeing of the `mem_cgroup` structure until a later drain rather than causing a use-after-free.

## Reclaim, Migration, and Swap Invariants

- **Unpinnable folio isolation (`collect_longterm_unpinnable_folios()` in `mm/gup.c`):** Returns the total count of unpinnable folios encountered, including folios skipped before list insertion (such as device-coherent folios or `folio_isolate_lru()` failures). Callers must check the returned count, not `list_empty()`, to determine whether unpinnable folios exist.
- **Bounded spinlock scans:** Loops filtering LRU or reclaim lists under a spinlock must advance the scan termination counter on skip paths (or bound skips via `SWAP_CLUSTER_MAX_SKIPPED`) to avoid unbounded lock hold times.
- **Migration rmap lock scope (`TTU_RMAP_LOCKED` / `RMP_LOCKED`):** When `TTU_RMAP_LOCKED` is passed to `try_to_migrate()`, `i_mmap_rwsem` must remain held continuously until `remove_migration_ptes()` with `RMP_LOCKED` completes; dropping `i_mmap_rwsem` between unmap and remap creates an ABBA deadlock with `folio_lock` (`unmap_and_move_huge_page()`).
- **kswapd dynamic order drop (`kswapd_shrink_node()`):** Drops `sc->order` to `0` once `compact_gap(order)` pages are reclaimed. Watermark checks in `pgdat_balanced()` / `balance_pgdat()` must test `sc->order != 0` dynamically rather than relying on a saved initial order.
- **Zone skip consistency:** Zone-skip predicates across `balance_pgdat()`, `pgdat_balanced()`, `allow_direct_reclaim()`, and `skip_throttle_noprogress()` must stay consistent so `kswapd_failures` can trigger and prevent infinite loops in `throttle_direct_reclaim()`.
- **Swap allocator `local_lock` (`folio_alloc_swap()`):** Runs under `local_lock()` with preemption disabled; no sleeping operations may be invoked without dropping the lock.
- **Counter-gated tracking lists and lock-drop iteration (`mm/shmem.c`):**
  - `shmem_swaplist` membership is gated by `info->swapped > 0`; error paths must check `info->swapped` before calling `list_del_init(&info->swaplist)`.
  - `list_for_each_entry_safe()` is not safe across a lock drop when concurrent paths can call `list_del_init()` (making the current entry self-referential); after re-acquiring the lock, restart from the list head (`shmem_unuse()`).

## Architectural Accessors and Safety

- **Huge Page Table Access:** Use architecture-specific accessors (e.g., `huge_pte_offset`) instead of standard `pte_offset_map` to avoid PFN decoding errors on huge pages.
- **Swap Usage Bit:** When updating `inuse_pages` (which contains `SWAP_USAGE_OFFLIST_BIT`), use atomic operations that preserve the bit status; do not overwrite the entire atomic value.
- **Folio Isolation:** `folio_isolate_lru()` must be called to remove a folio from the LRU list before migration or reclaim; failure to isolate requires immediate putback to avoid list corruption.
- **Folio Locking:** `folio_lock()` must be held during migration and swap-out to ensure atomicity of PTE updates and state transitions; never drop the folio lock while holding `i_mmap_rwsem` to prevent ABBA deadlocks.
- **Migration Path:** `unmap_and_move_huge_page()` requires strict adherence to the migration sequence: isolate, unmap, move, and remap; any failure in the middle requires a full rollback of the migration state to prevent memory leaks.
- **Atomic State Transitions:** All updates to shared memory flags and page states must be performed using atomic operations when accessed concurrently or from interrupt handlers.
- **Preemption and Latency:** Long-running reclaim loops must include periodic preemption points (e.g., `cond_resched()`) to prevent system-wide latency issues and soft lockups.
- **Error Propagation:** Internal error codes from memory operations must be propagated to the caller; do not collapse them into generic failure codes, as this masks the root cause of reclaim failures.
- **Integer Overflow Prevention:** All arithmetic operations involving user-provided offsets, lengths, and page frame numbers must be checked for potential integer overflows against the system memory layout.
- **Initialization Guarantees:** All fields in a newly allocated memory structure must be explicitly set to a known safe state before the structure is evaluated by conditional logic to prevent uninitialized memory exposure.
- **Reclaim Fallback Logic:** When `sc->may_swap` is false, reclaim paths must strictly avoid anonymous memory scanning; ensure `sc->proactive_swappiness` is respected to prevent unintended OOM triggers.
- **Signal Responsiveness:** Long-running kernel threads (e.g., `kswapd`) must check `fatal_signal_pending()` or `kthread_should_stop()` to ensure responsiveness to system-wide suspend/resume events.
- **Memory Accounting:** When reparenting memcg objects, ensure that accounting counters are moved atomically to prevent double-counting or stale values in the child cgroup.
- **Diagnostic Severity:** Use `WARN_ON_ONCE` for non-fatal consistency checks to avoid unnecessary system panics on production kernels while maintaining visibility into subsystem state machine violations.
