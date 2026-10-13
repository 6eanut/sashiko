# Multi-Function Device (MFD) Registration APIs (`include/linux/mfd/core.h`)

## Cell Registration, Teardown, and Auto-Indexing

* **`devm_mfd_add_devices()` vs `mfd_add_devices()` / `mfd_remove_devices()`**:
  * Prefer `devm_mfd_add_devices()` so registered child platform devices are automatically removed on probe failure or parent unbind.
  * If using unmanaged `mfd_add_devices()`, any subsequent failure in `probe()` and the parent `.remove()` callback must call `mfd_remove_devices()`.
* **Cell IDs (`PLATFORM_DEVID_AUTO`)**: Pass `PLATFORM_DEVID_AUTO` (or `PLATFORM_DEVID_NONE` when appropriate) to `mfd_add_devices()` / `devm_mfd_add_devices()` to avoid sysfs platform device name collisions when multiple instances of the MFD chip exist.
