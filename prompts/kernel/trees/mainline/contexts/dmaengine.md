# DMA Engine Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides architectural and safety invariants for the Linux DMA Engine subsystem (`drivers/dma/`, `include/linux/dmaengine.h`).

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **Core Objects:**
    *   `struct dma_device`: Represents the DMA controller hardware.
    *   `struct dma_chan`: Represents a single DMA channel.
    *   `struct dma_async_tx_descriptor`: The unit of work submitted to a channel.
*   **Ownership & Lifetime:**
    *   `dma_device` is embedded in the driver-specific engine structure. Its lifetime is tied to the `device` probe/remove cycle.
    *   `dma_chan` objects are managed by the DMA engine core. Drivers must not free these manually; they are cleaned up during `dma_async_device_unregister()`.
*   **Publication:** A `dma_device` is visible to the core only after `dma_async_device_register()` returns successfully. Do not expose channels to clients before the hardware is fully initialized and the `channels` list is populated.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Locking Hierarchy:**
    *   Virtual channel infrastructure uses a spinlock to protect descriptor lists (`desc_submitted`, `desc_issued`, `desc_completed`).
    *   **Rule:** Never perform blocking operations (e.g., `msleep`, `mutex_lock`) inside callbacks or interrupt handlers.
*   **Atomic vs. Process Context:**
    *   `dma_cookie_assign()` and `dma_cookie_complete()` require the caller to hold a spinlock.
    *   `dma_cookie_status()` is lockless and safe to call from any context.
*   **Interrupt Serialization:**
    *   Hardware interrupt handlers must use `spin_lock_irqsave()` if they share state with process-context code (e.g., `device_terminate_all`).
    *   **Bug Guard:** Ensure `dmaengine_terminate_sync()` is used when teardown requires waiting for hardware to quiesce; it is a blocking call and must not be called from atomic context.

## 3. Initialization, Registration & Teardown Ordering

*   **Symmetry:** Every `dma_async_device_register()` must have a corresponding `dma_async_device_unregister()` in the `remove` path.
*   **Error Unwind:**
    *   Use `devm_` managed resources where possible (e.g., `devm_request_irq`, `devm_ioremap`).
    *   **Critical:** If manual allocation is used, the error path must strictly reverse the order of allocation.
*   **Hardware State:**
    *   Always reset the DMA controller to a known state during `probe` and `remove`. Leaving descriptors in a "running" state in hardware can cause memory corruption if the kernel reboots or kexecs.

## 4. Error Handling, Resource Recovery & Boundary Validation

*   **IS_ERR vs NULL:**
    *   DMA engine functions returning pointers (e.g., `dma_request_chan`) return `ERR_PTR()` on failure. Use `IS_ERR()` and `PTR_ERR()`.
*   **Scatter-Gather (SGL) Safety:**
    *   **Bug Pattern:** Using `sg_dma_len(sg)` without verifying the `sg` pointer is valid.
    *   **Arithmetic:** When calculating DMA lengths, use `size_t` or `u64` to prevent 32-bit integer overflows.
*   **Boundary Checks:**
    *   Validate that `dma_addr_t` returned by `dma_map_*` is valid. If `dma_mapping_error()` returns true, the operation must be aborted immediately.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **The "Atomic Sleep" Guard:**
    *   *Pattern:* Calling `dmaengine_terminate_sync()` inside a `spin_lock` region.
    *   *Fix:* Use `dmaengine_terminate_all()` (non-blocking) if you cannot sleep, or drop the lock before calling the sync version.
*   **The "Scatterlist Length" Guard:**
    *   *Pattern:* Using the length of the first entry for all entries in a loop.
    *   *Fix:* Always iterate using `for_each_sg()` and access `sg_dma_len(sg)` for the current entry.

## 6. Hardware/Protocol Invariants (v7.3-rc5 Specific)

*   **eDMA (Freescale/NXP) Specifics:**
    *   `EDMA_TCD_CSR_DONE` must be checked to confirm completion.
    *   `EDMA_V3_CH_ES_ERR` indicates a bus error; the driver must clear this bit in the Error Status register to allow the channel to be reused.
*   **Cookie Management:**
    *   `dma_cookie_t` is an opaque type. Do not perform arithmetic on it. Use `dma_submit_error()` to check for failure.
    *   `dma_cookie_complete()` must be called exactly once per descriptor completion. Double completion will corrupt the channel's cookie tracking.
