# TTY & 8250 Serial Driver Architecture (`drivers/tty/serial/`)

## 8250 Module Split and Unidirectional Dependency (`drivers/tty/serial/8250/Makefile`)

When built as modules (`CONFIG_SERIAL_8250=m`), the 8250 driver is split into two `.ko` modules with a strict unidirectional dependency:
- **`8250_base.ko`:** `8250_port.o` (always) plus `8250_dma.o`, `8250_dwlib.o`, `8250_fintek.o`, `8250_pcilib.o`, `8250_rsa.o` (when enabled).
- **`8250.ko`:** `8250_core.o`, `8250_platform.o` (always) plus `8250_pnp.o` (when `CONFIG_SERIAL_8250_PNP` is enabled).

`8250.ko` depends on `8250_base.ko`:
- Code in `8250_base.ko` (`8250_port.c`, etc.) must **never** call functions defined in `8250.ko` (`8250_core.c`, `8250_platform.c`, `8250_pnp.c`), as that creates a circular module dependency under `CONFIG_SERIAL_8250=m`.
- Any helper in `8250_base.ko` called from `8250.ko` (or other 8250 sub-drivers) must be exported with `EXPORT_SYMBOL_GPL()`.
