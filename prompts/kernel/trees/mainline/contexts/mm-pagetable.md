# MM Page Table Internals (`mm/memory.c`, `mm/mprotect.c`, `mm/pagewalk.c`)

## PTE State Invariants

Software-enforced PTE state invariants across `mm/`:
- **Writable implies dirty:** A clean+writable PTE is invalid. For shared mappings, `can_change_shared_pte_writable()` (`mm/mprotect.c`) only allows making a PTE writable when `pte_dirty(pte)` is true so clean shared pages take a write fault for filesystem writenotify. For private/anonymous mappings, paths set both together via `pte_mkwrite(pte_mkdirty(entry))` (`do_anonymous_page()`, `migrate_vma_insert_page()`).
- **Exception — `MADV_FREE`:** `madvise_free_pte_range()` (`mm/madvise.c`) clears the dirty bit via `clear_young_dirty_ptes()` while preserving write permission, intentionally creating a clean+writable PTE so the page can be reclaimed without writeback unless written again.
- **Exception — `MADV_DONTNEED`:** `zap_pte_range()` (`mm/memory.c`) uses `MADV_DONTNEED` semantics to discard page contents, requiring `pmd_clear()` or `pte_clear()` to invalidate the mapping and ensure subsequent accesses trigger a fresh fault.
- **Dirty implies accessed (young):** `pte_mkdirty()` does not set the accessed bit on x86 or arm64; callers constructing dirty PTEs must also mark them young.
- **Writable requires `VM_WRITE`:** Writable PTE construction must verify `VM_WRITE` in `vma->vm_flags` (e.g., via `maybe_mkwrite()`) because VMA permissions can change via `mprotect()`.
- **Migration A/D bits (`include/linux/swapops.h`):** `SWP_MIG_YOUNG_BIT` and `SWP_MIG_DIRTY_BIT` are only available when `migration_entry_supports_ad()` is true (`swap_migration_ad_supported`). `remove_migration_pte()` (`mm/migrate.c`) sets the PTE dirty only if both the migration entry and the folio are dirty.
- **NUMA balancing (`change_pte_range()` in `mm/mprotect.c`):** Skips PTEs that are already `pte_protnone()` and checks `folio_can_map_prot_numa()`.
- **Swap PTE comparison (`include/linux/swapops.h`):** Only exclusive, soft-dirty, and uffd-wp flags survive in swap PTEs; `pte_swp_clear_flags()` strips them to compare bare swap entries (`pte_same_as_swp()`).

## Non-Present PTE Type Dispatch (`softleaf_type`)

Non-present PTEs encode distinct swap entry categories via `softleaf_type` / `swp_type` (`include/linux/leafops.h`, `check_pte()` in `mm/page_vma_mapped.c`):
- **Migration (`SOFTLEAF_MIGRATION_READ`, `_READ_EXCLUSIVE`, `_WRITE`):** Folio temporarily unmapped during migration; checked via `softleaf_is_migration()`.
- **Device-private (`SOFTLEAF_DEVICE_PRIVATE_READ`, `_WRITE`):** Folio migrated to unaddressable device memory (HMM); checked via `softleaf_is_device_private()`.
- **Device-exclusive (`SOFTLEAF_DEVICE_EXCLUSIVE`):** CPU access temporarily revoked for device atomic operations while folio remains in host memory; checked via `softleaf_is_device_exclusive()`.
- **HW poison (`SOFTLEAF_HWPOISON`):** Uncorrectable memory error; checked via `softleaf_is_hwpoison()`.
- **Marker (`SOFTLEAF_MARKER`):** Metadata-only entry (uffd-wp or poison marker); checked via `softleaf_is_marker()`.

Sharing `softleaf_has_pfn()` does not make entry types interchangeable in dispatch branches. Device-private and device-exclusive entries maintain mapcount until `zap_nonpresent_ptes()`, whereas migration entries are managed by `try_to_migrate_one()`.

## Flag Transfer and Soft-Dirty Tracking

