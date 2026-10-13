# KHO (Kexec Handover) APIs (`include/linux/kexec_handover.h`)

## Enablement Gating (`kho_is_enabled` / `is_kho_boot`)

* **Mandatory `kho_is_enabled()` Gate**: Callers of serialization-side KHO APIs (`kho_add_subtree()`, `kho_remove_subtree()`, `kho_preserve_*()`) must check `kho_is_enabled()` first. When KHO is disabled, `kho_out.fdt` is `NULL` and calling `kho_add_subtree()` or `kho_remove_subtree()` dereferences `NULL`.
* **`is_kho_boot()`**: Returns true when the running kernel booted via a KHO-enabled kexec (reliable only after `kho_populate()` runs during early boot).

## Memory Preservation and Restoration

* **Folio vs Order-0 Pages**:
  * `kho_preserve_folio()` / `kho_unpreserve_folio()` / `kho_restore_folio()` operate on compound folios and preserve the folio order across kexec.
  * `kho_preserve_pages()` / `kho_unpreserve_pages()` / `kho_restore_pages()` operate on contiguous ranges of order-0 pages (setting each page's refcount to 1 on restore). Never mix `kho_preserve_pages()` with `kho_restore_folio()`, and always pass the exact same `(page, nr_pages)` pair to `kho_unpreserve_pages()`.
* **vmalloc Preservation (`kho_preserve_vmalloc` / `kho_unpreserve_vmalloc`)**: Only supports `VM_ALLOC` and `VM_ALLOW_HUGE_VMAP` (`KHO_VMALLOC_SUPPORTED_FLAGS`); other flags fail with `-EOPNOTSUPP`.
* **Combined Allocation (`kho_alloc_preserve`)**: Allocates and preserves a zeroed power-of-two folio; pair with `kho_unpreserve_free()` on error/teardown or `kho_restore_free()` in the successor kernel.

## FDT Subtree Registration (`kho_add_subtree` / `kho_remove_subtree` / `kho_retrieve_subtree`)

* `kho_add_subtree(name, fdt)` only records the physical address of `fdt` in the KHO root tree (returning `-EEXIST` on duplicate names); the caller must independently preserve the backing memory of `fdt` via `kho_preserve_folio()` or `kho_preserve_pages()`.
* `kho_remove_subtree(fdt)` unlinks the subtree entry without unpreserving or freeing the backing memory.
* `kho_retrieve_subtree(name, phys)` looks up a subtree in the successor kernel (`phys` must be converted with `phys_to_virt()`).
