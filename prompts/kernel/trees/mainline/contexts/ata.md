# ATA / libata Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides architectural and safety invariants for the `libata` subsystem. Developers must adhere to these rules to prevent concurrency, lifecycle, and hardware-interfacing vulnerabilities.

## 1. Subsystem Architecture and Object Lifetimes

*   **Hierarchy:** The subsystem is structured as `ata_host` -> `ata_port` -> `ata_link` -> `ata_device`.
*   **Ownership:** `ata_host` is the top-level container. Objects are managed via `ata_port_alloc()` and `ata_host_alloc()`.
*   **Iteration:** Use `ata_link_next()` and `ata_dev_next()` for safe traversal. These helpers correctly handle PMP (Port Multiplier) links and slave devices.
*   **Log Directory and Quirk Handling (`drivers/ata/libata-core.c`):**
    *   `ata_dev_configure()` and `ata_read_log_directory()` probe device capabilities during initialization and error-handler (`libata-eh.c`) revalidation.
    *   Setting `dev->quirks |= ATA_QUIRK_NO_LOG_DIR` and calling `ata_clear_log_directory(dev)` permanently suppresses General Purpose Log probing for that device across subsequent reconfigurations.
    *   Static device quirks belong in `__ata_dev_quirks` (`drivers/ata/libata-core.c`); runtime callers must only disable log probing when I/O or structural corruption prevents safe parsing.

## 2. Synchronization Model and Concurrency Rules

*   **Locking Hierarchy:** The primary lock is `ap->lock` (the host lock).
*   **EH Context:** During Error Handling (`libata-eh.c`), the EH thread assumes exclusive control of the port. The host lock is often dropped and re-acquired; do not assume state consistency across blocking calls in EH.
*   **Atomic Contexts:** `qc_issue` and `qc_prep` callbacks are often called in atomic contexts. Never perform memory allocations (`GFP_KERNEL`) or blocking operations within these paths.
*   **Interrupts:** Hardware interrupts are masked at the controller level during EH. When writing to global interrupt registers (e.g., `HOST_CTL`), ensure bit-wise operations do not inadvertently clear unrelated pending interrupts.
*   **Deferred Commands:** `qc_defer` logic must be granular. Port-level deferral is incompatible with PMP Command-Based Switching. Track deferral state at the `ata_link` level to prevent cross-link interference.

## 3. Initialization, Registration, and Teardown

*   **Publication:** A device is "published" to the SCSI mid-layer only after `ata_port_probe()` completes successfully. Do not expose `ata_device` structures to sysfs or the SCSI layer before the device is fully configured.
*   **Symmetry:** Every `ata_tlink_add()` must have a corresponding `ata_tlink_delete()`.
*   **Error Unwind:** Use the "reverse-order" principle. If `ata_host_register()` fails, ensure all allocated resources (PHYs, regulators, IRQs) are released in the exact inverse order of acquisition.
*   **Premature Registration:** Never enable hardware interrupts before the `irq_handler` is registered and the `ata_port` state is initialized.

## 4. Error Handling and Resource Recovery

*   **IS_ERR vs NULL:** Always check return values of `devm_` or `of_` lookup functions using `IS_ERR()`.
*   **Sense Data:** When parsing sense buffers, clamp all length fields to the actual buffer size.
*   **State Invariants:** If a device reports zero capacity during revalidation, distinguish between transient hardware busy states and permanent failures.
*   **Resource Leaks:** Ensure `of_find_device_by_node()` references are released via `put_device()` on all exit paths.

## 5. Bug Patterns and False-Positive Guards

*   **Use-After-Free:** Do not access `pci_dev` or `ata_device` fields after the reference count has been dropped.
*   **Loop Iterators:** Ensure loop variables (e.g., `link`, `dev`) remain valid if the loop terminates early. Do not use the iterator variable to access the object after the loop finishes.
*   **Hardware Quirks:** Ensure all quirks applied during initial probe are also applied during `resume` from suspend.
*   **Register Clearing:** When clearing status registers, verify if the hardware requires a specific bit-mask. A full write-back (`= 0`) often clears unrelated pending events.
*   **DMA/Addressing:** Check hardware capabilities (e.g., 64-bit addressing support) before clearing or configuring DMA registers.

## 6. Power Management and Hardware Interfacing

*   **PHY/Regulator Balance:** Ensure PHY power-on/off counts are balanced.
*   **Timeout Values:** Use empirical worst-case values for hardware timeouts.
*   **LPM Policy:** When changing LPM (Link Power Management) policies, ignore PHY events for the first 10s to avoid spurious triggers caused by the transition.
*   **Register Access:** Always validate register base addresses against silicon specifications. Ensure cache flushes and memory barriers are performed as required by the architecture before and after DMA operations.
*   **Async I/O:** Ensure that buffer consumption logic in asynchronous interfaces is based on the actual number of bytes successfully processed, not the total requested size.
