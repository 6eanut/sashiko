# Hardware Monitoring (hwmon) API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for the `hwmon` subsystem, ensuring driver stability and compliance with the Linux v7.3-rc5 kernel architecture.

## 1. Registration and Core Serialization
*   **Registration:** Use `hwmon_device_register_with_info()` for manual lifecycle management or `devm_hwmon_device_register_with_info()` for device-managed (automatic) cleanup.
*   **Core Serialization:** The `hwmon` core automatically serializes thermal subsystem and sysfs callbacks for attributes described through the `info` parameter of `hwmon_device_register_with_info()` / `devm_hwmon_device_register_with_info()`.
*   **Custom Locking:** Drivers must provide their own locking for interrupt handlers and for custom attributes registered outside `info` (e.g., via `extra_groups`), using `hwmon_lock(hwmon_dev)` / `hwmon_unlock(hwmon_dev)` or `guard(hwmon_lock)(hwmon_dev)`.
*   **Ownership:** The `hwmon` core takes ownership of the `hwmon_device` structure upon successful registration. Drivers must not manually `kfree()` the `hwmon_device` pointer.
*   **Teardown:** If using `devm_` variants, the core automatically unregisters the device. If using the non-managed `hwmon_device_register_with_info()`, you **must** call `hwmon_device_unregister()` in the driver's `remove` path.
*   **Error Handling:** Both registration functions return an `ERR_PTR()` on failure. **Always** use `IS_ERR()` to validate the return pointer. Never assume a non-NULL pointer implies success.

## 2. Locking and Synchronization
*   **Locking Primitives:** Prefer the `guard(hwmon_lock)(hwdev)` RAII pattern to ensure locks are released on all exit paths, preventing deadlocks and leaks.
*   **Atomic Context:** Sysfs callbacks are generally permitted to sleep. Do not perform blocking operations (e.g., `mutex_lock`) inside interrupt handlers. If hardware access is required from an interrupt, use a `workqueue` to defer the work to a process context.
*   **Data Races:** If using `rcu` for lockless reads, ensure `rcu_read_lock()` is held during the entire duration of the pointer dereference.
*   **Hardware State:** Always verify that the hardware is in a valid state (e.g., clocks enabled, power domains active) before performing register access.

## 3. Return Values and Error Sentinels
*   **Sysfs Store:** Your `store` callbacks **must** return the number of bytes processed (e.g., `return count;`) on success. Returning `0` causes user-space to retry the write indefinitely.
*   **Error Propagation:** If a hardware read/write fails, return the negative error code (e.g., `-EIO`, `-ETIMEDOUT`). Do not return `0` for failure.
*   **Partial Failure:** If a probe function fails after partial initialization, ensure all allocated resources (e.g., `debugfs` entries, timers, workqueues) are explicitly cleaned up. The `devm_` framework does **not** automatically stop active timers or cancel pending work.

## 4. High-Frequency Bug Patterns & Review Invariants
*   **The "Dangling Timer" Pattern:** If your driver uses `timer_setup()` or `INIT_WORK()`, you **must** call `cancel_timer_sync()` or `cancel_work_sync()` in the `remove` path.
*   **The "IS_ERR" Mismatch:**
    ```c
    // WRONG
    hwdev = hwmon_device_register_with_info(...);
    if (!hwdev) return -ENOMEM; // Fails to catch ERR_PTR(-EEXIST)

    // RIGHT
    hwdev = hwmon_device_register_with_info(...);
    if (IS_ERR(hwdev)) return PTR_ERR(hwdev);
    ```
*   **Array Bounds:** When mapping hardware channels to arrays, validate the index against the maximum supported channels to prevent out-of-bounds writes.
*   **Integer Overflows:** Cast to `u64` before performing shifts or multiplications to prevent overflow.
*   **Resource Cleanup:** `debugfs` entries are **not** managed by `devm_`. You must manually remove them in the `remove` function or probe error path.

## 5. Device-Managed Cleanup (devm) Rules
*   **Mixing Managed/Unmanaged:** If you mix `devm_` allocations with manual `kmalloc`, ensure the manual cleanup is performed in the correct order.
*   **Probe Failure:** If `devm_hwmon_device_register_with_info()` fails, the `devm` framework cleans up resources allocated *before* the failure. However, manual hardware configuration (e.g., `i2c_smbus_write_byte`) must be manually rolled back in the error path.
*   **Lifecycle Integrity:** Ensure that all asynchronous tasks (timers/workqueues) are fully quiesced before the `devm` framework triggers the final `kfree` of the driver's private data structure.
*   **Reference Counting:** When a subsystem lookup yields a shared object, the caller must ensure the reference count is managed via the appropriate `get`/`put` helpers if the object lifetime exceeds the current scope.
*   **Asynchronous Handoff:** When passing an object to an asynchronous worker, the caller must ensure the object remains valid for the duration of the task, typically by incrementing a reference count or ensuring the worker is cancelled before the object is destroyed.
*   **Memory Barriers:** When publishing state to shared structures, use `smp_wmb()` or `rcu_assign_pointer()` to ensure that initialization is visible to other CPUs before the object is marked as active.
*   **State Consistency:** Ensure that all hardware-reported sensor types and IDs are explicitly handled or ignored by the driver's enumeration logic to prevent unknown sensor warnings.
*   **Clock Management:** Always ensure the required clock is enabled before performing any hardware register access; failure to do so is a common cause of bus errors.
