# Linux v7.3-rc5 Kernel IRQ API Guide

This guide defines the authoritative constraints and safety invariants for the Linux Interrupt Subsystem (genirq). It targets Linux v7.3-rc5 and incorporates empirical bug patterns derived from the last 100,000 `Fixes:` commits.

## 1. Core API Overview & Ownership / Reference-Counting Lifecycles

The IRQ subsystem manages the mapping between hardware interrupt signals and software handler routines. In v7.3-rc5, ownership is strictly divided between the **IRQ Core** (managing descriptors), **IRQ Chips** (managing hardware registers), and **Device Drivers** (consuming IRQ lines).

### 1.1 IRQ Request and Registration
*   **`request_irq()` / `request_threaded_irq()`**: Establishes a binding between a virtual IRQ number and a handler.
    *   **Ownership**: The caller must provide a `dev_id` (mandatory for `IRQF_SHARED`). This `dev_id` acts as the unique key for `free_irq()`.
    *   **Lifetime**: The handler is active immediately upon successful return (0). All driver-side resources (spinlocks, MMIO mappings, state variables) accessed by the handler **MUST** be initialized before calling `request_irq()`.
*   **`devm_request_irq()` / `devm_request_threaded_irq()`**: Managed variants that automate `free_irq()` on driver unbind.
    *   **Critical Invariant**: Managed IRQs are freed *after* the driver's `.remove()` callback. If the handler accesses resources freed in `.remove()` (e.g., `iounmap`, `kfree`), a use-after-free (UAF) occurs. Use `devm_free_irq()` in `.remove()` (or register cleanup via `devm_add_action_or_reset()` after `devm_request_irq()`) to synchronize teardown; never call manual `free_irq()` on an IRQ registered with `devm_request_irq()`.

### 1.2 IRQ Domains and Mapping
*   **`irq_create_mapping(domain, hwirq)`**: Translates a hardware-specific ID to a Linux virtual IRQ.
    *   **Return Value**: Returns the virtual IRQ number (positive integer) or `0` on failure. **Never** use `IS_ERR()` or check for `< 0` on the return value.
*   **`irq_dispose_mapping(virq)`**: Tears down a mapping. Callers must ensure the IRQ is not currently requested via `free_irq()` before disposing of the mapping.

## 2. Calling Context, Locking & Synchronization Rules

### 2.1 Handler Contexts
*   **Hardirq Handler (`handler`)**: Runs with local interrupts disabled on the current CPU.
    *   **Constraints**: Must not sleep. Must not call `mutex_lock()`, `down()`, or `schedule()`. Must return `IRQ_HANDLED`, `IRQ_NONE`, or `IRQ_WAKE_THREAD`.
*   **Threaded Handler (`thread_fn`)**: Runs in a dedicated kernel thread context.
    *   **Constraints**: Can sleep and acquire mutexes. However, it must remain efficient to avoid system-wide latency.
*   **`IRQF_ONESHOT`**: Mandatory for threaded IRQs where the hardirq handler is `NULL`. It keeps the interrupt line masked until the threaded handler completes, preventing an interrupt storm.

### 2.2 Synchronization and Disabling
*   **`disable_irq(irq)`**: Disables the IRQ and **waits** for any executing handlers on other CPUs to finish.
    *   **Context**: Can sleep. **Must not** be called from within the handler of the same IRQ (deadlock).
*   **`disable_irq_nosync(irq)`**: Disables the IRQ without waiting.
    *   **Context**: Safe to call from any context, including IRQ handlers.
*   **`synchronize_irq(irq)`**: Waits for all handlers (hard and threaded) to complete.
    *   **Context**: Can sleep. Often used in driver `.remove()` or suspend paths to ensure no ISR is running during resource teardown.

## 3. Return Values, Error Sentinels & Unwind / Teardown Symmetry

### 3.1 Return Value Conventions
*   **`request_irq` family**: Returns `0` on success, negative errno (e.g., `-EBUSY`, `-EINVAL`) on failure.
*   **`irq_find_mapping`**: Returns `0` if no mapping exists.
*   **Handler Return Codes**:
    *   `IRQ_HANDLED`: The device issued the interrupt and it was processed.
    *   `IRQ_NONE`: The interrupt was not from this device (critical for `IRQF_SHARED`).
    *   `IRQ_WAKE_THREAD`: (Hardirq only) Wake the associated threaded handler.

### 3.2 Teardown Symmetry
*   **Unwind Order**: Resources must be released in the exact reverse order of acquisition.
    ```c
    // Correct Unwind Pattern
    err = request_irq(irq, handler, ...);
    if (err) goto err_out;
    // ...
    free_irq(irq, dev_id); // Must happen before kfree(dev_id) or iounmap()
    ```
*   **`IRQF_NO_AUTOEN`**: If used, the IRQ is not enabled at request time. The driver must call `enable_irq()` when ready. This is the preferred method to prevent handlers firing during complex initialization.

## 4. High-Frequency Bug Patterns & Review Invariants

### 4.1 Premature Registration (The "Init-Before-Request" Rule)
*   **Bug**: Calling `request_irq()` before `spin_lock_init()`, `mutex_init()`, or `INIT_WORK()` for objects used in the handler.
*   **Invariant**: All synchronization primitives and hardware state required by the ISR must be fully initialized before the IRQ is requested.
*   **Review Check**: Verify `request_irq` occurs after all `init_waitqueue_head`, `spin_lock_init`, and `timer_setup` calls for objects touched by the ISR.

### 4.2 Shared IRQ `dev_id` Mismatch
*   **Bug**: Passing `NULL` as `dev_id` with `IRQF_SHARED`, or passing a different `dev_id` to `free_irq()` than was used in `request_irq()`.
*   **Invariant**: For `IRQF_SHARED`, `dev_id` must be globally unique (usually the driver's private data pointer) and non-NULL.

### 4.3 Use-After-Free in Threaded Handlers
*   **Bug**: Freeing the device private data in `.remove()` while the threaded IRQ handler is still sleeping or running.
*   **Invariant**: Call `free_irq()` (which performs an implicit `synchronize_irq()`) before freeing any data structures accessed by the `thread_fn`.

### 4.4 IRQ Domain Mapping Checks
*   **Bug**: Checking `if (virq < 0)` or `if (IS_ERR_VALUE(virq))` for `irq_create_mapping()`.
*   **Invariant**: The only valid error check for `irq_create_mapping()` and `irq_find_mapping()` is `if (!virq)`.

### 4.5 `IRQF_ONESHOT` with Threaded IRQs
*   **Bug**: Requesting a threaded IRQ with a `NULL` hardirq handler but omitting `IRQF_ONESHOT`.
*   **Invariant**: If `handler == NULL` in `request_threaded_irq()`, `flags` **MUST** include `IRQF_ONESHOT`. Failure to do so will result in the kernel rejecting the request or an immediate interrupt storm.

### 4.6 Missing `of_node_put()` in Domain Lookups
*   **Bug**: Using `of_irq_find_parent()` or similar OpenFirmware helpers without releasing the node reference.
*   **Invariant**: Ensure `of_node_put()` is called on the device node after the IRQ mapping is established or if the lookup fails.

### 4.7 `disable_irq` Deadlocks
*   **Bug**: Calling `disable_irq()` from a context that the IRQ handler itself is waiting on (e.g., a workqueue that the ISR signals).
*   **Invariant**: Ensure no circular dependency exists between the thread calling `disable_irq()` and the ISR's completion. Use `disable_irq_nosync()` if calling from atomic or re-entrant contexts where waiting is impossible.
