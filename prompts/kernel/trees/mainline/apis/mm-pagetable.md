# Page Table Accessors, Batching, and Walkers (`include/linux/pgtable.h`, `include/linux/pagewalk.h`)

## Special vs Normal Page Table Mappings

Marking a normal refcounted folio's page table entry as special causes `vm_normal_page()` (and `vm_normal_page_pmd()` / `vm_normal_page_pud()`) to return `NULL`, hiding the folio from page table walkers, GUP, and refcount management.
- Normal refcounted folios must NOT have their page table entry marked special (`pte_mkspecial()`, `pmd_mkspecial()`, `pud_mkspecial()`).
- Only raw PFN mappings (`VM_PFNMAP`, `VM_MIXEDMAP` without `struct page`), devmap entries (`pte_mkdevmap()`, `pmd_mkdevmap()`, `pud_mkdevmap()`), and the shared zero folios may be marked special.
- Use `folio_mk_pmd()` / `folio_mk_pud()` (`include/linux/mm.h`) when constructing huge entries for normal refcounted folios; use `pfn_pmd()` + `pmd_mkspecial()` or `pfn_pud()` + `pud_mkspecial()` only for raw PFN mappings.

## Page Table Entry to Folio/Page Conversion Preconditions

Present-entry conversion helpers interpret physical PFN bits directly and produce bogus `struct page *` pointers if called on non-present entries (migration, swap, or hwpoison entries):
- `pmd_folio(pmd)` and `pmd_page(pmd)` require `pmd_present(pmd)`, `pmd_trans_huge(pmd)`, or `pmd_devmap(pmd)`. Note that `pmd_trans_huge_lock()` succeeds for both present THP PMDs and non-present PMD leaf entries (migration, device-private), so callers must still verify `pmd_present()` before `pmd_folio()` / `pmd_page()`.
- `pte_page(pte)` and `vm_normal_page()` require `pte_present(pte)`.
- For non-present migration or device entries, extract the `softleaf_t` via `softleaf_from_pmd()` or `softleaf_from_pte()` and convert with `softleaf_to_folio()` (`include/linux/leafops.h`).

## PTE Batching (`folio_pte_batch` and `folio_pte_batch_flags`)

`set_ptes()` only advances the PFN across the batch and stamps the first PTE's permission and dirty bits onto all entries in the batch.
- `folio_pte_batch()` (`mm/util.c`) calls `folio_pte_batch_flags()` (`mm/internal.h`) with `flags = 0`, ignoring differences in writable, dirty, and soft-dirty bits. It is safe only when the caller unmaps or clears the batch (e.g., `zap_present_ptes()`, `folio_unmap_pte_batch()`).
- When writing batched PTEs back via `set_ptes()`, callers must use `folio_pte_batch_flags()` with `FPB_RESPECT_WRITE` (and `FPB_RESPECT_DIRTY` / `FPB_RESPECT_SOFT_DIRTY` or `FPB_MERGE_WRITE` / `FPB_MERGE_YOUNG_DIRTY` as appropriate) so PTEs with different permissions are not overwritten.
- **PMD boundary cap (`max_nr`):** Callers of `folio_pte_batch()` / `folio_pte_batch_flags()` must cap `max_nr` so scanning stays within a single PTE table and VMA (typically `(pmd_addr_end(addr, vma->vm_end) - addr) >> PAGE_SHIFT`). Deriving `max_nr` purely from `folio_nr_pages()` without capping at the PMD boundary reads out of bounds past the end of the page table.
- **Independent folio bound:** Loops batching consecutive PTEs must bound iteration using folio metadata (`folio_nr_pages()`) rather than relying solely on `pte_same()` against `pte_advance_pfn()`, because on Xen PV `pte_advance_pfn()` can produce `pte_none()` for PFNs without valid machine frames.

## `page_vma_mapped_walk()` Non-Present Entries

