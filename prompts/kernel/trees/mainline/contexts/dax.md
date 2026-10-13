# DAX (Direct Access) Subsystem Context (Linux v7.3-rc5)

## 1. XArray Entry Encoding & Locking (`fs/dax.c`)

- **Encoding:** DAX mappings do not store `struct folio *` pointers in `mapping->i_pages`; instead they store XArray value entries (`xa_is_value(entry)`) encoding the PFN shifted by `DAX_SHIFT` (`4`) and four flag bits: `DAX_LOCKED` (`1 << 0`), `DAX_PMD` (`1 << 1`), `DAX_ZERO_PAGE` (`1 << 2`), and `DAX_EMPTY` (`1 << 3`).
- **Locking:** Per-entry locking is serialized under `xas_lock_irq(&xas)` using `DAX_LOCKED` and the hashed `wait_table` (`get_unlocked_entry()` / `wait_entry_unlocked_exclusive()` / `put_unlocked_entry()`).
- **Wait Queues:** The `wait_table` uses `DAX_WAIT_TABLE_BITS` (12) to minimize collisions. When locking, the index is aligned to the PMD boundary if `DAX_PMD` is set, ensuring that all offsets within a PMD range share the same lock.
- **Invariant:** Never dereference a DAX XArray entry as a `struct folio *` or `struct page *` without first verifying `xa_is_value(entry)`.

## 2. `ZONE_DEVICE` Page Refcounts & `dax_layout_busy_page_range()`

- **Refcounts:** DAX `ZONE_DEVICE` pages are never onlined to the page allocator; an idle mapped DAX page has `page_ref_count(page) == 1` (not `0`). A page is busy (pinned by `get_user_pages()` / DMA) when its refcount exceeds `1` (`dax_busy_page()`).
- **Layout Break:** `dax_layout_busy_page()` / `dax_layout_busy_page_range()` calls `unmap_mapping_pages()` to zap page tables and force concurrent `get_user_pages_fast()` callers onto the PTE/PMD-locked slow path before scanning XArray entries for elevated refcounts.
- **Caller Locking Requirement:** Filesystems calling `dax_layout_busy_page*()` (during truncate, hole-punch, or layout break) must hold the locks that block new fault mappings (`filemap_invalidate_lock()` / `i_mmap_rwsem`) so new mappings cannot be re-established after `unmap_mapping_pages()`.

## 3. Fault & Writeback Entry Points

- **Fault Handling:** `dax_iomap_fault()` handles only `order == 0` (PTE) and `order == PMD_ORDER` (PMD) faults, returning `VM_FAULT_FALLBACK` for any other order.
- **Fault Locking:** Callers must hold filesystem fault locks (`filemap_invalidate_lock()` / `i_mmap_rwsem`) before entry.
- **Writeback:** `dax_writeback_mapping_range()` flushes dirty CPU cachelines for PMEM-backed XArray entries via `dax_flush()`.

## 4. Device-Level DAX (`drivers/dax/`)

- **Lifetimes:** `struct dev_dax` is managed via `devm_create_dev_dax()`. `kill_dev_dax()` must be called to quiesce the device before the driver's `remove()` callback completes.
- **Reference Counting:** `struct dax_region` uses `struct kref`. Ensure `put_dax()` is called on every successful `dax_dev_get()` or `alloc_dax()` path.
- **Registration:** A `dax_device` is visible to the bus only after `alloc_dax()` and subsequent registration. Do not expose the `dax_device` pointer until `dax_operations` are fully initialized.
- **Operations:** The `dax_operations` struct defines `direct_access`, which translates a device-relative logical-page-offset into an absolute physical PFN.

## 5. Synchronization & Concurrency Rules

- **Blocking Operations:** `dax_iomap_fault()` and related helpers may sleep; they must not be called from atomic contexts (interrupts, spinlocks).
- **Iterator Re-validation:** When dropping a lock to perform a blocking operation, the iterator state must be re-validated (e.g., `xas_reset()` or re-lookup) to ensure the pointer remains valid.
- **Alignment:** DAX operations are strictly aligned to `PAGE_SIZE` or `PMD_SIZE`. Use `IS_ALIGNED(offset, align)` to verify inputs before passing to `direct_access`.
- **Atomic Reads:** Shared pointers (e.g., `dax_device` holders) must be accessed using atomic primitives if they can be modified concurrently during teardown.

## 6. High-Frequency Bug Patterns & Verification

- **Units:** Distinguish between `pgoff_t` (page index) and `size_t` (bytes).
- **Null Checks:** Validate entries returned from `xas_load()` before conversion.
- **Folio Metadata:** When splitting a large `ZONE_DEVICE` folio, manually reset `order` and `nr_pages` to prevent metadata field overlaps.
- **Error Handling:** Always check return values from `fs_dax_get_by_bdev()` using `IS_ERR()`.
- **`direct_access`:** If `direct_access` returns an error, the caller must treat the range as non-DAX.
- **Stale Pointers:** Ensure all operation pointers are cleared when an object is unbound or destroyed to prevent use-after-free.
- **Initialization:** Ensure all pointers to dynamically allocated resources are cleared if the initialization process fails to prevent dangling pointers on probe failure.
- **Alignment:** Use appropriate alignment macros (floor vs ceiling) for specific memory offset requirements to prevent data corruption.
- **Lockdep:** Use `lockdep_assert_held()` to verify that `filemap_invalidate_lock()` is held in paths that modify the XArray or page tables.
- **Reentrancy:** Ensure that `dax_holder_operations` callbacks do not attempt to re-acquire locks held by the caller to prevent circular deadlocks.
- **Memory Failure:** `notify_failure` must be propagated through the `dax_holder` to ensure the filesystem can mark the affected blocks as bad.
- **Teardown:** The order of clearing shared state variables must be protected against concurrent access during object destruction to prevent race conditions in holder operations.
- **Consistency:** Shared pointers that can be modified concurrently must be accessed using atomic read primitives to ensure consistency between check and usage.
- **Validation:** All hardware-derived memory offsets must be validated against expected bounds before being used for address calculations to prevent silent misalignment.
- **Mapping:** Kernel virtual addresses must be derived using the correct mapping primitives rather than simple linear offsets to handle physical memory gaps.
