# Serial Core & UART Registration API (`include/linux/serial_core.h`)

## `uart_add_one_port()` Synchronous Callback Ordering and Runtime PM

`uart_add_one_port()` (`drivers/tty/serial/serial_core.c`) synchronously runs `uart_configure_port()` before returning, invoking:
- `uart_ops->config_port()` (when `UPF_BOOT_AUTOCONF` is set in `port->flags`)
- `uart_ops->pm()` (via `uart_change_pm()` to power on the port and power it back off for non-console ports)
- `uart_ops->set_mctrl()` (to deassert modem control lines)
- `port->rs485_config()` (via `uart_rs485_config()` when `SER_RS485_ENABLED` is set)

If any of these callbacks (especially `uart_ops->pm()`) calls Runtime PM helpers such as `pm_runtime_resume_and_get()` or `pm_runtime_put_sync()`, the driver **must** call `pm_runtime_enable()` or `devm_pm_runtime_enable()` *before* `uart_add_one_port()`. Enabling Runtime PM after `uart_add_one_port()` causes `uart_change_pm()` to run against an unenabled Runtime PM state during probe.
