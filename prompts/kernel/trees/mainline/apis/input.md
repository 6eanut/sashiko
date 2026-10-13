# Input Device and Force-Feedback APIs (`include/linux/input.h`)

## 1. Allocation, Registration, and `devm` Lifecycle

*   **Allocation**: Use `input_allocate_device()` for manual lifecycle management or `devm_input_allocate_device(dev)` for automatic cleanup. `devm_input_allocate_device(dev)` automatically sets `input_dev->dev.parent = dev`.
*   **Registration (`input_register_device`)**: There is no `devm_input_register_device()`; both managed and unmanaged input devices use `input_register_device()`.
    *   **Invariant**: All driver private data, callbacks, and `input_set_drvdata()` must be initialized **before** `input_register_device()`, as `open()` or userspace ioctls can be invoked immediately upon registration.
    *   **Core Behavior**: The core automatically sets `EV_SYN` / `SYN_REPORT`.
*   **Teardown (`input_free_device` vs `input_unregister_device`)**:
    *   **Unmanaged**: Call `input_unregister_device()` followed by `input_free_device()`.
    *   **Managed (`devm`)**: Calling `input_unregister_device()` is sufficient; the `devres` framework handles the final `input_free_device()`.
    *   **Safety**: `input_free_device(NULL)` is a safe no-op. Never call `input_free_device()` on a successfully registered `input_dev` (unless the driver unregisters it and sets `dev = NULL` first).
    *   **Tracking**: Calling `input_free_device()` (before registration) or `input_unregister_device()` (after registration) on a `devm_input_allocate_device()` pointer is safe and removes the devres tracking synchronously without double-free or double-unregister.

## 2. Event Reporting, Locking, and Synchronization

*   **`input_event()` Before Registration**: Safe to call after allocation and before `input_register_device()`; it updates internal state (`keybit`, abs values) without propagating to handlers.
*   **Typed Helpers**: Prefer `input_report_key()`, `input_report_abs()`, and `input_report_rel()` over raw `input_event()`.
*   **Synchronization**: Finish every logical event packet with `input_sync()`.
*   **Locking**: `input_event()` and `input_report_*()` acquire `input_dev->event_lock` (spinlock, disabling local interrupts). Do not hold locks that would invert lock ordering against `event_lock`.
*   **Active State Check (`input_device_enabled`)**: Must be called while holding `input_dev->mutex`.
*   **Serialization**: `input_mutex` protects `input_dev_list` and `input_handler_list`.

## 3. Force-Feedback (`input_ff_create_memless`)

*   `input_ff_create_memless(dev, data, play_effect)` takes ownership of `data` and frees it via `kfree()` when the input device is destroyed.
*   **Constraint**: If `data` is non-NULL, it **must** be allocated with `kmalloc()`/`kzalloc()` (never `devm_kzalloc()`) and **must not** be manually freed by the driver after `input_ff_create_memless()` succeeds.

## 4. Concurrency, Asynchronous Work, and Teardown

*   **Asynchronous Work**: If the driver uses `work_struct` or `timer_list` to report events, these **must** be cancelled synchronously (e.g., `cancel_work_sync()`, `timer_delete_sync()`) before `input_unregister_device()` returns to prevent use-after-free.
*   **Registration Failure**: If `input_register_device()` fails, the driver must ensure all allocated resources (e.g., IRQs, memory) are freed.
*   **State Sanitization**: Ensure the state machine handles "device disconnected" or "firmware reset" events by clearing all active keys/touches.
*   **Context Constraints**: The `setkeycode` method is called while holding `event_lock` and thus **must not sleep**. Ensure all paths leading to event reporting are atomic-safe.

## 5. Sanitization, Validation, and Data Integrity

*   **Validation**: Validate all user-supplied lengths against destination buffer sizes. Check for integer overflows when calculating offsets for `input_absinfo` or keycode tables.
*   **Endianness**: Use `le16_to_cpu()` or `be16_to_cpu()` when parsing raw hardware data to ensure cross-architecture compatibility.
*   **Confidence**: Always check hardware-provided "Confidence" or "Validity" flags before reporting touch data to prevent "ghost" inputs.
*   **Pointer Safety**: Always validate pointers obtained from external device input (e.g., HID reports) before dereferencing to prevent NULL pointer panics.
*   **Array Termination**: Ensure all static capability arrays are properly terminated with a sentinel value to prevent out-of-bounds reads during iteration.
*   **Stateful Filtering**: Avoid stateful filtering of input events; forward all events to the input stack unless explicitly required by the hardware protocol.
*   **Fragmented Packets**: Event processing logic must correctly handle fragmented or multi-part input packets to avoid redundant event reporting.
*   **Hardware Mapping**: Verify hardware-to-software index mappings against the device specification to prevent incorrect data reporting.
*   **Capability Initialization**: Partition capability registration (e.g., `EV_KEY`, `EV_ABS`) correctly between generic HID handlers and device-specific drivers to avoid redundant registration failures.
