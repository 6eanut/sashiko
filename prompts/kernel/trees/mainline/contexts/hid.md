# HID Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for the Linux HID subsystem. Developers must adhere to these rules to prevent vulnerabilities, including use-after-free, OOB memory access, and synchronization deadlocks.

## 1. `struct hid_ll_driver` Mechanics (`drivers/hid/hid-core.c`)

*   **Mandatory Callbacks**: `raw_request` is mandatory (`hid_add_device()` returns `-EINVAL` if absent) and must perform synchronous transfers without blocking on the core's `->wait()` helper. `parse` must pass the raw report descriptor to `hid_parse_report()` (otherwise `hid_add_device()` fails with `-ENODEV`).
*   **Serialized `open` / `close`**: `hid_hw_open()` and `hid_hw_close()` serialize `ll_driver->open` and `ll_driver->close` under `hdev->ll_open_lock` and refcount via `hdev->ll_open_count`. Transport drivers do not need redundant open/close refcounting.
*   **Asynchronous `output_report`**: Must be asynchronous and must not trigger synchronous `SET_REPORT` transfers.

## 2. Devres Unbind Ordering and `remove()`

*   **Devres/Remove Interaction**: The HID core opens a devres group before `probe()`. If a driver omits `.remove`, `hid_device_remove()` calls `hid_hw_stop()` *before* devres releases driver resources.
*   **Devres-Managed Stop**: If a driver manages `hid_hw_stop()` via `devm_add_action_or_reset()`, it **must** provide an empty `.remove()` callback so the core does not call `hid_hw_stop()` early out of LIFO order (e.g., before unregistering a child `devm` I2C adapter).
*   **Manual Teardown**: If a driver implements a custom `.remove()` without devres-managed stop, it must call `hid_hw_stop()` first and then synchronously cancel all custom timers/workqueues (`timer_delete_sync()`, `cancel_work_sync()`) before returning.

## 3. Report Descriptor Fixups and Validation

*   **`report_fixup` Memory Lifetime**: `hid_open_report()` passes a writable copy of the descriptor to `report_fixup()` and copies the returned buffer afterward. In-place edits require no allocation. If a driver allocates a replacement buffer and stores the pointer in private data, it must free that buffer in both `remove()` and `probe()` error paths.
*   **Report Bounds Validation**: Malicious or fuzzed descriptors can declare fewer fields or usages than expected. Always validate `report->maxfield` before indexing `report->field[N]`, and validate `field->maxusage` before indexing `field->usage[N]` or `field->value[N]`.
*   **Arithmetic Integrity**: Use `check_mul_overflow()` when calculating buffer sizes based on descriptor fields to prevent integer wrapping. Ensure allocations do not exceed `HID_MAX_BUFFER_SIZE`.

## 4. `hid-input` Usage Mapping and Quirks (`drivers/hid/hid-input.c`)

*   **`input_mapping()` Return Values**: Returning `0` selects generic mapping, `< 0` ignores the usage, and `> 0` indicates the driver handled the mapping. Returning `> 0` without setting `usage->type`/`usage->code` or capability bits is the standard idiom for silently hiding a HID usage from userspace (not a bug).
*   **Node Splitting Quirks**: `hid-generic` sets `HID_QUIRK_INPUT_PER_APP` by default (grouping reports by application collection). Do not suggest `HID_QUIRK_MULTI_INPUT` (which splits unconditionally on every report ID) without verifying the device's report descriptor structure.

## 5. Synchronization and Concurrency

*   **Atomic Context Constraints**: `hid_input_report()` and `hid_report_raw_event()` may be called from interrupt context. Never perform blocking operations (mutexes, `msleep()`, `kmalloc(GFP_KERNEL)`) in these paths. Use `GFP_ATOMIC` for allocations.
*   **Locking Hierarchy**: `hdev->ll_open_lock` (mutex) is the outermost lock for device state. Avoid holding driver-specific locks while calling `hid_hw_start()` or `hid_hw_stop()` to prevent circular dependencies.
*   **Timer/Work Safety**: Ensure timer/work callbacks check device state (e.g., `hdev->status & HID_STAT_STARTED`) before accessing `hdev` members. Use `timer_delete_sync()` to prevent re-arming during teardown.

## 6. Initialization and Object Lifetimes

*   **`hid_device` Ownership**: Drivers must not manually `kfree()` a `hid_device`. Use `hid_allocate_device()` and `put_device()` (via `hid_destroy_device()`).
*   **Premature Publication**: Do not call `hid_add_device()` until all internal state (e.g., `hdev->driver_data`) is fully initialized.
*   **Error Unwind**: Every `hid_parse()` or `hid_hw_start()` call must be balanced by a corresponding `hid_hw_stop()` in the error path.
*   **Sysfs Cleanup**: If `sysfs_create_group()` is used, ensure the error path calls `sysfs_remove_group()` for all previously created groups.

## 7. Power Management

*   **Reference Balance**: Every `pm_runtime_get_sync()` must have a corresponding `pm_runtime_put_autosuspend()` or `pm_runtime_put()`.
*   **Suspend/Resume**: Do not access hardware registers in `suspend()`/`resume()` if the device failed to probe. Verify `hdev->status & HID_STAT_ADDED`.
*   **Autosuspend**: Ensure `pm_runtime_disable()` is called in `remove()` to prevent race conditions with pending autosuspend work.
*   **Hardware State**: Always verify `hdev->status` flags before issuing commands to the transport layer during power transitions to avoid accessing uninitialized hardware.

## 8. Architectural Invariants for Subsystem Stability

*   **Input Validation**: All lengths provided by external hardware must be validated against the destination buffer size before memory operations to prevent OOB access.
*   **Pointer Arithmetic**: Perform pointer arithmetic on the base buffer pointer rather than a struct field to avoid incorrect object size inference by the compiler.
*   **Callback Context**: Cross-layer callbacks (e.g., `hid_haptic_input_mapping`) must respect the atomic/blocking constraints of the caller. If a callback is invoked from an interrupt context, it must not sleep.
*   **Error Propagation**: Always distinguish between `NULL` and `ERR_PTR()` when checking return values from `hid_allocate_device()` or similar allocation helpers.
*   **Quiescing**: Before freeing any `hid_device` or associated private data, ensure all workqueues (`wq`) and timers are explicitly cancelled using `cancel_work_sync()` and `timer_delete_sync()`.