`page_vma_mapped_walk()` (`mm/page_vma_mapped.c`) can return `true` with `pvmw.pte` pointing to a non-present PFN swap PTE (`softleaf_is_device_exclusive()` or `softleaf_is_device_private()`) even when `PVMW_MIGRATION` is not set.
- When `pvmw.pte` is set, always check `pte_present(ptep_get(pvmw.pte))` before calling present-PTE accessors (`pte_pfn()`, `pte_dirty()`, `pte_write()`, `pte_young()`, `pte_soft_dirty()`, `pte_uffd_wp()`).
- For non-present PFN swap PTEs, convert via `softleaf_from_pte()`, use `softleaf_to_pfn()` / `softleaf_is_device_private_write()`, and use swap-PTE flag accessors (`pte_swp_soft_dirty()`, `pte_swp_uffd_wp()`), which read different bit positions than present-PTE accessors.

## Page Table Walkers (`walk_page_range` and `mm_walk_ops`)

- **`pmd_entry` and THP (`include/linux/pagewalk.h`):** `pmd_entry` receives every non-empty PMD including `pmd_trans_huge()`. When `pmd_entry` is defined without `pte_entry`, the walker does not descend to PTEs automatically. Read the PMD locklessly with `pmdp_get_lockless()`, re-read under `pmd_lock()` for THP, check `pte_offset_map_lock()` for `NULL`, and take `folio_get()` before dropping PTL if returning a folio reference.
- **`VM_PFNMAP` default skip:** Without a custom `.test_walk` callback, the default `walk_page_test()` silently skips `VM_PFNMAP` VMAs and returns `0`. Callers that must visit all VMAs must provide a `.test_walk` callback returning `0`.
- **`ACTION_AGAIN` retry limit:** `ACTION_AGAIN` retries indefinitely. Because `pte_offset_map_lock()` returns `NULL` non-transiently for PMD migration entries, setting `ACTION_AGAIN` on `pte_offset_map_lock()` failure creates an infinite loop (`walk_pte_range()` already handles transient retries internally).

## PTE Mapping, Kernel Page Tables, Lazy MMU, and Helpers

- **`pte_offset_map()` / `pte_unmap()` and `CONFIG_HIGHPTE`:**
  - `pte_unmap()` must receive the exact pointer returned by `pte_offset_map()` or `pte_offset_map_lock()` (never the address of a local `pte_t` stack variable).
  - After a loop that increments `ptep`, `ptep` may point one-past-the-end of the PTE page (`PAGE_MASK` crosses into the next page on 32-bit `CONFIG_HIGHPTE`). Pass the saved start pointer or `ptep - 1` to `pte_unmap_unlock()`.
  - Multiple concurrent PTE mappings must be unmapped in strict LIFO order (`kunmap_local()` requirement).
- **Kernel page table population (`pgd_populate_kernel` / `p4d_populate_kernel`):** `pgd_populate()` and `p4d_populate()` do not synchronize kernel mappings across processes. Use `pgd_populate_kernel()` / `p4d_populate_kernel()` (which invoke `arch_sync_kernel_mappings()`), and ensure loops accumulating `pgtbl_mod_mask` use `break` rather than early `return` on error so `arch_sync_kernel_mappings()` is not skipped.
- **Lazy MMU mode (`arch_enter_lazy_mmu_mode` / `arch_leave_lazy_mmu_mode`):**
  - Error paths inside lazy MMU mode must `break` to reach `arch_leave_lazy_mmu_mode()`, never `return` early.
  - PTE reads after writes inside lazy MMU mode may return stale values unless bracketed by leave/enter.
  - Lazy MMU mode disables preemption on architectures such as sparc and powerpc; callbacks and loops inside lazy MMU mode must not sleep (`GFP_ATOMIC` / `GFP_NOWAIT`).
- **Page comparison (`pages_identical()`):** Use `pages_identical()` rather than raw `memchr_inv()` / `memcmp()` when deciding whether to merge pages or remap to `ZERO_PAGE(0)`, so architecture metadata such as arm64 MTE tags are checked.
- **Bit-lock barriers (`test_and_set_bit_lock` / `clear_bit_unlock`):** When a bit is used for mutual exclusion (e.g., `PGDAT_RECLAIM_LOCKED`), pair `test_and_set_bit_lock()` (acquire) with `clear_bit_unlock()` (release), not relaxed `clear_bit()`.
