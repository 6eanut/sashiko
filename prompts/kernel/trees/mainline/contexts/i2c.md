# I2C Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for the Linux I2C subsystem. Developers must adhere to these rules to prevent common concurrency, lifecycle, and hardware-interfacing bugs.

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **Core Objects:** The subsystem revolves around `struct i2c_adapter` (the controller/bus) and `struct i2c_client` (the device on the bus).
*   **Lifecycle Management:**
    *   `i2c_client` objects are managed by the I2C core. Drivers must not manually free `i2c_client` structures.
    *   **Debugfs:** Entries created under `client->debugfs` are automatically removed by the I2C core during device unbind (after the driver's `.remove()` callback returns and before devres release) and after `.probe()` failure. Do not manually remove these.
    *   **Teardown Ordering:** Ensure all sysfs attributes, timers, and work items are canceled *before* the driver's `.remove()` callback completes. Use `devm_*` managed resources to ensure symmetric unwinding.
*   **Ownership:** The `i2c_adapter` owns the bus state. If an adapter fails to register, all associated resources must be cleaned up to prevent dangling pointers in the `i2c_bus_type` lists.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Bus Locking:**
    *   `i2c_adapter` uses `struct rt_mutex bus_lock` to serialize access.
    *   **Atomic Transfers:** `i2c_in_atomic_xfer_mode()` is strictly reserved for late-stage system operations (e.g., PMIC power-down). Do not use atomic transfers for generic I/O.
    *   **Deadlock Prevention:** Never hold a subsystem-specific lock (e.g., a clock or regulator lock) while calling `i2c_transfer()`, as this causes AB/BA deadlocks with the I2C core's `bus_lock`.
*   **Interrupt Context:**
    *   I2C transfers are generally blocking. If an interrupt handler requires I2C access, it must use the atomic transfer path (if supported by the controller) or defer the work to a workqueue.
*   **Shared Pointers:** Access to `i2c_adapter` or `i2c_client` pointers in concurrent paths must be protected by the device's reference count or the adapter's `bus_lock`.

## 3. Initialization, Registration & Error-Path Unwind

*   **Probe Deferral:** If a required resource (GPIO, regulator, clock) is not yet available, return `-EPROBE_DEFER`. Do not poll within the probe function.
*   **Symmetric Unwind:** Every `i2c_add_adapter()` or `i2c_register_board_info()` must have a corresponding cleanup path.
*   **Error Handling:**
    *   **IS_ERR vs NULL:** Always use `IS_ERR()` to check return values from `i2c_new_client_device()`. Never treat a negative error code as a boolean `false`.
    *   **Partial Initialization:** If a probe fails, ensure all previously allocated resources (including those not managed by `devm`) are explicitly freed.
*   **Publication:** A device is "published" to the system once `device_register()` is called. Ensure all internal state is fully initialized before this call.

## 4. Hardware Interfacing, DMA & Power Management

*   **DMA Safety:** Use `i2c_master_send_dmasafe()` and `i2c_master_recv_dmasafe()` when the buffer is not guaranteed to be in cache-coherent memory.
*   **Timing & Frequency:**
    *   Do not hardcode timing parameters. Use `device_property_read_u32()` to fetch `clock-frequency` from the Device Tree.
    *   Validate frequency modes against `I2C_MAX_*_MODE_FREQ` constants defined in `include/linux/i2c.h`.
*   **Power Management:**
    *   The driver must handle `suspend`/`resume` callbacks to prevent hardware access while the bus is powered down.
    *   Use `pm_runtime_get_sync()` and `pm_runtime_put()` to manage power states. Ensure these are balanced on all error paths.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **The "Boolean Error" Pattern:**
    ```c
    // WRONG
    if (!i2c_master_send(...)) // Treats error as success
    // RIGHT
    ret = i2c_master_send(...);
    if (ret < 0) // Handle error
    ```
*   **The "UAF on Teardown" Pattern:** Ensure that any `timer_setup()` or `INIT_WORK()` items are explicitly canceled before the driver's `remove` function returns.
*   **The "Hardcoded Index" Pattern:** Avoid hardcoded hardware indices. Always resolve resources via Device Tree-based lookups to ensure compatibility across SoC variants.
*   **The "Bus-Busy" Pattern:** After a timeout, the controller may be left in a "busy" state. Implement a recovery sequence (e.g., toggling SCL) to clear the bus before returning an error.
