# LED Subsystem Maintainer Notes

## Commit Messages, Naming, and Logging Conventions

* **Commit Subject Capitalization**: Capitalize the description after the prefix: `leds: <driver>: <Capitalized description>` (e.g., `leds: qwerty: Add support for Qwerty LED`).
* **Private Data Naming**: Name the driver's private struct after the device (e.g., `struct qwerty_led`) and name local pointer variables `ddata`, rather than generic names like `info` or `priv`.
* **No Success Noise & `dev_err_probe()`**: Do not emit informational "LED registered" / "probe succeeded" messages; use `dev_err_probe()` on probe failure paths.

## Managed Registration and Device Tree Properties

* **Prefer Managed APIs**: New LED drivers should use `devm_led_classdev_register()` / `devm_led_classdev_register_ext()` and `devm_led_trigger_register()` instead of manual registration/unregistration.
* **`color` and `function` vs `label`**: New LED Device Tree bindings and drivers should use the standard `color` and `function` DT properties (producing `/sys/class/leds/<color>:<function>`) instead of the deprecated `label` property.
