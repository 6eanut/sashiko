# Folio, Page Cache, and XArray API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for the Linux memory management folio API. Adherence to these invariants is mandatory for all code submitted to the kernel.

## 1. Folio Reference, Locking, and Mapping Helpers (`include/linux/mm.h`, `include/linux/pagemap.h`)

*   **Folio Order vs Page Count**: Pass `1UL << order` (or `PAGE_SIZE << order`, `folio_nr_pages(folio)`, `folio_size(folio)`) to `round_up()`, `round_down()`, and `ALIGN()`, never the raw `order` exponent.
*   **Expected Reference Count (`folio_expected_ref_count`)**:
    *   Invariant: `folio_ref_count(folio) >= folio_mapcount(folio)` (warn on `mapcount > refcount`, never `mapcount < refcount`).
    *   Compare `folio_ref_count(folio)` against `folio_expected_ref_count(folio)` (after draining per-CPU LRU batches via `lru_add_drain_all()` when guarded by `folio_may_be_lru_cached(folio)`) rather than using `folio_test_lru()` as a proxy for extra references.
    *   Never read `folio_order()`, `folio_nr_pages()`, or flags after a call that may drop the last reference or after `folio_put_testzero()` / `put_page_testzero()` returns `false`.
*   **Locking (`folio_lock` vs `folio_trylock`, `folio_end_read`)**:
    *   After GUP pins a folio, use `folio_lock()` or `folio_lock_killable()`, not `folio_trylock()` (which is only for scan loops that skip locked folios).
    *   Because `folio_lock()` can sleep, re-validate folio state (`folio->mapping != NULL` for truncation, `folio->private` if releasable by `release_folio()`) after acquiring the lock. Note that `folio->mapping` is also `NULL` for swap-cached shmem folios (`!folio_test_anon()` does not guarantee `folio->mapping != NULL`).
    *   `folio_end_read()` uses XOR to set `PG_uptodate`; never call `folio_end_read(folio, true)` on an already-uptodate folio (it clears `PG_uptodate`) — use `folio_unlock()` instead.
*   **Subpage Access and Highmem Mapping (`folio_page`, `kmap_local_folio`)**:
    *   In PTE batch loops over a large folio, use `vm_normal_page()` for the subpage mapped by a PTE rather than `folio_page(folio, 0)`, and bounds-check `n < folio_nr_pages(folio)` before `folio_page(folio, n)`.
    *   `kmap_local_page()` and `kmap_local_folio()` map only a single `PAGE_SIZE` page; on `CONFIG_HIGHMEM`, accessing beyond `PAGE_SIZE` faults.

## 2. XArray Multi-Order Page Cache Iteration (`include/linux/xarray.h`)

*   **`xas_next()` Sibling Slots**: `xas_next()` visits every slot of a multi-order entry (including siblings), whereas `xas_find()` / `xas_find_marked()` skip siblings. When iterating with `xas_next()`, call `xas_advance(&xas, folio_next_index(folio) - 1)` after processing a folio.
*   **`find_get_entries()` vs `find_lock_entries()`**: `xas_descend()` does not update `xas.xa_index` to the canonical base of a multi-order entry (`base = xas.xa_index & ~((1UL << xas_get_order(&xas)) - 1)`), and `find_get_entries()` does not filter entries whose base is below `*start`.
*   **Multi-Order Atomicity**: Never read `xa_get_order()` under `rcu_read_lock()` and act on that order later under `xa_lock`; combine `xas_load()`, `xas_get_order()`, and `xas_store()` inside a single `xas_lock_irq()` critical section.

## 3. Lifecycle, Teardown, and Concurrency

*   **Ownership**: Lookups (e.g., `filemap_get_folio()`) return a reference that must be dropped via `folio_put()`.
*   **Teardown**: Never use `kfree()` on a folio; use the appropriate `folio_` teardown API. If initialization fails after `folio_alloc()`, perform a symmetric `folio_put()`.
*   **Visibility**: A folio must be fully initialized (e.g., `folio_mark_dirty()`) before it is made visible to other threads via the page cache or XArray.
*   **Asynchronous Handoff**: Increment the reference count (`folio_get()`) before scheduling a folio for asynchronous work.
*   **Memory Ordering**: When publishing to shared structures, ensure proper memory ordering. While XArray handles internal barriers, custom list-based tracking requires explicit `smp_wmb()` or `rcu_assign_pointer()`.

## 4. High-Frequency Bug Patterns & Review Invariants

*   **Unbalanced Refcounts**: Trace every `return` statement in functions calling `folio_alloc` or `filemap_get_folio` to ensure `folio_put()` is called on error paths.
*   **IS_ERR vs NULL**: `filemap_get_folio()` returns `NULL` on failure; do not use `IS_ERR()`.
*   **Dirty Bit Propagation**: When zapping page tables, ensure the dirty bit is propagated to the folio (`folio_mark_dirty()`) before the PTE is cleared.
*   **Integer Safety**: Use `unsigned long` for all index calculations to prevent overflow on 32-bit architectures.
*   **Atomic Context**: `folio_lock()` must not be called in atomic context. Use `kmap_local_folio()` for temporary mappings in any context; mappings must be unmapped in reverse order via `kunmap_local()`.
*   **Error Handling**: Always verify that resources acquired in a multi-step process are released in the exact reverse order of acquisition during failure unwinds.
*   **State Consistency**: Ensure that counters (like `mapcount`) are only incremented after successful validation of the object's state to prevent inconsistent tracking during partial failures.
*   **Registration**: Objects must be fully initialized before insertion into global registries (XArray/Page Cache) to prevent concurrent threads from observing uninitialized state.
*   **Cleanup**: All dynamically allocated internal structures (e.g., XArray nodes) must be explicitly destroyed during teardown to prevent memory leaks.
*   **Lock Hierarchy**: When acquiring both a sleeping `folio_lock` and the mapping's `xa_lock` spinlock, always acquire the sleeping `folio_lock` (page) first and `xa_lock` (mapping) second (`folio_lock` -> `i_mmap_rwsem` -> `xa_lock`). Never call sleeping `folio_lock()` while holding `xa_lock` or any spinlock.
*   **Interrupt Safety**: If a folio operation is performed in an interrupt context, ensure that all locks are IRQ-safe (`spin_lock_irqsave`) and that no sleeping functions (like `folio_lock`) are invoked.
*   **Boundary Validation**: Always validate `offset + length` against `folio_size(folio)` before performing subpage operations to prevent out-of-bounds access.
