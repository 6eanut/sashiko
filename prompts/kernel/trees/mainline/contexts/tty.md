# TTY & 8250 Serial Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides architectural and safety invariants for the TTY and 8250 serial subsystems to prevent concurrency, lifecycle, and error-handling vulnerabilities in Linux v7.3-rc5.

---

## 1. 8250 Module Split and Unidirectional Dependency (`drivers/tty/serial/8250/Makefile`)

When built as modules (`CONFIG_SERIAL_8250=m`), the 8250 driver is split into two `.ko` modules with a strict unidirectional dependency:
- **`8250_base.ko`:** Contains `8250_port.o` (always) plus `8250_dma.o`, `8250_dwlib.o`, `8250_fintek.o`, `8250_pcilib.o`, `8250_rsa.o` (when enabled).
- **`8250.ko`:** Contains `8250_core.o`, `8250_platform.o` (always) plus `8250_pnp.o` (when `CONFIG_SERIAL_8250_PNP` is enabled).

**Dependency Rules:**
- `8250.ko` depends on `8250_base.ko`.
- Code in `8250_base.ko` must **never** call functions defined in `8250.ko` (`8250_core.c`, `8250_platform.c`, `8250_pnp.c`), as this creates a circular dependency.
- Any helper in `8250_base.ko` called from `8250.ko` or other sub-drivers must be exported with `EXPORT_SYMBOL_GPL()`.

---

## 2. Synchronization Model & Lock Hierarchy

### Lock Hierarchy (TTY Core)
To prevent deadlocks in PTY pairs, the following strict order is enforced:
1. **`legacy_mutex`**: Master PTY must be locked before Slave PTY.
2. **`tty_buffer lock`**: Slave PTY must be locked before Master PTY.
3. **`termios_rwsem`**: Always acquired *after* the `tty_buffer lock`.

### Atomic vs. Blocking Contexts
- **Atomic Context**: Functions like `uart_write_wakeup()` or interrupt handlers must **never** call `mutex_lock()`, `msleep()`, or any sleeping function.
- **Interrupt Masking**: When accessing shared hardware registers, use `spin_lock_irqsave()` to prevent reentrancy deadlocks between thread context and hardware interrupt handlers.
- **Re-validation**: Always re-validate pointers after dropping and re-acquiring locks, as the object may have been freed or re-initialized in the interim.

---

## 3. Object Lifecycles & Teardown

- **`struct tty_struct`**: Reference-counted. Never access after dropping a reference unless holding a valid lock or verifying via `tty_kref_get()`.
- **`struct tty_port`**: Manages physical port state; often outlives `tty_struct`.
- **Teardown Sequence**:
    1. Quiesce hardware (stop TX/RX).
    2. Cancel work (`tty_buffer_cancel_work()`).
    3. Flush work (`tty_buffer_flush_work()`).
    4. Release references.
- **Asynchronous Safety**: Ensure all `work_struct` and `timer_list` objects are canceled/shutdown synchronously (`cancel_work_sync`, `timer_shutdown_sync`) before the memory backing them is freed.

---

## 4. Initialization & Error Unwind

- **Premature Publication**: Never register a device (e.g., `tty_register_device()`) or expose a global pointer until the object is fully initialized.
- **Symmetry**: Every `tty_port_init()` or `tty_alloc_driver()` must have a corresponding cleanup call in the error path.
- **Error Pointers**: Always check allocation functions with `IS_ERR()` before dereferencing.
- **Resource Leaks**: Ensure `of_node_put()` is called on all error paths for nodes obtained via `of_find_node_by_name()`.

---

## 5. Hardware Interfacing & DMA

- **Register Access**: Use provided driver-specific accessors. Direct access bypasses memory barriers and platform-specific quirks.
- **DMA State**: If a DMA transfer is zero-length, the state machine must transition to `IDLE` to prevent hangs.
- **Errata**: Always check for hardware-specific flags (e.g., `UPF_BUG_THRE`) before enabling features like DMA or specific FIFO thresholds.
- **Interrupts**: If hardware interrupt generation depends on a FIFO threshold, enable the interrupt *after* the FIFO is filled to guarantee the trigger is caught.

---

## 6. Bug Patterns & False-Positive Guards

- **Flag**: Any `kfree()` of a structure containing a `timer_list` or `work_struct` without a preceding `timer_shutdown_sync()` or `cancel_work_sync()`.
- **Flag**: Any `spin_lock()` call inside a function documented to sleep (e.g., `tty_ldisc_lock`).
- **Ignore**: "Unused variable" warnings in `8250_base.c` within `#ifdef CONFIG_SERIAL_8250_DMA` blocks.
- **Validation**: Always verify function pointers (e.g., `ops->callback`) before invocation to prevent NULL pointer dereferences.
