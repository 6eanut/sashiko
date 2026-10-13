# Input Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for the Linux Input Subsystem. It is intended for automated patch review and architectural validation.

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **Core Structures**: The primary object is `struct input_dev` (`include/linux/input.h`). It encapsulates device state, event capabilities (`evbit`, `keybit`, etc.), and synchronization primitives.
*   **Ownership & Lifecycle**: 
    *   `input_dev` is reference-counted via `struct device`. 
    *   **Registration**: `input_register_device()` publishes the device. It must be fully initialized (bitmaps, `name`, `id`) before calling this.
    *   **Teardown**: `input_unregister_device()` is the mandatory teardown entry point. It disconnects handlers and ensures no further synchronous callbacks (`open`, `close`, `event`) occur.
*   **Handler/Device Decoupling**: Handlers (e.g., `evdev`) attach to `input_dev`. The core uses `input_dev->mutex` to serialize `open()` and `close()` calls.
*   **Empirical Guard**: Never assume `input_dev` is valid after `input_unregister_device()` returns if the driver does not hold an explicit reference. Use `input_get_device()`/`input_put_device()` if the driver needs to extend the object lifetime beyond the unregistration sequence.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **`input_dev->mutex` Serialization**: The input core serializes `input_dev->open()` and `input_dev->close()` under `input_dev->mutex`. `input_unregister_device()` automatically invokes `close()` if the device is currently open.
*   **Synchronous Handler Disconnection**: During `input_unregister_device()`, the input core immediately disconnects all attached handlers (e.g., setting `evdev->exist = false` under mutex) and drains in-flight synchronous handler calls before returning. Synchronous callbacks (`open`, `close`, `event`) cannot be invoked after unregistration completes.
*   **`event_lock` (Spinlock)**: Protects the event queue and state bitmaps.
    *   **Constraint**: Methods like `setkeycode` are called under `event_lock` and **must not sleep**.
*   **Interrupt Context**: Input drivers often operate in hard IRQ or threaded IRQ contexts. 
    *   **Rule**: Any function called from an interrupt handler (e.g., `input_report_key`, `input_report_abs`) must be atomic. Never perform memory allocations (`GFP_KERNEL`) or blocking operations in these paths.
*   **Deadlock Prevention**: The Input core maintains a strict hierarchy. Drivers must not attempt to acquire `input_dev->mutex` inside an event reporting path, as this will cause a circular dependency if the core holds the mutex during a state change.

## 3. Initialization, Registration & Teardown Ordering

*   **Premature Publication**: All `set_bit()` calls and `input_abs_set_max()` calls must precede `input_register_device()`.
*   **Error Unwind**: If `input_register_device()` fails, the driver must ensure all resources (IRQs, workqueues, timers) are cleaned up. 
    *   **Managed Resources**: Prefer `devm_input_allocate_device()` and `devm_add_action_or_reset()` to automate cleanup. Do not mix `devm` with manual `kfree()` or `input_free_device()`.
*   **Asynchronous Work**: Handler disconnection does *not* stop driver-managed asynchronous work (e.g., custom workqueues for haptics). Drivers must synchronously cancel their own workqueues/timers in `close()` or `remove()` *before* `input_unregister_device()`. Failure to do so leads to UAF if the work/timer fires after the device is freed.

## 4. Error Handling, Resource Recovery & State Validation

*   **IS_ERR vs NULL**: The Input subsystem uses `ERR_PTR` for many allocation returns. Always use `IS_ERR()` and `PTR_ERR()` to validate pointers returned from core functions.
*   **Probe Deferral**: If a dependency (e.g., a regulator or GPIO) is missing, return `-EPROBE_DEFER`. Do not suppress this error code.
*   **Bounds Validation**: 
    *   **Input Reports**: Always validate `input_event` values against `absinfo` limits.
    *   **User-space Buffers**: When implementing custom `ioctl` or `write` handlers, validate buffer lengths against `sizeof(struct input_event)` or the specific `ff_effect` size. Use `input_event_size()` to handle 32/64-bit compatibility.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **Uncancelled Work/Timers**: 
    *   *Pattern*: Timer re-arms itself in the callback.
    *   *Guard*: Ensure the teardown sequence sets a "dying" flag (e.g., `dev->going_away`) and checks it inside the timer/work callback before re-arming.
*   **Information Leaks**: 
    *   *Pattern*: Copying structs from userspace without zero-initialization.
    *   *Guard*: Always use `memset(..., 0, ...)` or stack-initialize structs before copying from `__user` memory.
*   **Integer Overflows**:
    *   *Pattern*: Scaling force-feedback magnitudes using 32-bit math.
    *   *Guard*: Use `u64` for intermediate calculations in `input_ff_upload` or scaling logic.
*   **Sysfs Race Conditions**:
    *   *Pattern*: Exposing sysfs attributes before the device is fully initialized.
    *   *Guard*: Use `input_register_device()` as the final step in the probe sequence. Do not create sysfs groups manually if the input core can handle them via `dev_groups`.

## 6. Force Feedback (FF) Specifics

*   **Ownership**: `input_ff_upload()` and `input_ff_erase()` require a `struct file *` to verify ownership. 
*   **Compatibility**: Use `input_ff_effect_from_user()` to handle `compat_uptr_t` and structure padding differences between 32-bit and 64-bit userspace.
*   **State Invariants**: `input_ff_flush()` must be called during `evdev_release()` to ensure that effects owned by a closing file descriptor are purged, preventing stale effects from triggering on the hardware.
