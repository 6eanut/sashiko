# Serial Core & UART API Guide (Linux v7.3-rc5)

This guide defines the authoritative API contract for the Linux Serial Core subsystem. Adherence to these rules is mandatory for all UART drivers to ensure stability, prevent deadlocks, and avoid use-after-free vulnerabilities.

## 1. Serial Core & UART Registration API (`include/linux/serial_core.h`)

### `uart_add_one_port()` Synchronous Callback Ordering and Runtime PM
`uart_add_one_port()` (`drivers/tty/serial/serial_core.c`) synchronously runs `uart_configure_port()` before returning, invoking:
- `uart_ops->config_port()` (when `UPF_BOOT_AUTOCONF` is set in `port->flags`)
- `uart_ops->pm()` (via `uart_change_pm()` to power on the port and power it back off for non-console ports)
- `uart_ops->set_mctrl()` (to deassert modem control lines)
- `port->rs485_config()` (via `uart_rs485_config()` when `SER_RS485_ENABLED` is set)

If any of these callbacks (especially `uart_ops->pm()`) calls Runtime PM helpers such as `pm_runtime_resume_and_get()` or `pm_runtime_put_sync()`, the driver **must** call `pm_runtime_enable()` or `devm_pm_runtime_enable()` *before* `uart_add_one_port()`. Enabling Runtime PM after `uart_add_one_port()` causes `uart_change_pm()` to run against an unenabled Runtime PM state during probe.

## 2. Object Ownership, Lifetime, and Reference Counting
*   **Reference Tracking:** Use `uart_port_ref()` and `uart_port_deref()` to manage port object lifetimes. When a lookup yields a shared object, the caller must ensure the reference is held for the duration of the operation.
*   **Teardown:** `uart_remove_one_port()` must be called to unregister. Ensure all asynchronous work (e.g., `uart_write_wakeup` triggers) is flushed or cancelled before calling this to prevent dangling pointer dereferences.
*   **Initialization:** The object must be fully linked and initialized before it is exposed to user-space or the console subsystem.
*   **Global Registry:** Pointers stored in global registries must be cleared immediately upon object destruction or failed initialization to prevent use-after-free.

## 3. Execution Context, Locking & Synchronization
*   **Atomic Context:** Functions marked "This call must not sleep" (e.g., `uart_ops->tx_empty`, `start_tx`, `stop_tx`) must **never** call `mutex_lock()`, `msleep()`, or any function that might block.
*   **Locking Rules:**
    *   `port->lock` is a spinlock. It must be held when accessing hardware registers or modifying `port->state`.
    *   **Interrupt Safety:** Always use `spin_lock_irqsave()` and `spin_unlock_irqrestore()` for `port->lock` to ensure the lock is safe against concurrent interrupt handlers.
    *   **Deadlock Prevention:** Never perform memory allocation (`kmalloc` with `GFP_KERNEL`) or free operations while holding `port->lock`. This is a common source of recursion in console drivers.
*   **Asynchronous Callbacks:** If a workqueue or timer accesses the port, it **must** acquire `port->lock` and verify the port's existence via `uart_port_check()` before proceeding.

## 4. Return Values & Error Recovery
*   **Error Propagation:** Helper functions return standard integer error codes. If a probe function fails, it must perform a full rollback of all resources (unmap registers, free buffers, disable IRQs) before returning the error.
*   **Pointer Sentinels:** Use `IS_ERR()` to validate pointers returned by subsystem lookups. Do not assume a non-NULL pointer is valid.
*   **Partial Failure:** If a multi-step configuration fails, the error path must restore the hardware to a known state (e.g., disabling interrupts) before returning.
*   **Initialization:** Ensure all fallible allocations are performed *before* exposing an object to global subsystems that might use it.

## 5. High-Frequency Bug Patterns & Review Invariants
*   **The "Double-Free" Guard:** When freeing transmit buffers (`uart_free_xmit_buf`), **never** hold `port->lock`. The core handles the synchronization; holding the lock here is a frequent cause of circular locking dependencies.
*   **The "Early-Console" Race:** Ensure the console is fully initialized before exposing the port to user-space. Use `uart_add_one_port()` only after all private data structures are fully populated.
*   **The "FIFO Drain" Invariant:** In `stop_tx`, ensure the hardware FIFO is drained if required by the hardware specification. Simply stopping the transmitter when the software buffer is empty can lead to truncated output.
*   **Struct Initialization:** Use empty braces `{}` for struct initialization to avoid compiler warnings about excess elements or uninitialized members.
*   **Static Encapsulation:** Always verify that `uart_ops` implementations are `static` to prevent namespace pollution and ensure the driver's internal state machine remains encapsulated.
*   **Feature Gating:** All user-facing configuration interfaces (e.g., RS485 ioctls) must be gated by the actual hardware support flags.

## 6. Summary of Mandatory Invariants
| Scenario | Requirement |
| :--- | :--- |
| **Probe Failure** | Roll back all resources (IRQs, buffers, mappings) before returning error. |
| **Runtime PM** | Enable PM *before* `uart_add_one_port()`. |
| **Locking** | Never sleep while holding `port->lock`. |
| **Console** | Protect register access with `port->lock` to prevent `earlycon` interference. |
| **Registration** | Fully initialize private data *before* calling `uart_add_one_port()`. |
| **Buffer Cleanup** | Do not free transmit buffers while holding `port->lock`. |
| **State Tracking** | Track registration state to ensure proper cleanup and re-insertion. |
