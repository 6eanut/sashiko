# USB Mass Storage Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides the authoritative technical context for the `drivers/usb/storage/` subsystem. It is designed for automated patch review and bug prevention, focusing on the unique invariants of the USB Mass Storage (UMS) class driver.

---

## 1. `UNUSUAL_DEV()` Subclass and Protocol Override Rules

The `UNUSUAL_DEV()` macro in `drivers/usb/storage/unusual_devs.h` is the primary mechanism for handling hardware quirks.

```c
UNUSUAL_DEV(idVendor, idProduct, bcdDeviceMin, bcdDeviceMax,
            vendor_name, product_name,
            use_protocol,   /* 7th arg: subclass (USB_SC_*) */
            use_transport,  /* 8th arg: protocol (USB_PR_*) */
            init_function, Flags)
```

*   **Override Invariants:**
    *   `USB_SC_DEVICE` (`0xff`) and `USB_PR_DEVICE` (`0xff`) instruct `get_device_info()` (`drivers/usb/storage/usb.c`) to use the device's self-reported `bInterfaceSubClass` and `bInterfaceProtocol`.
    *   If an `UNUSUAL_DEV()` entry specifies an explicit `USB_SC_*` (e.g., `USB_SC_SCSI` = `0x06`) or `USB_PR_*` (e.g., `USB_PR_BULK` = `0x50`) override that matches what the device already reports in its USB interface descriptor (`Sub=06 Prot=50` in `/sys/kernel/debug/usb/devices`), `get_device_info()` logs a `dev_notice` on every device insertion.
    *   Unless the device misreports its subclass/protocol, `UNUSUAL_DEV()` entries must pass `USB_SC_DEVICE, USB_PR_DEVICE`.
    *   Redundant entries are prohibited unless `US_FL_NEED_OVERRIDE` is set.

## 2. Subsystem Architecture, Core Data Structures & Object Lifetimes

The core of the subsystem is the `struct us_data` (defined in `drivers/usb/storage/usb.h`), which encapsulates the state of a single USB mass storage interface.

*   **Object Lifecycle:** The `us_data` structure is allocated during the probe sequence and freed during `disconnect`. The lifecycle is tied to the `usb_interface`.
*   **Ownership:** The `usb_storage` driver owns the `us_data` object. All pending I/O (SCSI commands) must be aborted and the kernel thread terminated before the structure is freed.
*   **Reference Counting:** The subsystem relies on the underlying `usb_device` reference counting. Developers must ensure that `usb_get_dev()` and `usb_put_dev()` are balanced across the probe/disconnect lifecycle.

## 3. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Control Thread:** The kernel thread is the primary execution context for command processing. It must sleep on a wait queue (`us->delay_wait`) when idle.
*   **Locking:** 
    *   `us->dev_mutex`: Protects the state of the `us_data` structure, particularly during device reset or configuration changes.
    *   **Atomic Contexts:** Never perform memory allocations with `GFP_KERNEL` or blocking operations (like `mutex_lock`) inside SCSI command completion callbacks or URB completion handlers. Use `GFP_ATOMIC` for allocations in these paths.
*   **Deadlock Prevention:** The `us->dev_mutex` must never be held while waiting for a SCSI command to complete, as the completion handler may interact with the device state, leading to a circular wait.

## 4. Error Handling, Resource Recovery & Boundary Validation

*   **IS_ERR vs NULL:** Always use `IS_ERR()` to check for failure before dereferencing pointers returned by USB core functions.
*   **Unwind Symmetry:** Every `usb_alloc_urb()` must have a corresponding `usb_free_urb()`. Every `usb_register_dev()` must have a `usb_deregister_dev()`.
*   **Empirical Guard:** Ensure that if the probe sequence fails after allocation, the cleanup routine (which calls `kfree`) is invoked to prevent memory leaks.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **TOCTOU/Bounds Checking:** When parsing sense buffers or device-provided lengths, always clamp the length to the destination buffer size:
    *   *Guard:* `min_t(size_t, device_provided_len, sizeof(local_buffer))`
*   **Data Direction Mismatch:** Ensure that the `scsi_cmnd` data direction matches the actual USB pipe direction. Processing `Data-In` for a `Write` command is a critical security and stability risk.
*   **Uninitialized Variables:** Ensure all fields in `struct us_data` are zero-initialized (use `kzalloc`).
*   **Format Strings:** Never pass a variable string as the format argument to `usb_stor_dbg()`. Use `usb_stor_dbg(us, "%s", variable_string)`.

## 6. Domain-Specific: Storage Controllers & DMA

*   **DMA Alignment:** The USB mass storage layer relies on the USB core for DMA mapping. Ensure that scatter-gather lists are properly handled and that the `max_sectors` flag is respected to prevent buffer overflows on hardware with limited DMA capabilities.
*   **Power Management:** During `suspend`, the driver must ensure all pending URBs are cancelled. During `resume`, the device must be re-initialized if the hardware loses state. Use `usb_autopm_get_interface()` and `usb_autopm_put_interface()` to manage runtime power states.

---

### Critical Checklist for Patch Review
1.  **`UNUSUAL_DEV`:** Ensure it is not redundant and uses the correct `USB_SC_`/`USB_PR_` constants.
2.  **Sleep-in-Atomic:** Verify `GFP_KERNEL` is not used in callbacks.
3.  **Error Paths:** Verify all error paths are balanced (missing `kfree`, `usb_put_dev`, or `mutex_unlock`).
4.  **Validation:** Verify length clamping on all device-supplied data.
5.  **Lifecycle:** Ensure the control thread is stopped before freeing `us_data`.
