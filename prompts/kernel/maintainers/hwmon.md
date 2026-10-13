# Hardware Monitoring Subsystem Maintainer Notes

## Coding Style and Conventions

* Follow the guidelines in `Documentation/hwmon/submitting-patches.rst`.
* Enum values in `drivers/hwmon/` are traditionally lowercase; uppercase is permitted, so do not require existing or new enum names to be renamed solely for case.

## Subsystem Placement and Auxiliary Devices

* Hardware monitoring drivers must reside in `drivers/hwmon/`. Registering new hwmon devices directly from outside `drivers/hwmon/` violates subsystem layering and bypasses maintainer review (existing drivers outside `drivers/hwmon/` are grandfathered and are not bugs).
* If hardware monitoring is secondary functionality on a multi-purpose device (such as a network controller, DRM driver, or platform MFD), expose it as an auxiliary device whose hwmon driver lives in `drivers/hwmon/`.
* Conversely, if a primary hardware monitoring chip exposes secondary functionality (such as GPIO or LEDs), implement the secondary functionality via an auxiliary device in its respective subsystem directory.

## Registration API Policy

* New hwmon drivers must register using `hwmon_device_register_with_info()` or `devm_hwmon_device_register_with_info()`.
