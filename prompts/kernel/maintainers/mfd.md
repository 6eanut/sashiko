# Multi-Function Devices (MFD) Maintainer Notes

## Commit Messages, Naming, and Header Placement

* **Commit Subject Capitalization**: Capitalize the description after the prefix: `mfd: <driver>: <Capitalized description>` (e.g., `mfd: max77650: Add support for MAX77650`).
* **Naming Conventions**: Do not include `"mfd"` or the driver filename in struct or device names. Name the private struct after the hardware device (e.g., `struct max77650`) and name local pointer variables `ddata` rather than `info` or `priv`.
* **Header Placement**: Do not place driver-specific headers in `include/linux/mfd/` if they are only needed within `drivers/mfd/`.

## MFD Scope, Directory Placement, and Architecture

* **True Multi-Function Devices Only**: Do not use the MFD API for single-function devices; use it only when registering multiple child devices across different subsystems (and prefer standard `"simple-mfd"` or `"simple-pm-bus"` DT compatibles when no custom parent logic is needed).
* **Target Directory**: New core MFD drivers and new calls to `mfd_add_devices()` / `devm_mfd_add_devices()` belong in `drivers/mfd/` (existing callers outside `drivers/mfd/` are not bugs).
* **Parent-Child Data Sharing**:
  * Child drivers must retrieve shared parent state (such as regmap or IRQ domain) directly via `dev_get_drvdata(pdev->dev.parent)` or `platform_get_drvdata()`, not via bespoke parent-exported getter functions.
  * Resources used by only one child device (e.g., a child-specific clock) must be acquired in that child driver rather than in the MFD parent.
  * Sibling child drivers must not make direct function calls or driver-level callbacks to each other; use standard kernel subsystem APIs.
* **`static const struct mfd_cell` Selection**: In new drivers, declare `mfd_cell` arrays as `static const` without copying/mutating them at runtime, and store a variant `enum` ID in `of_device_id.data` (selecting the cell array via a `switch` on `device_get_match_data(dev)` in `probe()`) rather than passing cell-array pointers directly through `.data`.
