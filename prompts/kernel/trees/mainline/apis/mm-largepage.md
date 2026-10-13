# Large Folio, THP, and Hugetlb API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for managing large folios, Transparent Huge Pages (THP), and Hugetlb pages. It integrates empirical bug patterns to prevent concurrency, lifecycle, and state-machine violations.

## 1. Large Folio Splitting Contracts
*   **Minimum Folio Order (`min_order_for_split` / `mapping_min_folio_order`)**:
    *   File-backed mappings on large-block-size (LBS) filesystems can require `mapping_min_folio_order(folio->mapping) > 0`.
    *   `split_huge_page()` and `split_folio_to_list()` always target order 0 and fail with `-EINVAL` when `min_order > 0`.
    *   To split to the lowest supported order, call `try_folio_split_to_order(folio, min_order_for_split(folio))` (or `split_huge_page_to_list_to_order()`), and do not assume a successfully split folio is order 0 (`folio_test_large()` may still be true).
*   **Lock-Before-Refcount Precondition (`split_folio`)**:
    *   `__folio_split()` requires `folio_expected_ref_count(folio) == folio_ref_count(folio) - 1`.
    *   Never call `folio_get(folio)` and then block on `folio_lock(folio)` before `split_folio()` — concurrent tasks blocked on `folio_lock()` inflate the refcount and cause `split_folio()` to fail with `-EAGAIN` (livelocking retry loops). Use `folio_trylock(folio)` *before* `folio_get(folio)` (as in `madvise_free_huge_pmd()`).
*   **Page-Cache Removal Refcount (`folio_put_refs`)**: `__filemap_add_folio()` adds `folio_nr_pages(folio)` references; after `__filemap_remove_folio()`, drop all of them with `folio_put_refs(folio, folio_nr_pages(folio))`, never single-ref `folio_put()`.

## 2. Page-Flag Policies and Non-Atomic Flag Helpers (`include/linux/page-flags.h`)
*   **Subpage vs Head-Page Flags**:
    *   `PF_HEAD` (`PG_dirty`, `PG_referenced`, `_refcount`): stored on the head page for the whole folio.
    *   `PF_SECOND` (`PG_has_hwpoisoned`, `PG_large_rmappable`, `PG_partially_mapped`): stored on the first tail page (`folio_page(folio, 1)`).
    *   `PF_ANY` (`PG_hwpoison`, `PG_anon_exclusive` on PTE-mapped THP): stored per subpage.
*   **Non-Atomic `__folio_set_*` / `__SetPage*` Hazard**: Non-atomic flag helpers (`__set_bit` / `__clear_bit`) RMW the entire `unsigned long` flags word. Because multiple subsystems modify different flags in the same word under different locks (especially on the first tail page where `PF_SECOND` and `PF_ANY` flags share a word), only use non-atomic flag ops during isolated allocation or final teardown when the caller has exclusive ownership of the entire page.

## 3. Hugetlb and HWPoison Predicates
*   **`pmd_trans_huge()` vs `is_vm_hugetlb_page(vma)`**: `pmd_trans_huge()` can match both THP and hugetlb PMDs; check `is_vm_hugetlb_page(vma)` first before invoking THP-specific helpers like `split_huge_pmd()`.
*   **`folio_test_hugetlb()` and `folio_hstate()`**: `__update_and_free_hugetlb_folio()` clears the hugetlb page type under `hugetlb_lock`, making `folio_hstate()` return `NULL`. Either hold `hugetlb_lock` across `folio_test_hugetlb(folio)` + `folio_hstate(folio)` or use `size_to_hstate(folio_size(folio))` with an explicit `NULL` check.
*   **HWPoison Content Access Guard**: Check `PageHWPoison(page)` (per subpage) or `folio_contain_hwpoisoned_page(folio)` before reading page contents (`copy_page()`, `pages_identical()`, `kmap_local_page()`) in migration, KSM, compaction, or THP split paths to avoid triggering an unrecoverable hardware MCE.

## 4. Concurrency, Context, and Serialization
*   **Lock Hierarchy**: `folio_lock` is a sleeping bit-lock, while `hugetlb_lock` is a global spinlock for Hugetlb metadata (`folio_lock` -> `hugetlb_lock`). Always acquire `folio_lock` before `hugetlb_lock`, and never call sleeping `folio_lock()` while holding `hugetlb_lock`.
*   **Atomic Context**: Operations involving `split_huge_page` or `hugetlb` state modifications often sleep; these are forbidden in `in_atomic()` or hard-IRQ contexts.
*   **Memory Ordering**: When publishing a folio to a shared structure, use `smp_wmb()` or store-release semantics to ensure initialization is visible before the folio is reachable.
*   **Recursive Locking**: Avoid recursive spinlock acquisition by ensuring that failure-handling paths do not re-acquire locks already held by the primary operation.

## 5. High-Frequency Bug Patterns (Review Invariants)
*   **Incorrect Helpers**: Do not use standard PTE helpers on Hugetlb entries; use architecture-specific helpers (e.g., `huge_pte_offset`).
*   **Integer Overflows**: Use `unsigned long` for offset calculations. Use `folio_page(folio, n)` for tail-page access instead of manual pointer arithmetic.
*   **Union Field Overlap**: Verify that `struct page` union fields (e.g., `mapping`, `private`) do not conflict with flags or pointers in your specific path.
*   **Unit Mismatch**: Ensure all indexing logic uses the correct granularity (huge-page vs. base-page units).
*   **Type-Specific Checks**: Never call `folio_test_large()` on a tail page without verifying it is the head page, or use folio-aware helpers.
*   **Resource Cleanup**: Ensure all dynamically allocated test buffers or resv_map structures are explicitly freed on both success and failure paths.
*   **Initialization**: Ensure all memory structures are fully initialized before the subsystem performs global state validation or zone scanning to prevent observing uninitialized `struct page` data.
*   **Performance Heuristics**: Verify that performance-critical memory management heuristics remain effective across VMA boundaries.
*   **Batching**: Can multiple operations that require synchronization be batched to reduce the overhead of cross-processor communication?
*   **Data Integrity**: Does the synchronization logic verify if the target filesystem supports the required data integrity guarantees before waiting?
*   **Reference Counting**: Always balance `folio_get()` with `folio_put()` or `folio_put_refs()`. On error paths, ensure that any reference acquired during preparation is released before returning to the caller.
*   **Asynchronous Cleanup**: When an object is passed to an asynchronous worker, the caller must ensure the object's lifetime is extended (e.g., via `folio_get`) until the worker completes.
*   **NUMA Awareness**: Ensure that memory allocation policies (e.g., `alloc_pages_node`) correctly handle NUMA topologies, especially when dealing with memoryless nodes during early boot.
*   **State Idempotency**: Ensure that state transitions (e.g., `set_page_dirty`) are idempotent and do not leave the system in an inconsistent state if interrupted.

