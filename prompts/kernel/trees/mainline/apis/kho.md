# Kexec Handover (KHO) API Guide (Linux v7.3-rc5)

This guide defines the authoritative usage patterns for the Kexec Handover (KHO) subsystem, designed for reliable state transfer across kexec boundaries.

## 1. Enablement Gating (`kho_is_enabled` / `is_kho_boot`)

* **Mandatory `kho_is_enabled()` Gate**: Callers of serialization-side KHO APIs (`kho_add_subtree()`, `kho_remove_subtree()`, `kho_preserve_*()`) must check `kho_is_enabled()` first. When KHO is disabled, `kho_out.fdt` is `NULL` and calling `kho_add_subtree()` or `kho_remove_subtree()` dereferences `NULL`.
* **`is_kho_boot()`**: Returns true when the running kernel booted via a KHO-enabled kexec (reliable only after `kho_populate()` runs during early boot).

## 2. Memory Preservation and Restoration

* **Folio vs Order-0 Pages**:
  * `kho_preserve_folio()` / `kho_unpreserve_folio()` / `kho_restore_folio()` operate on compound folios and preserve the folio order across kexec.
  * `kho_preserve_pages()` / `kho_unpreserve_pages()` / `kho_restore_pages()` operate on contiguous ranges of order-0 pages (setting each page's refcount to 1 on restore). Never mix `kho_preserve_pages()` with `kho_restore_folio()`, and always pass the exact same `(page, nr_pages)` pair to `kho_unpreserve_pages()`.
* **vmalloc Preservation (`kho_preserve_vmalloc` / `kho_unpreserve_vmalloc`)**: Only supports `VM_ALLOC` and `VM_ALLOW_HUGE_VMAP` (`KHO_VMALLOC_SUPPORTED_FLAGS`); other flags fail with `-EOPNOTSUPP`.
* **Combined Allocation (`kho_alloc_preserve`)**: Allocates and preserves a zeroed power-of-two folio; pair with `kho_unpreserve_free()` on error/teardown or `kho_restore_free()` in the successor kernel.

## 3. FDT Subtree Registration (`kho_add_subtree` / `kho_remove_subtree` / `kho_retrieve_subtree`)

* `kho_add_subtree(name, fdt)` only records the physical address of `fdt` in the KHO root tree (returning `-EEXIST` on duplicate names); the caller must independently preserve the backing memory of `fdt` via `kho_preserve_folio()` or `kho_preserve_pages()`.
* `kho_remove_subtree(fdt)` unlinks the subtree entry without unpreserving or freeing the backing memory.
* `kho_retrieve_subtree(name, phys)` looks up a subtree in the successor kernel (`phys` must be converted with `phys_to_virt()`).

## 4. High-Frequency Bug Patterns & Review Invariants

### Memory Safety & Alignment
* **Alignment Invariant**: All scratch memory and preserved regions must respect `SCRATCH_ALIGNMENT_BYTES` (`PAGE_SIZE * MAX_ORDER_NR_PAGES`). Failure to align results in `deferred_init_memmap_chunk` crashes during buddy allocator initialization.
* **Compound Pages**: `kho_preserve_folio()` is the only valid interface for compound pages. Using `kho_preserve_pages()` on compound pages corrupts the `page->private` metadata used by KHO.
* **Integer Overflow**: When calculating sizes for `kho_preserve_pages()` or `kho_add_subtree()`, use `unsigned long` or `size_t` to prevent 32-bit truncation on 64-bit systems.

### State Machine & Lifecycle
* **Symmetry**: Every `kho_preserve_*` call must have a matching `kho_unpreserve_*` call on error paths to prevent KHO metadata leaks.
* **Context**: KHO APIs are not safe for hard interrupt contexts. Use `GFP_KERNEL` for allocations.
* **KASAN**: When restoring `vmalloc` regions via `kho_restore_vmalloc()`, ensure the memory is manually unpoisoned; KHO does not automatically register restored regions with KASAN.

### Review Checklist
1.  **Is `kho_is_enabled()` checked?** (Prevents `NULL` dereference).
2.  **Is the alignment `SCRATCH_ALIGNMENT_BYTES`?** (Prevents buddy allocator corruption).
3.  **Are `page->private` fields preserved?** (KHO uses this for `KHO_PAGE_MAGIC` and order; do not overwrite).
4.  **Is the allocator matched?** (e.g., `kho_alloc_preserve` must be paired with `kho_unpreserve_free` or `kho_restore_free`).
5.  **Are log messages accurate?** (Ensure `pr_fmt` is used and messages reflect the actual state of the handover).
6.  **Are error paths complete?** (Ensure `kho_remove_subtree` or `kho_unpreserve_*` is called on failure).
7.  **Is `phys_to_virt` used correctly?** (Required for `kho_retrieve_subtree` results).
8.  **Are radix keys encoded correctly?** (Use `kho_encode_radix_key` and `kho_decode_radix_key` to maintain consistency across kexec).
9.  **Is the FDT blob size validated?** (Ensure `size` passed to `kho_add_subtree` matches the actual blob size to prevent buffer overruns).
10. **Are memoryless NUMA nodes handled?** (Ensure per-node allocations skip memoryless nodes to prevent initialization failures).

## 5. Concurrency and Synchronization
* **Locking**: `kho_out.lock` protects the FDT and radix tree during serialization. Callers must not hold this lock across blocking operations.
* **Radix Tree**: Use `kho_encode_radix_key` to generate keys for `kho_radix_add_key`. The key width is limited to `KHO_RADIX_KEY_WIDTH`; exceeding this requires an ABI bump.
* **Memory Ordering**: When publishing objects, ensure memory barriers are implicit in the `kho_add_subtree` call, which serializes access via the internal mutex.
