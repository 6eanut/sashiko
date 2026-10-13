# GPIO Subsystem Maintainer Notes

**MAINTAINERS Sections:** `GPIO SUBSYSTEM`

## Legacy `<linux/gpio.h>` Rejection

- Reject any new code that includes `<linux/gpio.h>` or calls legacy integer-based `gpio_*()` APIs (`gpio_request()`, `gpio_get_value()`, `gpio_set_value()`, etc.).
- GPIO provider drivers must include `<linux/gpio/driver.h>`; GPIO consumers must include `<linux/gpio/consumer.h>` (`gpiod_*()` descriptor APIs).

## Polarity Quirks vs Raw Accessors (`gpiod_get_raw_value()`)

- When converting consumers from legacy `gpio_get_value()`/`gpio_set_value()` to `gpiod_get_value()`/`gpiod_set_value()`, descriptor APIs honor active-low polarity flags from Device Tree (`GPIO_ACTIVE_LOW` / `GPIO_ACTIVE_HIGH`).
- If an existing Device Tree binding specifies the wrong polarity, add a polarity fixup entry in `of_gpio_try_fixup_polarity()` (`drivers/gpio/gpiolib-of.c`) rather than switching the consumer to `gpiod_get_raw_value*()` or `gpiod_set_raw_value*()`.

## Generic MMIO (`GPIO_GENERIC`) & Regmap (`GPIO_REGMAP`) Helpers

- **MMIO bank drivers:** New GPIO drivers with direct 1-bit-per-line 8/16/32/64-bit MMIO registers should `select GPIO_GENERIC` in `Kconfig` and use `<linux/gpio/generic.h>` rather than open-coding `.get`/`.set`/`.direction_*` callbacks. Multi-register controllers (e.g., 128 lines across four 32-bit registers) should model each 8/16/32/64-line bank as a chip or sub-node where appropriate.
- **Regmap-based drivers:** New GPIO drivers built on `<linux/regmap.h>` with regular bit/register strides should `select GPIO_REGMAP` and use `<linux/gpio/regmap.h>`.
