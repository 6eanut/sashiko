# PCI Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for the Linux PCI subsystem. Developers must adhere to these rules to prevent common vulnerabilities, including use-after-free, resource leaks, and hardware state corruption.

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **Core Objects:** The hierarchy is defined by `struct pci_bus`, `struct pci_dev`, and `struct pci_slot`. 
*   **Reference Counting:** `pci_dev` objects are reference-counted via `pci_dev_get()` and `pci_dev_put()`. 
    *   **Invariant:** Any function returning a `pci_dev` pointer (e.g., `pci_get_domain_bus_and_slot()`) increments the reference count. The caller **must** call `pci_dev_put()` on all exit paths, including error returns.
*   **Object Lifecycle:** A `pci_dev` must not be accessed after its reference count reaches zero. Drivers must not cache `pci_dev` pointers without holding a reference during asynchronous teardown or hotplug events.
*   **Slot Identity:** `struct pci_slot` represents a physical slot. For multi-function devices, ensure `per_func_slot` is correctly set if the hardware requires per-function reset granularity.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Configuration Space Access:** Access to PCI configuration space is serialized via `pci_bus_sem`. 
*   **Register Access:** Hardware register modifications (e.g., ASPM, MSI-X) must be performed using atomic read-modify-write sequences. 
*   **Locking Hierarchy:** 
    1.  `pci_bus_sem` (Global/Bus-level)
    2.  `device_lock` (Driver-level)
    *   **Rule:** Never hold a spinlock while performing blocking operations (e.g., `msleep()`, `pci_read_config_*` on slow buses).
*   **Interrupt Context:** Interrupt handlers must not perform blocking PCI configuration reads/writes. Use deferred work (e.g., `work_struct`) if configuration access is required.

## 3. Initialization, Registration & Error Unwind

*   **Probe Sequence:** A driver must not register or publish a device until all internal state (clocks, power domains, MSI-X vectors) is fully initialized.
*   **Symmetric Unwind:** Every `pci_enable_device()` must be paired with `pci_disable_device()`. Every `pci_alloc_irq_vectors()` must be paired with `pci_free_irq_vectors()`.
*   **Error Path Invariants:**
    *   **IS_ERR vs NULL:** Always use `IS_ERR()` to check return values of functions like `pci_get_slot()`. Do not assume `NULL` is the only error sentinel.
    *   **Resource Cleanup:** Use `devm_*` managed resources where possible to automate cleanup. If manual allocation is used, ensure the cleanup order is the exact reverse of the allocation order.
*   **Hardware Sequencing:** Follow the specification strictly. Do not enable clocks before asserting core resets. Refer to `PCIE_T_PVPERL_MS` and `PCIE_RESET_CONFIG_WAIT_MS` for mandatory timing delays.

## 4. Asynchronous Work, Interrupts & Teardown Quiescing

*   **Quiescing:** Before freeing a `pci_dev` or unloading a driver:
    1.  Disable interrupts (`pci_disable_msi`/`pci_disable_msix`).
    2.  Cancel all pending `work_struct` and `timer_list` objects.
    3.  Ensure no hardware DMA is in flight.
*   **Interrupt Storms:** If a device triggers spurious interrupts, the driver must implement a threshold-based disable or a "mask-and-poll" mechanism to prevent CPU lockup.

## 5. Power Management & Runtime Suspend-Resume

*   **Power References:** Use `pm_runtime_get_sync()` and `pm_runtime_put()` to manage power states. 
*   **Context Save:** Volatile hardware registers must be saved to memory before entering `D3hot` or `D3cold` and restored upon resume.
*   **Atomic Context:** Suspend/Resume callbacks are often called in atomic contexts; avoid memory allocations with `GFP_KERNEL`. Use `GFP_ATOMIC` if allocation is strictly necessary.

## 6. High-Frequency Bug Patterns & False-Positive Guards

*   **Out-of-Bounds Access:** Always validate MMIO offsets against the BAR size. Never assume the hardware register block size matches a software-defined struct size.
    *   **Guard:** `if (offset + size > pci_resource_len(dev, bar)) return -EINVAL;`
*   **Register Width:** Ensure the access width (8, 16, 32-bit) matches the hardware specification. 
*   **Firmware Nodes:** Use firmware node identity mapping (e.g., `fwnode_handle`) instead of deprecated name-based matching for GPIO or resource lookups.

## 7. PCI Bridge Emulation (Specific to `pci-bridge-emul`)

*   **Callback Contracts:** When implementing `pci_bridge_emul_ops`, the `read_base` and `write_base` callbacks must return `PCI_BRIDGE_EMUL_HANDLED` only if the operation is fully completed. If the operation is not handled, the common code will fall back to the in-memory configuration space copy.
*   **State Consistency:** Ensure the `pci_bridge_emul_conf` structure is kept in sync with the hardware state. Any write to the emulated configuration space must trigger the corresponding hardware register update.
