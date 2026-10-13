# PCI Endpoint & IRQ Vector API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for the PCI Endpoint (EPC/EPF) and MSI/MSI-X subsystems. Adherence to these patterns is mandatory for all drivers targeting the Linux v7.3-rc5 kernel.

## 1. PCI Endpoint Error Return Conventions
*   **`pci_epc_get()` (`drivers/pci/endpoint/pci-epc-core.c`):** Returns `ERR_PTR(-EINVAL)` or another error pointer on failure, never `NULL`. Check the return value with `IS_ERR()`, not `!epc`.
*   **`pci_epf_create()` (`drivers/pci/endpoint/pci-epf-core.c`):** Returns `ERR_PTR()` on failure, never `NULL`. Check the return value with `IS_ERR()`, not `!epf`.
*   **Destruction Safety:** Never pass an `ERR_PTR` value to `pci_epf_destroy()`, which unconditionally dereferences `epf`.
*   **Refcounting:** `pci_epc_put()` checks `IS_ERR_OR_NULL()` internally. Every successful `pci_epc_get()` must be balanced by a corresponding `pci_epc_put()` on all exit paths, including error handling and module removal.

## 2. PCI IRQ Vector Allocation and Cleanup
*   **`pci_alloc_irq_vectors(pdev, min_vecs, max_vecs, flags)`:** Returns the negative `errno` on failure or the positive number of allocated vectors on success.
*   **Cleanup:** Once `pci_alloc_irq_vectors()` succeeds, every subsequent error path in probe/init (and the driver remove/teardown path) must call `pci_free_irq_vectors(pdev)` (unless managed via `pcim_enable_device()`).

## 3. Execution Context and Synchronization
*   **Sleeping Constraints:** Most `pci_epc_*` and `pci_epf_*` operations (e.g., `pci_epc_write_header`, `pci_epc_set_bar`) involve hardware communication and **may sleep**. They must not be called from atomic contexts (interrupt handlers, `spin_lock` critical sections).
*   **Locking:** The subsystem manages internal state via mutexes. Do not attempt to hold custom spinlocks while calling into the EPC core to avoid "scheduling while atomic" bugs.

## 4. Object Ownership and Lifetime
*   **Initialization Rollback:** If a probe function fails after acquiring multiple resources, you must unwind in the exact reverse order of acquisition. Failure to do so is the primary cause of reference leaks and kernel panics during module unloading.
*   **Asynchronous Handoff:** When passing an object to an asynchronous worker, ensure the reference count is incremented before scheduling and dropped by the worker upon completion.

## 5. Concurrency and Memory Ordering
*   **State Publication:** When initializing a new `pci_epf` device, ensure all internal state is fully configured and validated *before* calling registration functions that expose the device to the core.
*   **Memory Barriers:** If implementing a custom backend, use memory barriers when publishing state to shared structures to guarantee that the host-side observes the initialized configuration before the "Link Up" event is signaled.

## 6. Registration and Teardown Invariants
*   **Premature Exposure:** Never register a device or expose it to configfs until the underlying hardware is fully initialized.
*   **Synchronous Cancellation:** During `remove` or `unbind`, you must synchronously cancel all pending work (e.g., `cancel_work_sync`) and disable hardware interrupts before freeing associated memory.

## 7. Buffer Sizing and DMA Alignment
*   **BAR Mapping:** Verify that the requested size and offset are within the bounds defined by the EPC hardware. Accessing offsets beyond the mapped BAR size is a common cause of page faults.
*   **Alignment:** Always respect the alignment constraints returned by `pci_epc_ops->align_addr`. Failure to align DMA buffers to the controller's requirements results in silent data corruption or hardware errors.

## 8. Device-Managed Cleanup (Devres)
*   **Prefer `pcim_*`:** Use device-managed functions (e.g., `pcim_enable_device()`, `pcim_iomap()`) to handle resource release on driver detach.
*   **Manual vs. Managed:** Do not mix manual `kfree()` or `pci_free_irq_vectors()` with `devm_*` managed resources for the same object to avoid double-free vulnerabilities.

## 9. High-Frequency Bug Patterns (Review Checklist)
*   **[ ] IS_ERR vs NULL:** Did you check `IS_ERR()` for `pci_epc_get` and `pci_epf_create`?
*   **[ ] Unbalanced Refcount:** Is there a `pci_epc_put` for every `pci_epc_get` on *all* exit paths?
*   **[ ] IRQ Cleanup:** Did you call `pci_free_irq_vectors` in the `remove` routine and all error paths after a successful `pci_alloc_irq_vectors`?
*   **[ ] Bounds Checking:** Are all MMIO offsets verified against the BAR size before access?
*   **[ ] Atomic Context:** Are you calling `pci_epc_*` functions from a process context? (If not, you are violating the sleep constraint).
*   **[ ] Rollback Order:** Does the error path unwind resources in the exact reverse order of their acquisition?
*   **[ ] Atomic RMW:** Are hardware register modifications performed using atomic read-modify-write accessors?
*   **[ ] Capability Checks:** Are hardware-specific features gated by capability checks (e.g., `pci_epc_get_features`) before invocation?
*   **[ ] Auxiliary Resources:** When using `pci_epc_get_aux_resources`, ensure the `num_resources` matches the array size to prevent buffer overflows.
*   **[ ] BAR Reservation:** Use `pci_epc_get_first_free_bar` and `pci_epc_get_next_free_bar` to avoid collisions with reserved BARs.
