# Hardware Monitoring APIs (`include/linux/hwmon.h`)

## Registration and Core Serialization

* `hwmon_device_register_with_info()` and `devm_hwmon_device_register_with_info()` register a hardware monitoring device using `struct hwmon_chip_info`.
* The hwmon core automatically serializes thermal subsystem and sysfs callbacks for attributes described through the `info` parameter of `hwmon_device_register_with_info()` / `devm_hwmon_device_register_with_info()`.
* Drivers must provide their own locking for interrupt handlers and for custom attributes registered outside `info` (e.g., via `extra_groups`), using `hwmon_lock(hwmon_dev)` / `hwmon_unlock(hwmon_dev)` or `guard(hwmon_lock)(hwmon_dev)`.
