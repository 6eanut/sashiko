# NVMe Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for the NVMe subsystem (`drivers/nvme/`, `include/linux/nvme.h`). It is intended for automated patch review and architectural validation.

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **Core Objects:**
    *   `struct nvme_ctrl`: The root controller object. Lifecycle managed via `kref`. Transitions are governed by `nvme_ctrl_state`.
    *   `struct nvme_ns`: Namespace object, linked to `nvme_ctrl`. Must be destroyed before the controller.
    *   `struct nvme_queue`: Represents an I/O submission/completion queue pair.
*   **Ownership & Teardown:**
    *   `nvme_ctrl` objects are reference-counted. The `kref` must be held during any asynchronous operation (e.g., `nvme_scan_work`).
    *   **Invariant:** A controller must be quiesced (`nvme_quiesce_io_queues`) and all pending work cancelled/flushed before the final `kref_put` triggers `nvme_free_ctrl`.
    *   **Bug Pattern:** Failure to call `cancel_work_sync()` on controller-associated work leads to UAF when the workqueue fires after `nvme_free_ctrl`.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **State Machine Serialization:**
    *   `nvme_change_ctrl_state()` is the atomic gatekeeper for controller state transitions. It uses `spin_lock_irq(&ctrl->lock)`.
    *   **Rule:** Never perform blocking operations (e.g., `mutex_lock`, `msleep`) inside the `ctrl->lock` critical section.
*   **Lock Hierarchy:**
    *   `ctrl->lock` (spin) -> `ctrl->scan_lock` (mutex) -> `device_lock` (bus/driver core).
    *   **Deadlock Prevention:** Do not acquire `ctrl->lock` while holding `device_lock` if the path can be inverted by controller removal paths.
*   **RCU Usage:**
    *   `nvme_ns` lists are protected by RCU. Use `list_for_each_entry_rcu` for iteration.
    *   **Invariant:** When removing a namespace, `synchronize_rcu()` must be called after removing the namespace from the list and before freeing the `nvme_ns` object.

## 3. Initialization, Registration & Error Unwind

*   **Publication Boundary:**
    *   A controller is "published" to the block layer via `nvme_init_ctrl` and `nvme_start_ctrl`.
    *   **Rule:** Do not expose `nvme_ctrl` to `sysfs` or `configfs` until all mandatory fields (e.g., `ctrl->ops`, `ctrl->device`) are fully initialized.
*   **Symmetric Unwind:**
    *   Every controller allocation must have a corresponding `nvme_put_ctrl`.
    *   **Bug Pattern:** Partial initialization often leaks resources. Use `goto` labels that mirror the allocation order in reverse.
    *   **IS_ERR vs NULL:** `nvme_find_get_ns` returns `NULL` if not found. Always check `IS_ERR()` before dereferencing functions returning pointers to controller structures.

## 4. Asynchronous Work, Interrupts & Teardown Quiescing

*   **Quiescing Protocol:**
    *   Before teardown or reset, call `nvme_quiesce_io_queues()`. This prevents new requests from being queued while allowing existing ones to complete or time out.
    *   **Invariant:** Hardware interrupts must be disabled (`pci_disable_msi` or equivalent) *after* the queues are quiesced to prevent race conditions where an interrupt handler attempts to process a queue that is being torn down.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **Integer/Bounds Integrity:**
    *   **Pattern:** `NVME_NSID_ALL` (0xffffffff) is a special value. Ensure logic handling `nsid` does not treat it as a valid index for array access.
    *   **Guard:** All `nsid` inputs from `ioctl` or `configfs` must be validated against `ctrl->nn` (number of namespaces).
*   **DMA/Memory Safety:**
    *   **Pattern:** Uninitialized DMA buffers.
    *   **Guard:** Always use `dma_alloc_coherent` or `kzalloc` for command buffers. Never assume memory is zeroed if reused.
*   **Format Specifiers:**
    *   **Guard:** Namespace IDs and Controller IDs are `u32`. Always use `%u` or `%x` in `dev_info`/`dev_err` logs. Never use `%d`.

## 6. Hardware Interfacing & DMA

*   **Alignment:**
    *   Admin queues must be page-aligned. Use `dma_alloc_coherent` to ensure alignment requirements of the NVMe specification (e.g., `NVME_ADM_SQES` size constraints).
*   **Register Access:**
    *   Use `readl`/`writel` for register access. Ensure `wmb()` (write memory barrier) is called before triggering the doorbell register to ensure the command is visible in memory.
