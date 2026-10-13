# LED Class and Trigger API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for the Linux LED subsystem. Adherence to these patterns is mandatory for all drivers interacting with `include/linux/leds.h`.

## 1. Lifecycle and Unregistration Ordering
*   **`led_classdev_register()` / `led_classdev_unregister()` vs `devm_led_classdev_register[_ext]()`**:
    *   Unmanaged `led_classdev_register()` exposes sysfs attributes backed by the driver's `struct led_classdev`. In `.remove()` and `.probe()` error paths, `led_classdev_unregister()` **must** be called before freeing the containing driver private structure (`ddata`), otherwise sysfs access races with or follows `kfree(ddata)`.
    *   `devm_led_classdev_register()` and `devm_led_classdev_register_ext()` tie unregistration to devres LIFO release order.
*   **`led_trigger_register()` / `led_trigger_unregister()` vs `devm_led_trigger_register()`**:
    *   Custom LED triggers registered with `led_trigger_register()` are linked into the global `trigger_list` and must be unregistered via `led_trigger_unregister()` on probe failure or module unload (or registered via `devm_led_trigger_register()`).

## 2. Core API Ownership and Reference Counting
*   **Reference Counting:** When using `led_get()` or `devm_of_led_get()`, the caller receives a reference to the `led_classdev`. You **must** release this reference using `led_put()` if the registration was not managed via `devm_`.
*   **Fwnode Handling:** When parsing device tree properties, ensure `fwnode_handle_put()` is called if you manually incremented the reference count of an `fwnode_handle`.

## 3. Calling Context, Locking & Synchronization
*   **Brightness Setting:**
    *   `brightness_set()`: **Must not sleep.** It is called from atomic contexts (e.g., timer interrupts or trigger events).
    *   `brightness_set_blocking()`: Used for hardware requiring bus communication (I2C/SPI). This **can sleep**.
*   **Trigger Locking:** The `led_cdev->trigger_lock` protects the trigger state. If you are implementing a custom trigger, you must hold this lock when calling `led_trigger_set()`.
*   **Atomic Contexts:** Never perform memory allocations with `GFP_KERNEL` inside `brightness_set()`. Use `GFP_ATOMIC` or pre-allocated buffers.

## 4. Return Values & Error Sentinels
*   **Pointer Sentinels:** Functions like `devm_of_led_get()` return `ERR_PTR()` on failure. **Always** use `IS_ERR()` to validate the pointer before dereferencing.
*   **Hardware Communication:** Never ignore the return value of I2C/SPI writes during probe. If hardware communication fails, the probe **must** abort and return the error code to prevent "phantom" device registration.
*   **Partial Failure:** If a driver manages multiple LEDs and one fails, ensure all previously registered LEDs are unregistered before returning the error code to avoid resource leaks.

## 5. Concurrency & Memory Ordering
*   **State Updates:** When updating hardware state, ensure the logic is idempotent.
*   **Race Conditions:** Use `led_cdev->work_flags` and the `trigger_lock` to serialize access. If a "set brightness to 0" work item is pending, ensure it is flushed or cancelled before changing the trigger state.
*   **Synchronous Cancellation:** During `remove()`, ensure all pending `led_cdev->set_brightness_work` items are cancelled via `cancel_work_sync()`.

## 6. Registration & Teardown Invariants
*   **Quiescent State:** Hardware must be initialized in a "safe" or quiescent state before registration. Do not start hardware sequencers or blink patterns until the `led_classdev` is fully registered.
*   **Name Collisions:** If registering multiple LEDs, ensure unique names. Use `LED_REJECT_NAME_CONFLICT` if the subsystem should prevent duplicate sysfs entries.

## 7. Buffer Sizing & Validation
*   **User Input:** Validate all lengths from sysfs or device tree. Never use string functions on user-provided buffers without verifying null-termination.
*   **Bounds Checking:** Indices derived from device tree properties (e.g., `led-names` index) **must** be validated against the actual array size.
*   **Integer Integrity:** Use `unsigned` types for register calculations to prevent signedness mismatches and verify formulas against the datasheet to avoid off-by-one errors.

## 8. Device-Managed Cleanup (devres)
*   **Automatic vs. Manual:** Do not manually free `devm_` allocated resources.
*   **Mixing Rules:** If you mix manual and `devm_` allocations, ensure the manual cleanup (e.g., `kfree`) does not conflict with the `devres` release.
*   **Mutexes:** If you initialize a `mutex` in `probe()`, you **must** destroy it in `remove()` or via `devm_add_action_or_reset()`.

## 9. High-Frequency Bug Patterns (Review Checklist)
*   [ ] **Probe Error Path:** Are all acquired resources (GPIOs, mutexes, IRQs) released on every failure branch?
*   [ ] **Hardware Sequence:** Is the device enabled/powered before I2C/SPI communication begins?
*   [ ] **Signedness:** Are all comparisons between array indices and lengths using `unsigned` types?
*   [ ] **PM Wrappers:** Are `suspend`/`resume` functions wrapped in `ifdef CONFIG_PM` to avoid "unused function" warnings?
