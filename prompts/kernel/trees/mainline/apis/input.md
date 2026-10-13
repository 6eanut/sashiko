# Input Device and Force-Feedback APIs (`include/linux/input.h`)

## Allocation, Registration, and `devm` Lifecycle

* **Allocation**: Allocate with `input_allocate_device()` or `devm_input_allocate_device(dev)`. `devm_input_allocate_device(dev)` automatically sets `input_dev->dev.parent = dev`.
* **Registration (`input_register_device`)**: There is no `devm_input_register_device()`; both managed and unmanaged input devices use `input_register_device()`.
  * All driver private data, callbacks, and `input_set_drvdata()` must be initialized before `input_register_device()`, as `open()` or userspace ioctls can be invoked before `input_register_device()` returns.
  * The core automatically sets `EV_SYN` / `SYN_REPORT`.
* **Teardown (`input_free_device` vs `input_unregister_device`)**:
  * Never call `input_free_device()` on a successfully registered `input_dev` (unless the driver unregisters it and sets `dev = NULL` first, since `input_free_device(NULL)` is a safe no-op).
  * Calling `input_free_device()` (before registration) or `input_unregister_device()` (after registration) on a `devm_input_allocate_device()` pointer is safe and removes the devres tracking synchronously without double-free or double-unregister.
* **Active State Check (`input_device_enabled`)**: Must be called while holding `input_dev->mutex`.

## Event Reporting Helpers

* **`input_event()` Before Registration**: Safe to call after allocation and before `input_register_device()`; it updates internal state (`keybit`, abs values) without propagating to handlers.
* **Typed Helpers and `input_sync()`**: Prefer `input_report_key()`, `input_report_abs()`, and `input_report_rel()` over raw `input_event()`, and finish every logical event packet with `input_sync()`.
* **Locking**: `input_event()` and `input_report_*()` acquire `input_dev->event_lock` (spinlock, disabling local interrupts). Do not hold locks that would invert lock ordering against `event_lock`.

## Force-Feedback (`input_ff_create_memless`)

* `input_ff_create_memless(dev, data, play_effect)` takes ownership of `data` and frees it via `kfree()` when the input device is destroyed.
* If `data` is non-NULL, it **must** be allocated with `kmalloc()`/`kzalloc()` (never `devm_kzalloc()`) and **must not** be manually freed by the driver after `input_ff_create_memless()` succeeds.
