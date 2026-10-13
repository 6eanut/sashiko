# HID Transport and Driver APIs

## Low-Level Transport Lifecycle (`drivers/hid/hid-core.c`)

* **Manual Lifecycle**: Allocate `struct hid_device` with `hid_allocate_device()`. There is no `devm_` variant for `struct hid_device`.
* **Initialization**: Populate `name`, `phys`, `uniq`, `ll_driver`, `bus`, `vendor`, `product`, `version`, `country`, `dev.parent`, and `driver_data` before calling `hid_add_device()`.
* **Teardown**: Call `hid_destroy_device()` if `hid_add_device()` fails or when the transport device is removed.
* **Safe Input Reporting**: Prefer `hid_safe_input_report(hid, type, data, size, bufsize, interrupt)` over `hid_input_report()` so the core can bounds-check against the allocated buffer size (`bufsize`). Responses to synchronous requests (`raw_request`) must be returned directly to the caller and never passed to `hid_input_report()` or `hid_safe_input_report()`.

## High-Level HID Driver APIs (`include/linux/hid.h`)

* **`hid_parse()` and `hid_hw_start()`**: Call `hid_parse()` (or `hid_open_report()`) followed by `hid_hw_start()` in `probe()`.
  * If `probe()` fails after `hid_parse()` but before `hid_hw_start()`, the core automatically calls `hid_close_report()`.
  * Once `hid_hw_start()` succeeds, every subsequent error path in `probe()` and any custom `remove()` callback must call `hid_hw_stop()`.
  * All driver private data needed by userspace callbacks (such as LED workers or ioctls) must be initialized before `hid_hw_start(..., HID_CONNECT_HIDINPUT)` exposes the input node.
* **Transport Guard (`hid_is_usb`)**: Never cast `hdev->dev.parent` with `to_usb_interface()` without first verifying `hid_is_usb(hdev)`; HID devices can bind over I2C, Bluetooth, or BPF transports.
* **Probe/Remove Input Gating (`hid_device_io_start` / `hid_device_io_stop`)**:
  * By default, `hid_device_probe()` and `hid_device_remove()` hold `hdev->driver_input_lock`, causing `__hid_input_report()` to drop incoming reports until `probe()` returns.
  * Drivers that need to receive reports during `probe()` (e.g., waiting for a firmware handshake reply via `raw_event()`) must call `hid_device_io_start(hdev)`.
  * Leaving I/O started when returning from `probe()`/`remove()` is safe (the core releases the lock automatically), and initializing `raw_event()` state after `hid_hw_start()` without calling `hid_device_io_start()` is race-free.
  * Do not nest `hid_device_io_start()` / `hid_device_io_stop()` calls or invoke them outside the `probe()`/`remove()` thread.
