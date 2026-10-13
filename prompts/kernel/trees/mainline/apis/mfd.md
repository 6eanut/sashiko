# Multi-Function Device (MFD) API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for the MFD subsystem (`include/linux/mfd/core.h`), focusing on device lifecycle, registration, and error handling.

## 1. Core API Overview & Ownership
The MFD subsystem facilitates the registration of child `platform_device` objects from a single parent device.

*   **Ownership:** When `mfd_add_devices()` or `devm_mfd_add_devices()` is called, the MFD core takes ownership of the `mfd_cell` array. The core creates `platform_device` instances and assigns the `mfd_cell` data to the `platform_device`'s platform data.
*   **Lifecycle:** The lifetime of the child devices is bound to the parent device. Using `devm_mfd_add_devices()` ensures that `mfd_remove_devices()` is called automatically during the parent's `devres` teardown sequence.
*   **Reference Counting:** When manually creating platform devices, ensure that any associated `device_node` (OF node) has its reference count managed via `of_node_get()` if the node is stored or used beyond the immediate scope of the registration loop.

## 2. Cell Registration, Teardown, and Auto-Indexing
*   **`devm_mfd_add_devices()` vs `mfd_add_devices()` / `mfd_remove_devices()`**:
    *   Prefer `devm_mfd_add_devices()` so registered child platform devices are automatically removed on probe failure or parent unbind.
    *   If using unmanaged `mfd_add_devices()`, any subsequent failure in `probe()` and the parent `.remove()` callback must call `mfd_remove_devices()`.
*   **Cell IDs (`PLATFORM_DEVID_AUTO`)**: Pass `PLATFORM_DEVID_AUTO` (or `PLATFORM_DEVID_NONE` when appropriate) to `mfd_add_devices()` / `devm_mfd_add_devices()` to avoid sysfs platform device name collisions when multiple instances of the MFD chip exist.

## 3. Execution Context, Locking & Synchronization
*   **Context:** MFD registration functions (`mfd_add_devices`, `devm_mfd_add_devices`) perform memory allocations (`kzalloc`) and device registration, which may sleep. **These functions must not be called from atomic context (e.g., hard IRQ handlers or holding a spinlock).**
*   **Serialization:** The MFD core manages internal lists of devices and OF node entries. Callers must ensure that the parent device's probe/remove sequence is not concurrently modified by other threads.
*   **Initialization:** Ensure all synchronization primitives (e.g., `mutex_init`) required by sub-devices are initialized *before* calling `mfd_add_devices()`. Sub-devices may attempt to access these primitives immediately upon probe.

## 4. Return Values, Error Sentinels & Unwind Symmetry
*   **Return Codes:** All registration functions return `0` on success or a negative error code.
*   **Unwind Rules:**
    *   **Partial Failure:** If `mfd_add_devices()` fails, it performs internal cleanup of any devices it successfully registered before the failure. You do not need to manually remove partially registered devices if the call itself returns an error.
    *   **IS_ERR vs NULL:** The MFD core returns integer error codes. Check `if (ret < 0)`.
    *   **Resource Cleanup:** If you perform custom allocations alongside `mfd_add_devices()`, ensure these are managed via `devm_kzalloc()` to maintain a consistent cleanup order.

## 5. High-Frequency Bug Patterns & Review Invariants
*   **Managed vs. Unmanaged:** Do not mix `devm_` managed resources with manual cleanup. If you use `devm_mfd_add_devices()`, do not call `mfd_remove_devices()` in your `remove()` function; this will cause a double-free.
*   **Teardown Ordering:** Ensure the parent device's resources (e.g., regulators, IRQ domains) are not released until *after* `mfd_remove_devices()` has completed.
*   **Registration Timing:** Never expose the `drvdata` of the parent device to sub-devices until *after* `mfd_add_devices()` has returned successfully.
*   **IRQ Flags:** When registering threaded IRQs for MFD children, always use `IRQF_ONESHOT` if no primary handler is provided to prevent interrupt storms.
*   **Hardware State:** Automatic cleanup does *not* reset hardware registers. If your MFD requires a specific "shutdown" register write, this must be performed in the `remove()` callback or via a `devm_add_action()` hook.

## 6. Advanced Semantics & Implementation Details
*   **`mfd_get_cell()`:** This helper is used to retrieve the `mfd_cell` associated with a `platform_device` created by the MFD core. It is safe to call only after the device has been successfully registered.
*   **OF Node Matching:** When using `MFD_CELL_OF`, the core attempts to match the child device to a specific `device_node` in the DT. If `use_of_reg` is set to `true`, the core will use the `of_reg` field to match against the `reg` property in the DT, which is critical for hardware with multiple identical sub-components.
*   **ACPI Matching:** The MFD core supports `mfd_cell_acpi_match` to handle ACPI-based enumeration. It correctly handles `set_primary_fwnode()` to ensure that both ACPI and OF nodes can coexist if necessary, preventing the overwriting of previously populated firmware nodes.
*   **Resource Management:** The MFD core uses `mfd_of_node_entry` to track which OF nodes have been claimed. This prevents multiple child devices from accidentally binding to the same DT node.

## 7. Summary Checklist for Reviewers
1.  **Is `devm_mfd_add_devices` used?** If not, is `mfd_remove_devices` present in the `remove` path?
2.  **Are IDs correct?** Does the cell use `PLATFORM_DEVID_AUTO` to prevent sysfs name collisions?
3.  **Is the IRQ domain handled?** If `irq_domain` is passed, is it valid and persistent for the lifetime of the children?
4.  **Are there race conditions?** Is the parent `drvdata` set *after* registration?
5.  **Are resources managed?** Are all allocations `devm_` managed to avoid leaks on probe failure?
6.  **Is the teardown sequence correct?** Does the parent outlive the children?
7.  **Are synchronization primitives initialized?** Are mutexes/spinlocks ready before `mfd_add_devices`?
8.  **Is `mfd_get_cell` used correctly?** Ensure it is only called on devices created by the MFD core.
9.  **Are OF nodes handled?** If using `MFD_CELL_OF`, ensure the `of_compatible` string matches the device tree binding.
10. **Is `use_of_reg` set?** If the hardware requires matching via `reg` property in DT, ensure `use_of_reg` is true and `of_reg` is populated.
