# Page Table Accessors, Batching, and Walkers (`include/linux/pgtable.h`, `include/linux/pagewalk.h`)

## Special vs Normal Page Table Mappings
- Normal refcounted folios must NOT have their page table entry marked special (`pte_mkspecial()`, `pmd_mkspecial()`, `pud_mkspecial()`).
- Only raw PFN mappings (`VM_PFNMAP`, `VM_MIXEDMAP` without `struct page`), devmap entries (`pte_mkdevmap()`, `pmd_mkdevmap()`, `pud_mkdevmap()`), and the shared zero folios may be marked special.
- Use `folio_mk_pmd()` / `folio_mk_pud()` (`include/linux/mm.h`) when constructing huge entries for normal refcounted folios; use `pfn_pmd()` + `pmd_mkspecial()` or `pfn_pud()` + `pud_mkspecial()` only for raw PFN mappings.
- `vm_normal_page_pmd()` and `vm_normal_page_pud()` are used to identify if a huge page table entry points to a normal refcounted folio; they return `NULL` for special mappings.

## Page Table Entry to Folio/Page Conversion Preconditions
- `pte_t` represents a standard page table entry. Always verify `pte_present(pte)` before calling `pte_page(pte)` or `vm_normal_page()`.
- `pmd_folio(pmd)` and `pmd_page(pmd)` require `pmd_present(pmd)`, `pmd_trans_huge(pmd)`, or `pmd_devmap(pmd)`. `pmd_trans_huge_lock()` succeeds for both present THP PMDs and non-present PMD leaf entries; callers must verify `pmd_present()` before `pmd_folio()` / `pmd_page()`.
- For non-present migration or device entries, extract the `softleaf_t` via `softleaf_from_pmd()` or `softleaf_from_pte()` and convert with `softleaf_to_folio()` (`include/linux/leafops.h`).
- `softleaf_is_device_exclusive()` and `softleaf_is_device_private()` must be used to identify specific non-present leaf types before accessing swap-specific metadata.

## Page Table Population and Hierarchy
- `pgd_populate(mm, pgd, p4d)` and `p4d_populate(mm, p4d, pud)` are used to link page table levels. These must be called when initializing new page table pages to ensure the hardware walker can traverse the hierarchy.
- `p4d_populate` and `pgd_populate` must be paired with appropriate memory barriers if the architecture requires them for visibility to the MMU.

## PTE Batching (`folio_pte_batch` and `folio_pte_batch_flags`)
- `set_ptes()` advances the PFN across the batch and stamps the first PTE's permission and dirty bits onto all entries.
- `folio_pte_batch()` calls `folio_pte_batch_flags()` with `flags = 0`. It is safe only when unmapping or clearing (e.g., `zap_present_ptes()`, `folio_unmap_pte_batch()`).
- When writing batched PTEs via `set_ptes()`, use `folio_pte_batch_flags()` with `FPB_RESPECT_WRITE` (and `FPB_RESPECT_DIRTY` / `FPB_RESPECT_SOFT_DIRTY` or `FPB_MERGE_WRITE` / `FPB_MERGE_YOUNG_DIRTY`) to prevent overwriting differing permissions.
- **PMD boundary cap:** Callers must cap `max_nr` at `(pmd_addr_end(addr, vma->vm_end) - addr) >> PAGE_SHIFT` to prevent OOB reads.
- **Independent folio bound:** Loops must bound iteration using `folio_nr_pages()` rather than `pte_same()` against `pte_advance_pfn()`, as Xen PV can produce `pte_none()` for valid machine frames.

## `page_vma_mapped_walk()` Non-Present Entries
- `page_vma_mapped_walk()` can return `true` with `pvmw.pte` pointing to a non-present PFN swap PTE, even if `PVMW_MIGRATION` is not set.
- Always check `pte_present(ptep_get(pvmw.pte))` before calling present-PTE accessors (`pte_pfn()`, `pte_dirty()`, `pte_write()`, `pte_young()`, `pte_soft_dirty()`, `pte_uffd_wp()`).
- For non-present PFN swap PTEs, convert via `softleaf_from_pte()`, use `softleaf_to_pfn()` / `softleaf_is_device_private_write()`, and use swap-PTE flag accessors (`pte_swp_soft_dirty()`, `pte_swp_uffd_wp()`).

## Page Table Walkers (`walk_page_range` and `mm_walk_ops`)
- **`pmd_entry` and THP:** `pmd_entry` receives every non-empty PMD including `pmd_trans_huge()`. If defined without `pte_entry`, the walker does not descend. Read PMD locklessly with `pmdp_get_lockless()`, re-read under `pmd_lock()` for THP, check `pte_offset_map_lock()` for `NULL`, and take `folio_get()` before dropping PTL.
- **`VM_PFNMAP` default skip:** Without a custom `.test_walk`, `walk_page_test()` silently skips `VM_PFNMAP` VMAs.
- **`ACTION_AGAIN` retry limit:** Do not set `ACTION_AGAIN` on `pte_offset_map_lock()` failure; it creates an infinite loop. `walk_pte_range()` handles transient retries internally.

## PTE Mapping, Kernel Page Tables, Lazy MMU, and Helpers
- **`pte_offset_map()` / `pte_unmap()` and `CONFIG_HIGHPTE`:** `pte_unmap()` must receive the exact pointer returned by `pte_offset_map()` or `pte_offset_map_lock()`. After incrementing `ptep`, pass the saved start pointer or `ptep - 1` to `pte_unmap_unlock()`. Multiple mappings must be unmapped in strict LIFO order (`kunmap_local()` requirement).
- **Kernel page table population:** Use `pgd_populate_kernel()` / `p4d_populate_kernel()` to ensure `arch_sync_kernel_mappings()` is invoked. Use `break` in loops accumulating `pgtbl_mod_mask` to avoid skipping synchronization.
- **Lazy MMU mode (`arch_enter_lazy_mmu_mode` / `arch_leave_lazy_mmu_mode`):**
  - Error paths must `break` to reach `arch_leave_lazy_mmu_mode()`, never `return` early.
  - PTE reads after writes inside lazy MMU mode may return stale values unless bracketed by leave/enter.
  - Lazy MMU mode disables preemption on architectures such as sparc and powerpc; callbacks and loops inside lazy MMU mode must not sleep (`GFP_ATOMIC` / `GFP_NOWAIT`).
- **Page comparison:** Use `pages_identical()` to ensure architecture metadata (e.g., MTE tags) is checked, rather than `memchr_inv()`.
- **Bit-lock barriers:** Pair `test_and_set_bit_lock()` (acquire) with `clear_bit_unlock()` (release). When using `PGDAT_RECLAIM_LOCKED`, never use relaxed `clear_bit()`.
- **Address Masking:** Always use `PAGE_MASK` when performing arithmetic on PTE pointers to ensure alignment and prevent crossing into adjacent page table pages.
