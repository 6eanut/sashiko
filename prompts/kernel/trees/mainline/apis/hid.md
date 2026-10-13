# HID Transport and Driver API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for developing and maintaining HID drivers in the Linux kernel. It is based on `include/linux/hid.h` and `drivers/hid/hid-core.c`.

## 1. Low-Level Transport Lifecycle (`drivers/hid/hid-core.c`)

*   **Manual Lifecycle**: Allocate `struct hid_device` with `hid_allocate_device()`. There is **no** `devm_` variant for `struct hid_device`.
*   **Initialization**: Populate `name`, `phys`, `uniq`, `ll_driver`, `bus`, `vendor`, `product`, `version`, `country`, `dev.parent`, and `driver_data` before calling `hid_add_device()`.
*   **Teardown**: Call `hid_destroy_device()` if `hid_add_device()` fails or when the transport device is removed.
*   **Safe Input Reporting**: Prefer `hid_safe_input_report(hid, type, data, size, bufsize, interrupt)` over `hid_input_report()` so the core can bounds-check against the allocated buffer size (`bufsize`).
*   **Synchronous Requests**: Responses to synchronous requests (`raw_request`) must be returned directly to the caller and never passed to `hid_input_report()` or `hid_safe_input_report()`.

## 2. High-Level HID Driver APIs (`include/linux/hid.h`)

*   **`hid_parse()` and `hid_hw_start()`**: Call `hid_parse()` (or `hid_open_report()`) followed by `hid_hw_start()` in `probe()`.
    *   If `probe()` fails after `hid_parse()` but before `hid_hw_start()`, the core automatically calls `hid_close_report()`.
    *   Once `hid_hw_start()` succeeds, every subsequent error path in `probe()` and any custom `remove()` callback must call `hid_hw_stop()`.
    *   All driver private data needed by userspace callbacks (such as LED workers or ioctls) must be initialized before `hid_hw_start(..., HID_CONNECT_HIDINPUT)` exposes the input node.
*   **Transport Guard (`hid_is_usb`)**: Never cast `hdev->dev.parent` with `to_usb_interface()` without first verifying `hid_is_usb(hdev)`; HID devices can bind over I2C, Bluetooth, or BPF transports.
*   **Probe/Remove Input Gating (`hid_device_io_start` / `hid_device_io_stop`)**:
    *   By default, `hid_device_probe()` and `hid_device_remove()` hold `hdev->driver_input_lock`, causing `__hid_input_report()` to drop incoming reports until `probe()` returns.
    *   Drivers that need to receive reports during `probe()` (e.g., waiting for a firmware handshake reply via `raw_event()`) must call `hid_device_io_start(hdev)`.
    *   Leaving I/O started when returning from `probe()`/`remove()` is safe (the core releases the lock automatically), and initializing `raw_event()` state after `hid_hw_start()` without calling `hid_device_io_start()` is race-free.
    *   Do not nest `hid_device_io_start()` / `hid_device_io_stop()` calls or invoke them outside the `probe()`/`remove()` thread.

## 3. Concurrency, Memory, and State Invariants

*   **Concurrency**: Synchronously cancel all workqueues and timers (e.g., `cancel_work_sync()`) before `hid_hw_stop()` to prevent use-after-free.
*   **Endianness**: Use `le16_to_cpu()` or `be16_to_cpu()` for multi-byte fields in raw report data.
*   **State Machine**: Explicitly clear state for "release" events (e.g., multitouch contacts). Validate hardware confidence flags before input stack submission.
*   **Error Handling**: Every `hid_` function returning an `int` must be checked. Ignoring these leads to NULL pointer dereferences.
*   **Binding**: Verify `hid_device_id` tables for overlaps with generic drivers to prevent incorrect binding.
*   **Report Integrity**: The core expects complete reports; if the transport layer fragments packets, reassemble them fully before passing to the HID core.
*   **Atomic Context**: HID drivers must not sleep in `raw_event()` or `event()` callbacks. Use `schedule_work()` to offload blocking operations (e.g., firmware loading) to a process context.
*   **Resource Management**: Use `devm_` managed resources for driver-specific allocations (e.g., `devm_kzalloc`) to ensure cleanup on `probe()` failure, but remember that `struct hid_device` itself is not managed by `devm_`.

## 4. Review Checklist for Maintainers

1.  **Is `hid_hw_stop()` called in all `remove()` and error paths?**
2.  **Is `hid_is_usb()` used before casting `dev.parent`?**
3.  **Are all `hid_` return values checked?**
4.  **Is `hid_safe_input_report()` used instead of `hid_input_report()`?**
5.  **Are all workqueues/timers cancelled before `hid_hw_stop()`?**
6.  **Are multi-byte fields handled with endianness-aware macros?**
7.  **Are hardware confidence flags validated before processing input data?**
8.  **Is the device ID table free of duplicates and correctly scoped?**
9.  **Are all pointers obtained from external device input validated before dereference?**
10. **Are user-supplied lengths validated against destination buffers?**
11. **Does the driver handle fragmented packets by reassembling them before core submission?**
12. **Are blocking operations (firmware load) offloaded to process context?**
13. **Is `hid_device_io_start()` used only when necessary during `probe()`?**
14. **Are all `devm_` resources correctly ordered to prevent premature free?**
15. **Is the state machine robust against unexpected hardware resets?**
