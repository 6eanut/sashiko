# PCI Endpoint & IRQ Vector API (`include/linux/pci.h`, `include/linux/pci-epc.h`, `include/linux/pci-epf.h`)

## PCI Endpoint Error Return Conventions (`pci_epc_get`, `pci_epf_create`)

- `pci_epc_get()` (`drivers/pci/endpoint/pci-epc-core.c`) returns `ERR_PTR(-EINVAL)` or another error pointer on failure, never `NULL`. Check the return value with `IS_ERR()`, not `!epc`. (`pci_epc_put()` checks `IS_ERR_OR_NULL()` internally.)
- `pci_epf_create()` (`drivers/pci/endpoint/pci-epf-core.c`) returns `ERR_PTR()` on failure, never `NULL`. Check the return value with `IS_ERR()`, not `!epf`. Never pass an `ERR_PTR` value to `pci_epf_destroy()`, which unconditionally dereferences `epf`.

## PCI IRQ Vector Allocation and Cleanup (`pci_alloc_irq_vectors` / `pci_free_irq_vectors`)

- `pci_alloc_irq_vectors(pdev, min_vecs, max_vecs, flags)` returns the negative errno on failure or the positive number of allocated vectors on success.
- Once `pci_alloc_irq_vectors()` succeeds, every subsequent error path in probe/init (and the driver remove/teardown path) must call `pci_free_irq_vectors(pdev)` (unless managed via `pcim_enable_device()`).
