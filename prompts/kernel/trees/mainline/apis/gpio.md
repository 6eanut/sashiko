# GPIO Consumer & Driver API Rules (`include/linux/gpio/consumer.h`, `include/linux/gpio/driver.h`)

## Legacy `<linux/gpio.h>` vs Descriptor API (`gpiod_*`)

- `<linux/gpio.h>` and integer-GPIO helpers (`gpio_get_value()`, `gpio_set_value()`, `gpio_direction_input()`, `gpio_direction_output()`, `gpio_request()`) are deprecated and must not be used in new code.
- `gpiod_get_value()` / `gpiod_set_value()` operate on logical line state (inverting physical level when `GPIO_ACTIVE_LOW` is set), whereas `gpiod_get_raw_value()` / `gpiod_set_raw_value()` ignore active-low polarity and should not be used to work around buggy Device Tree polarity flags (add a quirk in `of_gpio_try_fixup_polarity()` in `drivers/gpio/gpiolib-of.c` instead).
