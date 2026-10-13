# Media Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for the Linux Media subsystem, specifically targeting V4L2 subdevices and the CEC (Consumer Electronics Control) framework.

## 1. V4L2 Subdevice State Validation Wrappers (`check_state` / `check_format`)

*   **`v4l2_subdev_state_get_format()` Pre-Validation via `v4l2_subdev_call()`**:
    *   `v4l2_subdev_state_get_format(state, pad, stream)` can return `NULL` in general.
    *   However, when a `v4l2_subdev_pad_ops` callback (`enum_mbus_code`, `get_fmt`, `set_fmt`, `enum_frame_size`, `enum_frame_interval`, `get_selection`, `set_selection`) is invoked exclusively through the media-core wrappers (`call_enum_mbus_code()`, `call_get_fmt()`, `call_set_fmt()`, etc. in `drivers/media/v4l2-core/v4l2-subdev.c`), `check_state()` validates the exact `(state, pad, stream)` tuple first (including calling `v4l2_subdev_state_get_format(state, pad, stream)` when `V4L2_SUBDEV_FL_STREAMS` is set under `CONFIG_VIDEO_V4L2_SUBDEV_API` and returning `-EINVAL` if `NULL`).
*   **When a Missing `NULL` Check Is NOT a Bug**:
    *   If every caller of the callback goes through the matching `call_*` wrapper in `v4l2-subdev.c` and the callback passes the exact same `(state, pad, stream)` arguments validated by `check_state()`, do not report a missing `NULL` check on `v4l2_subdev_state_get_format()`, and cite the `call_*` wrapper + `check_state()` as proof.
*   **When a Missing `NULL` Check IS a Bug**:
    *   Any direct caller (such as `.init_state` or an internal driver call) bypasses the `call_*` wrapper.
    *   The callback queries a different pad, stream, or helper such as `v4l2_subdev_state_get_opposite_stream_format()` (which performs a separate routing lookup not validated by `check_state()`).

## 2. CEC Framework Objects and Synchronization

### CEC Locking Hierarchy
1.  **`cec_devnode->lock`**: Must be held to serialize `open()`, `release()`, and device registration/unregistration.
2.  **`cec_devnode->lock_fhs`**: Protects the `fhs` list.
3.  **`cec_fh->lock`**: Protects the `events` and `msgs` queues for a specific file handle.
4.  **`cec_adapter->lock`**: The primary adapter lock. Many `cec_adap_ops` callbacks are invoked with this lock held. **Never** perform blocking operations (e.g., `msleep()`, `mutex_lock()` on a different mutex) while holding `adap->lock`.

### Teardown and Concurrency
*   **Workqueues**: When a `cec_adapter` is deleted, `cec_pin_ops->free` is invoked. All pending `delayed_work` (e.g., `cec_data->work`) must be cancelled via `cancel_delayed_work_sync()` before the adapter or its associated data structures are freed.
*   **Atomic Context**: `cec_pin_changed()` is called from interrupt context. Do not perform I2C/SPI transfers or `mutex_lock()` inside this path. Use `schedule_work()` to defer to process context.

## 3. Initialization, Registration & Error Unwind

*   **Premature Publication**: A `cec_adapter` or `v4l2_subdev` must be fully initialized (all internal state, workqueues, and hardware registers configured) *before* calling `cec_register_adapter()` or `v4l2_device_register_subdev()`.
*   **Error Unwind**: Use the "Reverse-Order" principle. If `cec_notifier_cec_adap_register()` succeeds, the corresponding `cec_notifier_cec_adap_unregister()` must be called in the error path.
*   **Resource Management**: Prefer `devm_*` functions to ensure automatic cleanup. If `devm_*` is not used, ensure every allocation has a corresponding free in the error path. Failure to unregister `cec_notifier_conn_register` leads to reference count leaks.

## 4. Untrusted Input & Boundary Validation

*   **Array Indexing**: Validate all indices against `CEC_NUM_EVENTS` or the defined control range.
*   **Buffer Copying**: Use `copy_from_user()` and verify the length against the destination buffer size. Clamp input lengths to the maximum allowed size for the specific IOCTL.
*   **Hardware Limits**: Query hardware constraints (e.g., `v4l2_subdev_pad_ops->enum_frame_size`) rather than using hardcoded constants to ensure compatibility with future hardware revisions.
*   **Integer Overflows**: When calculating buffer sizes (e.g., `width * height * bpp`), use `size_t` and check for overflow before allocation.
*   **Cross-Layer Contracts**: When registering a callback function table, validate all mandatory hooks. Optional hooks (like `cec_pin_ops->received`) must be checked for `NULL` before invocation.
*   **Error Propagation**: Always distinguish between `NULL` pointers and `ERR_PTR` sentinels when receiving results from `cec_notifier_conn_register` or similar factory functions.

## 5. Architectural Invariants for Media Drivers

*   **Context Safety**: Always verify if a callback is invoked in atomic context (e.g., `cec_pin_changed`) before attempting to sleep.
*   **State Consistency**: Ensure that `cec_fh` list operations are protected by `cec_devnode->lock` followed by `cec_devnode->lock_fhs`.
*   **Resource Cleanup**: Ensure `cec_pin_ops->free` handles all hardware-specific cleanup to prevent resource leaks during module removal.
*   **Notifier Lifecycle**: Always pair `cec_notifier_conn_register` with `cec_notifier_conn_unregister` to maintain correct reference counts on the notifier object.
*   **Hardware Register Access**: Ensure all register access is performed within the appropriate power domain, verified by the driver's runtime PM state.
*   **Locking Invariants**: When multiple locks are required (e.g., `cec_devnode->lock` and `cec_devnode->lock_fhs`), the acquisition order must be strictly maintained to prevent circular wait conditions.
*   **Memory Safety**: All `cec_msg` structures must be validated for length before being queued in `cec_fh->msgs`.
*   **Asynchronous Completion**: Use `completion` objects for synchronous waits in `cec_data` to ensure the thread remains blocked until the hardware operation finishes or times out.