- **Non-present to present reconstruction:** Every path converting a swap, migration, or device-exclusive PTE to a present PTE (`do_swap_page()`, `restore_exclusive_pte()`, `remove_migration_pte()`, `try_to_map_unused_to_zeropage()`, `unuse_pte()`) must transfer soft-dirty and uffd-wp bits across encodings:
  ```c
  if (pte_swp_soft_dirty(old_pte))
      newpte = pte_mksoft_dirty(newpte);
  if (pte_swp_uffd_wp(old_pte))
      newpte = pte_mkuffd_wp(newpte);
  ```
  When copying non-present to non-present entries (`copy_nonpresent_pte()`), use `pte_swp_mksoft_dirty()` and `pte_swp_mkuffd_wp()`.
- **Soft-dirty on move/remap (`move_soft_dirty_pte()` in `mm/mremap.c`):** Soft-dirty (`pte_mksoft_dirty()` / `pte_swp_mksoft_dirty()`) tracks mapping changes for `/proc/pid/pagemap` and CRIU. Gate soft-dirty logic with `pgtable_supports_soft_dirty()` rather than `#ifdef CONFIG_MEM_SOFT_DIRTY` so runtime-detected architectural support (e.g., RISC-V) is handled.
- **VMA flag and PTE/PMD flag consistency:** Clearing a VMA flag such as `VM_UFFD_WP` or `VM_SOFT_DIRTY` requires clearing the corresponding bits across present PTEs/PMDs, swap PTEs, and PTE markers (see `clear_uffd_wp_pmd()` in `mm/huge_memory.c`).

## Large Folio PTE Installation Forward Progress

When `pte_range_none()` returns `false` during large folio installation because some PTEs in the range are already populated, the fault handler must ensure forward progress:
- **Page cache folios (`finish_fault()`):** Fall back to single-PTE installation. Returning `VM_FAULT_NOPAGE` without falling back to a single PTE causes an infinite fault livelock.
- **Freshly allocated anonymous folios (`do_anonymous_page()`):** Release the folio and retry at a smaller order via `alloc_anon_folio()`.
- **PMD level (`do_set_pmd()`):** Return `VM_FAULT_FALLBACK`.

## TLB Synchronisation, Per-VMA Lock Drain, and Page Table Teardown

- **TLB flushes after permission reduction:** Making a PTE less permissive (writable-to-readonly or present-to-non-present) requires a TLB flush (`change_pte_range()`, `zap_pte_range()`).
- **`flush_tlb_batched_pending(mm)` after PTL re-acquisition:** After dropping and re-acquiring the page table lock (PTL), call `flush_tlb_batched_pending(mm)` (`mm/rmap.c`) in case reclaim on another CPU queued batched TLB flushes while the lock was dropped.
- **Page table removal vs `gup_fast()`:** Clearing a PUD/PMD to free a page-table page requires `tlb_remove_table()` or `tlb_remove_table_sync_one()` (`mm/mmu_gather.c`) to synchronize against lockless `gup_fast()` walkers.
- **`vma_start_write(vma)` before page table access under `mmap_write_lock`:** `mmap_write_lock` does not revoke existing per-VMA read locks (`lock_vma_under_rcu()`). Any `mmap_write_lock` path must call `vma_start_write(vma)` before reading or modifying PMDs or PTEs.
- **Re-validation after PTL drop:** After dropping and re-acquiring PTL, concurrent threads may have repopulated entries; any decision to free page-table structures must be re-validated (`try_to_free_pte()`).
- **Page fault lock hierarchy:** `->fault` and `->page_mkwrite` run under `mmap_lock`, which nests inside `i_rwsem` and `sb_start_write`. Fault handlers must not block on filesystem freeze protection; copy user data with `copy_folio_from_iter_atomic()` (`generic_perform_write()`).
- **Page table walker loop advancement:** In `do { ... } while (pte++, addr += PAGE_SIZE, addr != end)` loops, advance the pointer and address unconditionally in the `while` condition so `continue` statements do not stall the loop.
- **Memory hotplug lock on unprotected `init_mm` walks:** Walking `init_mm` kernel page tables over hotplug-removable direct-map or vmemmap ranges requires `get_online_mems()` / `put_online_mems()` (acquired outer to `mmap_lock`) unless the range's backing object lifetime already prevents memory offlining or the caller already holds the memory hotplug lock.
