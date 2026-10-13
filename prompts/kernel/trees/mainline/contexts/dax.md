# DAX (Direct Access) Subsystem Context (`fs/dax.c`, `drivers/dax/`)

## XArray Entry Encoding & Locking (`fs/dax.c`)

- DAX mappings do not store `struct folio *` pointers in `mapping->i_pages`; instead they store XArray value entries (`xa_is_value(entry)`) encoding the PFN shifted by `DAX_SHIFT` (`4`) and four flag bits: `DAX_LOCKED` (`1 << 0`), `DAX_PMD` (`1 << 1`), `DAX_ZERO_PAGE` (`1 << 2`), and `DAX_EMPTY` (`1 << 3`).
- Per-entry locking is serialized under `xas_lock_irq(&xas)` using `DAX_LOCKED` and the hashed `wait_table` (`get_unlocked_entry()` / `wait_entry_unlocked_exclusive()` / `put_unlocked_entry()`).

## `ZONE_DEVICE` Page Refcounts & `dax_layout_busy_page_range()`

- DAX `ZONE_DEVICE` pages are never onlined to the page allocator, so an idle mapped DAX page has `page_ref_count(page) == 1` (not `0`). A page is busy (pinned by `get_user_pages()` / DMA) when its refcount exceeds `1` (`dax_busy_page()`).
- `dax_layout_busy_page()` / `dax_layout_busy_page_range()` calls `unmap_mapping_pages()` to zap page tables and force concurrent `get_user_pages_fast()` callers onto the PTE/PMD-locked slow path before scanning XArray entries for elevated refcounts.
- **Caller locking requirement:** Filesystems calling `dax_layout_busy_page*()` (during truncate, hole-punch, or layout break) must hold the locks that block new fault mappings (`filemap_invalidate_lock()` / `i_mmap_rwsem`) so new mappings cannot be re-established after `unmap_mapping_pages()`.

## Fault & Writeback Entry Points

- `dax_iomap_fault()` handles only `order == 0` (PTE) and `order == PMD_ORDER` (PMD) faults, returning `VM_FAULT_FALLBACK` for any other order, and requires the caller to hold filesystem fault locks before entry.
- `dax_writeback_mapping_range()` flushes dirty CPU cachelines for PMEM-backed XArray entries via `dax_flush()`.
