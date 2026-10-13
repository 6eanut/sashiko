# I2C Subsystem API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for the Linux I2C subsystem. It is intended for developers and automated review systems to ensure API compliance, memory safety, and synchronization correctness.

---

## 1. Core API Overview & Ownership
*   **Ownership:** The `struct i2c_client` represents a device on the bus. The lifecycle is managed by the I2C core; drivers must not manually free `i2c_client` structures.
*   **Reference Counting:** When a subsystem lookup (e.g., `i2c_verify_client`) yields a pointer, the caller does not assume ownership of a new reference.
*   **Initialization Failure:** If a driver's `probe` function fails, it must ensure all resources (clocks, regulators, IRQs) are released. Use `devm_*` managed resources to automate this and prevent leaks.
*   **Asynchronous Handoff:** I2C transfers are synchronous. If scheduling work, the caller must ensure the `i2c_client` remains valid (e.g., by holding a reference to the parent device).

## 2. Execution Context, Preemption, and Sleeping
*   **Sleeping Constraints:** `i2c_transfer`, `i2c_master_send`, and `i2c_master_recv` are **blocking** operations. They **must not** be called from atomic context (e.g., hard IRQ handlers, `spin_lock` critical sections, or `preempt_disable` regions).
*   **Atomic Context:** If an I2C operation is required in an atomic context, defer the work to a `workqueue` or a threaded interrupt handler.
*   **Memory Allocation:** Since I2C transfers can sleep, memory allocations within the transfer path must use `GFP_KERNEL`.

## 3. Return Values and Error Sentinels
*   **Transfer Results:** `i2c_transfer`, `i2c_master_send`, and `i2c_master_recv` return:
    *   **Positive:** The number of bytes transferred (on success).
    *   **Negative:** A standard Linux error code (e.g., `-EIO`, `-ETIMEDOUT`, `-ENXIO`).
*   **Error Handling:** Always check `if (ret < 0)` explicitly. Never treat a negative return value as a boolean "false" or a byte count.
*   **Integer Truncation:** When assigning error codes (e.g., `int ret = i2c_master_send(...)`), ensure the variable is a signed `int`. Assigning to `u8` or `u16` will truncate negative error codes, turning them into positive success values.

## 4. Concurrency and Serialization
*   **Serialization:** The I2C core serializes access to the `i2c_adapter` via internal locks. Drivers do not need to implement additional locking for standard `i2c_transfer` calls.
*   **Register Access:** When using `regmap` with I2C, ensure the `regmap` configuration matches the hardware's capabilities.
*   **Register Defaults:** All `regmap` default tables **must** be sorted by address to ensure binary search compatibility.

## 5. DMA Bounce Buffer APIs
To prevent DMA-related memory corruption, the I2C core provides specific helpers for DMA-capable adapters.

### Client Transfer Buffer DMA Rules
*   **Ordinary Transfers (`i2c_transfer`, `i2c_master_send`, `i2c_master_recv`, `i2c_smbus_*`)**:
    *   Safe to call with stack-allocated, struct-embedded, or vmalloc buffers.
    *   The I2C core bounce-buffers any message that lacks `I2C_M_DMA_SAFE`.
*   **DMA-Safe Transfers (`i2c_master_send_dmasafe`, `i2c_master_recv_dmasafe`, `I2C_M_DMA_SAFE`)**:
    *   Assert that the buffer is already DMA-safe (`kmalloc`/`kzalloc`, cacheline-aligned, not on the stack or in vmalloc).
    *   Passing a stack, vmalloc, or non-cacheline-aligned buffer to `*_dmasafe()` or setting `I2C_M_DMA_SAFE` on it bypasses bounce buffering and is a bug.

### Adapter DMA Bounce Helpers
*   I2C controller (`struct i2c_algorithm`) `master_xfer` implementations that perform DMA must call `i2c_get_dma_safe_msg_buf(msg, threshold)` before mapping a message buffer for DMA (`NULL` return indicates the driver should fall back to PIO).
*   On transaction completion or failure, the adapter driver must call `i2c_put_dma_safe_msg_buf(buf, msg, xferred)` across all paths to copy received data back and free any temporary bounce buffer.

## 6. High-Frequency Bug Patterns & Review Invariants
*   **Unsorted Register Tables:** Verify that `reg_default` arrays are sorted by address.
*   **NULL Pointer Dereferences:** Always check if optional callbacks are non-NULL before invoking them.
*   **State Invariants:** Ensure hardware is fully configured (e.g., page selection, power state) before triggering commands.
*   **Reset/Cache Sync:** After a device reset, ensure the register cache is invalidated or synchronized to prevent the driver from using stale state.
*   **Kconfig Dependencies:** Ensure `select REGMAP_I2C` or relevant dependencies are present in `Kconfig` to avoid link-time errors.
