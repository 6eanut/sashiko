# Input Subsystem Code Context (`drivers/input/input.c`)

## Unregistration, Handler Disconnection, and Async Work

* **`input_dev->mutex` Serialization**: The input core serializes `input_dev->open()` and `input_dev->close()` under `input_dev->mutex`. `input_unregister_device()` automatically invokes `close()` if the device is currently open.
* **Synchronous Handler Disconnection**: During `input_unregister_device()`, the input core immediately disconnects all attached handlers (such as `evdev`, setting `evdev->exist = false` under mutex) and drains in-flight synchronous handler calls before returning. Even though the refcounted `struct input_dev` may outlive the driver until userspace closes all file descriptors, synchronous callbacks (`open`, `close`, `event`) cannot be invoked after unregistration completes.
* **Asynchronous Background Work Exception**: Handler disconnection does *not* stop driver-managed asynchronous work (such as custom workqueues used to defer haptic or LED writes). While `ff-memless` stops its own internal effect timer during `input_unregister_device()` (via `ff->stop`), drivers must still synchronously cancel their own workqueues/timers in `close()` or `remove()`.
