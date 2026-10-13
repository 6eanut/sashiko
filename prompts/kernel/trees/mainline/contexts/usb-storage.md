# USB Mass Storage `unusual_devs.h` (`drivers/usb/storage/`)

## `UNUSUAL_DEV()` Subclass and Protocol Override Rules (`drivers/usb/storage/unusual_devs.h`, `drivers/usb/storage/usb.c`)

```c
UNUSUAL_DEV(idVendor, idProduct, bcdDeviceMin, bcdDeviceMax,
            vendor_name, product_name,
            use_protocol,   /* 7th arg: subclass (USB_SC_*) */
            use_transport,  /* 8th arg: protocol (USB_PR_*) */
            init_function, Flags)
```

- `USB_SC_DEVICE` (`0xff`) and `USB_PR_DEVICE` (`0xff`) instruct `get_device_info()` (`drivers/usb/storage/usb.c`) to use the device's self-reported `bInterfaceSubClass` and `bInterfaceProtocol`.
- If an `UNUSUAL_DEV()` entry specifies an explicit `USB_SC_*` (e.g., `USB_SC_SCSI` = `0x06`) or `USB_PR_*` (e.g., `USB_PR_BULK` = `0x50`) override that matches what the device already reports in its USB interface descriptor (`Sub=06 Prot=50` in `/sys/kernel/debug/usb/devices`), `get_device_info()` logs a `dev_notice` on every device insertion asking the user to report the redundant entry (unless `US_FL_NEED_OVERRIDE` is set).
- Unless the device misreports its subclass/protocol, `UNUSUAL_DEV()` entries should pass `USB_SC_DEVICE, USB_PR_DEVICE`.
