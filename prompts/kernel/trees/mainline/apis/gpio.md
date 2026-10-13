# GPIO Subsystem API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for the Linux GPIO subsystem. It is mandatory for all kernel developers and automated review systems to adhere to these invariants to prevent common classes of bugs.

## 1. Legacy vs. Descriptor API
- **Prohibition**: `<linux/gpio.h>` and integer-GPIO helpers (`gpio_get_value()`, `gpio_set_value()`, `gpio_direction_input()`, `gpio_direction_output()`, `gpio_request()`) are deprecated and must not be used in new code.
- **Descriptor API**: All new code must use the descriptor-based API (`gpiod_*`).
- **Polarity**: `gpiod_get_value()` / `gpiod_set_value()` operate on logical line state (inverting physical level when `GPIO_ACTIVE_LOW` is set). `gpiod_get_raw_value()` / `gpiod_set_raw_value()` ignore active-low polarity and should not be used to work around buggy Device Tree polarity flags; add a quirk in `of_gpio_try_fixup_polarity()` in `drivers/gpio/gpiolib-of.c` instead.

## 2. Ownership and Lifecycle
- **Managed Resources**: Use `devm_gpiod_get()` to tie the GPIO lifecycle to the `struct device`. The subsystem automatically calls `gpiod_put()` during driver unbind.
- **Manual Management**: If `gpiod_get()` is used, the caller **must** explicitly call `gpiod_put()` to prevent resource leaks.
- **Reference Counting**: The `gpio_chip` is reference-counted internally via `struct gpio_device`. Drivers must not manually manipulate the refcount of the `gpio_chip` structure.
- **Teardown**: Ensure all asynchronous work (e.g., `kthread_worker`, `delayed_work`) is cancelled synchronously before the driver unbinds to prevent use-after-free.

## 3. Execution Context and Synchronization
- **Atomic vs. Sleepable**:
    - **Non-sleeping**: `gpiod_get_value()` and `gpiod_set_value()` are strictly for atomic contexts. They must **never** be called if the underlying GPIO controller requires a sleeping bus (e.g., I2C/SPI expanders).
    - **Violation**: Calling a sleep-capable function in an atomic context (e.g., spinlock held, hard IRQ) will trigger a kernel panic.
- **Locking**: The GPIO core manages internal synchronization for state changes. Drivers implementing `gpio_chip` callbacks must ensure no circular dependencies exist with `pinctrl` or `irq_chip` subsystems.
- **Context Awareness**: If a GPIO operation requires a sleeping bus, the driver must ensure the caller is in a process context where `might_sleep()` is permitted.

## 4. Error Handling and Return Conventions
- **Error Sentinels**: All `gpiod_get*` functions return an `ERR_PTR()` on failure.
    - **Mandatory Check**: Always use `IS_ERR(desc)` to check for failure. Do not check for `NULL` unless the function is explicitly documented as returning `NULL` (e.g., `gpiod_get_optional` returns `NULL` if the GPIO is not defined in the device tree).
- **Propagation**: If `gpiod_get_value` returns an error, propagate it; do not assume a default value.
- **Probe Deferral**: If a GPIO is not yet available, `gpiod_get` returns `-EPROBE_DEFER`. Drivers must propagate this error to trigger a re-probe.

## 5. Registration and Publication
- **Registration**: Use `devm_gpiochip_add_data()` to ensure the chip is automatically removed on driver detach.
- **Publication-Before-Initialization**: Ensure all hardware-specific state (registers, IRQ domains) is fully initialized **before** calling `gpiochip_add_data()`. Exposing a chip before it is ready leads to race conditions where consumers may attempt to access uninitialized registers.
- **Hotplug**: Ensure `remove` callbacks are idempotent. Stop all pending work and ensure no further calls to `gpiod_*` are made after the device is removed.

## 6. Hardware Constraints and MMIO
- **Endianness**: When using `GPIO_GENERIC`, verify hardware endianness. Use `GPIO_GENERIC_BIG_ENDIAN` if the hardware register bit order is reversed.
- **Alignment**: MMIO registers must be accessed according to hardware alignment requirements. Use `readl`/`writel` to ensure proper memory ordering.
- **Validation**: Validate `ngpio` and register offsets against the hardware datasheet. Accessing non-existent registers causes system hangs.
- **Bitmasking**: When using `gpio_regmap`, ensure `reg_mask_xlate` correctly maps the offset to the hardware register bit.

## 7. Review Invariants
- **Mixing Resources**: Do not mix `devm_` and manual `gpiod_put()` for the same resource; this leads to double-free vulnerabilities.
- **Mandatory Callbacks**: If implementing a custom `gpio_chip`, ensure `get_direction` and `direction_input`/`direction_output` are implemented if the hardware supports dynamic direction switching.
- **Pinmuxing**: Ensure `pinctrl` state is set to "default" before the GPIO chip is registered.
- **Reset Logic**: Re-apply GPIO configuration after any hardware reset event.
- **Parameter Validation**: Always validate input parameters (e.g., `offset < ngpio`) before performing register arithmetic to prevent out-of-bounds memory access.
- **Standardization**: Use `gpiod_flags` (e.g., `GPIOD_OUT_LOW`, `GPIOD_IN`) to ensure the initial state is applied atomically during the acquisition phase.
- **IRQ Domains**: When using `gpio_irq_chip`, ensure `child_to_parent_hwirq` is correctly implemented for hierarchical domains to prevent IRQ mapping failures.
- **Regmap Usage**: When using `gpio_regmap_config`, ensure `reg_stride` is set if registers are non-consecutive to prevent incorrect register access.
- **Atomic Context Safety**: Always verify if the GPIO controller is backed by a sleeping bus (I2C/SPI) before calling `gpiod_get_value` in an interrupt handler.
- **Resource Cleanup**: Ensure `devm_gpiod_put` is used only for resources allocated via `devm_gpiod_get`.
- **Hierarchical IRQs**: When implementing `gpio_irq_chip`, ensure `populate_parent_alloc_arg` is defined if the default `gpiochip_populate_parent_fwspec_twocell` is insufficient for the hardware topology.
