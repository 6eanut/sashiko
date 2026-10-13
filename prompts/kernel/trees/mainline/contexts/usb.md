# Linux v7.3-rc5 USB Subsystem Code Context Guide

This guide defines architectural and safety invariants for the Linux USB subsystem (v7.3-rc5) for automated patch review and static analysis.

---

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

### Core Hierarchy
The USB subsystem follows a strict tree structure: `usb_device` → `usb_configuration` → `usb_interface` → `usb_host_endpoint`.
*   **`struct usb_device`**: Root object representing the physical device. Managed via `usb_get_dev()` and `usb_put_dev()`.
*   **`struct usb_interface`**: Represents a specific function of the device. Drivers bind to these.
*   **`struct urb` (USB Request Block)**: Fundamental unit of I/O. Must be initialized via `usb_init_urb()` and submitted via `usb_submit_urb()`.

### Object Lifetimes
*   **Reference Counting**: Drivers must use `kref` to manage private data. The `usb_interface` lifecycle is managed by the USB core; drivers must not access `usb_interface` members after `disconnect()` returns.
*   **Teardown**: Upon `disconnect()`, drivers **must** call `usb_kill_urb()` or `usb_unlink_urb()` for all active URBs. Failure to do so causes use-after-free when the HCD completes an URB for a freed buffer.

---

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

### Lock Hierarchy
1.  **`io_mutex` (Driver-level)**: Serializes `open()`, `release()`, and `disconnect()`.
2.  **`limit_sem`**: Throttles concurrent I/O submissions (e.g., `WRITES_IN_FLIGHT`).
3.  **`usb_anchor`**: Preferred mechanism for tracking URBs. `usb_anchor_urb()` and `usb_unanchor_urb()` are atomic-safe.

### Concurrency Rules
*   **Atomic Context**: `usb_submit_urb()` accepts `GFP_ATOMIC` for atomic context. `usb_kill_urb()` is synchronous and **must not** be called from atomic context if it requires waiting for completion.
*   **Disconnect Serialization**: The `disconnected` flag must be checked under `io_mutex` before any I/O submission.
*   **Deadlock Prevention**: Do not hold a mutex while calling `usb_kill_urb()` if the associated URB completion handler attempts to acquire the same mutex.

---

## 3. Initialization, Registration & Teardown Ordering

*   **Probe/Disconnect Symmetry**: Every resource allocated in `probe()` (e.g., `kmalloc`, `usb_alloc_urb`) must be freed in `disconnect()`.
*   **Premature Publication**: Do not expose the device to user-space (e.g., `usb_register_dev()`) until all internal structures (buffers, mutexes, anchors) are fully initialized.
*   **Error Unwind**: Use the "goto" pattern for error unwinding. Ensure `usb_put_dev()` is called if `usb_get_dev()` succeeded, even on failure paths.

---

## 4. Error Handling, Resource Recovery & Boundary Validation

*   **IS_ERR vs NULL**: `usb_get_intf()` returns `NULL` or valid pointers. Always check `IS_ERR()` for functions returning error pointers (e.g., `usb_alloc_urb` returns `NULL` on failure).
*   **Input Validation**:
    *   **Buffer Sizes**: `MAX_TRANSFER` (e.g., `PAGE_SIZE - 512`) must be strictly enforced.
    *   **User-Space**: All `ioctl` parameters must be validated against `MAX_TRANSFER` and device-specific limits before being used as lengths for `usb_bulk_msg` or `usb_submit_urb`.
*   **DMA Safety**: Buffers passed to URBs must be DMA-capable. Use `usb_alloc_coherent()` for buffers accessed by hardware to avoid cache coherency issues.

---

## 5. High-Frequency Bug Patterns & False-Positive Guards

### Empirical Bug Patterns
1.  **Unbalanced Refcount**: Missing `usb_put_dev()` on error paths in `probe()`.
2.  **Use-After-Free (Teardown)**: Accessing private data after `usb_deregister_dev()` or during `disconnect()`. *Guard*: Check `disconnected` bit under `io_mutex`.
3.  **Sleeping in Atomic**: Calling `mutex_lock()` inside an URB completion handler. *Guard*: Completion handlers run in atomic context; use `spin_lock()` or `usb_anchor` logic.
4.  **Stale URBs**: Failing to call `usb_kill_urb()` before freeing the buffer associated with the URB.
5.  **Integer Overflow**: Calculating transfer sizes without checking for overflow when adding headers/padding.

### False-Positive Guards
*   **`usb_kill_urb`**: `usb_kill_urb` returns `0` (success) or `-ENOENT` (already dead). Both are valid outcomes in teardown.
*   **`usb_anchor`**: If an URB is anchored, `usb_kill_anchored_urbs()` is the mandatory teardown method. Do not manually track individual URBs if they can be anchored.

---

## 6. Power Management (Runtime PM)

*   **Autosuspend**: Drivers must call `usb_autopm_get_interface()` before I/O and `usb_autopm_put_interface()` after.
*   **Resume/Suspend**: Do not call `usb_set_interface()` while the device is suspended without first waking it.
*   **Balance**: Every `get` must have a corresponding `put`, including on error paths. Use `usb_autopm_put_interface_async()` if the context requires non-blocking behavior.
