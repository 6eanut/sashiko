# Linux SCSI Subsystem Code Context Guide (v7.3-rc5)

This guide provides authoritative technical invariants for the Linux SCSI subsystem. It is designed to prevent common bug patterns identified in the last 100,000 kernel commits.

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **Core Objects:** The subsystem revolves around `Scsi_Host` (the controller), `scsi_device` (the logical unit), and `scsi_cmnd` (the I/O request).
*   **Ownership & Lifecycle:**
    *   `Scsi_Host` is the root of the device hierarchy. It must be initialized via `scsi_host_alloc()` and registered via `scsi_add_host()`.
    *   **Reference Counting:** Every `scsi_device` holds a reference to its parent `Scsi_Host`. Use `scsi_device_get()` and `scsi_device_put()` to manage lifetimes.
    *   **Teardown:** During `remove` or error paths, `scsi_remove_host()` must be called to quiesce the host before `scsi_host_put()`. Failure to do so leads to use-after-free when the block layer attempts to access a defunct host.
*   **Invariants:**
    *   Never access `scsi_cmnd` fields after calling `scsi_done()`. The command ownership transfers back to the mid-layer immediately upon invocation of the completion callback.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Locking Hierarchy:**
    *   The `Scsi_Host` lock (`host_lock`) is the primary synchronization point for command submission.
    *   **Atomic Contexts:** `scsi_done` callbacks are frequently invoked in interrupt context. Never perform `GFP_KERNEL` allocations or sleep-inducing operations (e.g., `mutex_lock`) within the completion path. Use `GFP_ATOMIC` if memory allocation is unavoidable.
*   **Concurrency Guards:**
    *   When modifying `scsi_device` state (e.g., `SDEV_OFFLINE`), always hold the host-level lock to prevent races with I/O submission threads.
    *   **Workqueues:** For asynchronous teardown (e.g., `fcoe_fcf_device`), ensure `cancel_work_sync()` or `flush_workqueue()` is called before freeing the parent structure to prevent use-after-free in the worker thread.

## 3. Initialization, Registration & Error Unwind

*   **Symmetry:** Every `scsi_add_host()` must be paired with `scsi_remove_host()`.
*   **Error Paths:**
    *   If `scsi_host_alloc()` succeeds but subsequent registration fails, you must call `scsi_host_put()` to release the initial reference.
    *   **Resource Tracking:** Use `devm_*` managed resources where possible for hardware registers and IRQs. If the driver requires explicit teardown ordering (e.g., stopping DMA before freeing buffers), use manual cleanup instead of `devm_`.
*   **Publication:** A `Scsi_Host` is visible to user-space (via sysfs) as soon as `scsi_add_host()` returns. Ensure all internal state (queues, DMA mappings) is fully initialized before this call.

## 4. Asynchronous Work, Interrupts & Teardown Quiescing

*   **Interrupts:**
    *   Always check the status register (e.g., `TWL_HISTAT` in 3w-sas) before processing an interrupt to avoid infinite loops or processing stale data.
*   **Quiescing:** Before removing a device, the command queue must be drained. Use `scsi_device_quiesce()` to prevent new commands from entering the mid-layer while the driver is tearing down hardware queues.

## 5. Untrusted Input, Sizing & Buffer Validation

*   **Sense Buffers:** Hardware-provided sense buffers are a common source of OOB access. Always clamp the length field provided by the device to the actual size of the allocated buffer:
    ```c
    size_t len = min_t(size_t, hw_reported_len, sizeof(cmd->sense_buffer));
    memcpy(cmd->sense_buffer, hw_buffer, len);
    ```
*   **Integer Overflows:** When calculating DMA scatter-gather list sizes, use `check_mul_overflow()` to prevent integer wrapping.
*   **TOCTOU:** Copy all user-provided parameters (e.g., `iscsi_uevent` structures) to a local kernel-space buffer before validation. Never validate and use a field directly from a user-mapped pointer.

## 6. High-Frequency Bug Patterns & False-Positive Guards

*   **IS_ERR vs NULL:** Many SCSI helper functions return `ERR_PTR(-ENOMEM)` on failure. Always use `IS_ERR()` and `PTR_ERR()` rather than checking for `NULL`.
*   **Uninitialized Variables:** Ensure all fields in `scsi_cmnd` or private driver structures are zeroed (e.g., `kzalloc`) before use.
*   **DMA Mapping:** Always verify the return of `dma_map_single()` or `dma_map_sg()` using `dma_mapping_error()`.
*   **False-Positive Guard:** If a static analysis tool flags a "potential NULL dereference" on a `scsi_device` pointer, verify if the pointer is protected by a `scsi_device_get()` reference. If the reference is held, the pointer is stable.

## 7. Domain-Specific Invariants (UFS/FCoE)

*   **FCoE (`fc_frame`):**
    *   `fc_frame_alloc()` must be used for frame allocation.
    *   The `skb` inside `fc_frame` must be handled with standard network stack care; ensure `fr_len` is updated before passing the frame to the transport layer.
