# Linux Kernel API Guide: Common Clock Framework (CCF)

This guide provides authoritative rules for the Common Clock Framework (CCF) as of Linux v7.3-rc5. Adherence to these patterns is mandatory for all drivers interacting with the `clk` subsystem.

---

## 1. Core API Overview & Ownership
The CCF distinguishes between **consumers** (drivers using `struct clk`) and **providers** (drivers implementing `struct clk_hw`).

*   **Reference Counting:** Consumers obtain a reference via `clk_get()` or `devm_clk_get()`. This reference is owned by the caller and must be released via `clk_put()`.
*   **Managed Resources:** Use `devm_clk_get()` and `devm_clk_get_enabled()` to tie the clock lifecycle to the `struct device`. This eliminates manual `clk_put()` and `clk_disable_unprepare()` calls in error paths and `remove()` functions.
*   **Initialization Rollback:** If a probe function fails after acquiring multiple clocks, the `devm_*` family handles cleanup automatically. If using manual `clk_get()`, you must unwind in reverse order of acquisition.
*   **Shared Ownership:** The `struct clk_core` manages internal reference counting (`prepare_count`, `enable_count`). Consumers do not manipulate these directly; they use the `clk_prepare_enable()` and `clk_disable_unprepare()` wrappers.

## 2. Calling Context, Locking & Synchronization
The CCF enforces strict context rules based on the operation type:

*   **Prepare vs. Enable:**
    *   `clk_prepare()` / `clk_unprepare()`: **May sleep.** These operations can involve mutex locks and I/O (e.g., I2C/SPI) to configure hardware. Never call these in atomic context (interrupt handlers, spinlocks).
    *   `clk_enable()` / `clk_disable()`: **Atomic.** These are designed to be fast and are safe to call in atomic context.
*   **Atomic Context Restrictions:** If you are in an interrupt handler, you must ensure the clock is already "prepared" (usually done during `probe` or `resume`) so that you only need to call `clk_enable()`/`clk_disable()`.
*   **Locking:** The CCF uses an internal global mutex (`prepare_lock`) for `prepare` operations and a spinlock for `enable` operations. Do not attempt to acquire these locks manually.

## 3. Return Values & Error Sentinels
The CCF uses the standard Linux error pointer convention.

*   **Pointer Validation:** All `clk_get*` functions return a valid `struct clk *` or an `ERR_PTR()` encoded error.
*   **Mandatory Check:** Always use `IS_ERR(clk)` to check for failure.
*   **Optional Clocks:** Use `devm_clk_get_optional()` or `devm_clk_get_optional_enabled()`. These return `NULL` if the clock is missing (e.g., not defined in DT), which is a **success** condition. Do not treat `NULL` as an error.
*   **Unwind Symmetry:** Every successful `clk_prepare_enable()` must be matched by a `clk_disable_unprepare()` on all exit paths, including error handling and `remove()`.

## 4. High-Frequency Bug Patterns & Review Invariants
Based on 100,000+ commits, these are the most critical review invariants:

*   **The "Optional" Trap:** Always check `IS_ERR(clk)` on the result of `devm_clk_get_optional()`, and never use `IS_ERR_OR_NULL(clk)` to reject `NULL`. `devm_clk_get_optional()` returns `NULL` when the optional clock is absent (a valid non-error state where the driver proceeds without it) and an `ERR_PTR()` on real failures (such as `-EPROBE_DEFER` or `-ENOMEM`).
*   **Unbalanced Cleanup:** A common failure is failing to disable a clock in the error path of a `probe` function. **Rule:** If you `clk_prepare_enable()` in `probe`, you must `clk_disable_unprepare()` in the error label and `remove()`.
*   **Register Access:** Never access hardware registers for a peripheral before calling `clk_prepare_enable()`. Accessing registers while the clock is gated will cause a bus hang or silent data corruption.
*   **Hardcoded Indices:** Avoid hardcoded clock indices. Use `of_clk_get_by_name()` or `devm_clk_get()` with string identifiers to ensure stability across hardware revisions.
*   **Division by Zero:** When calculating clock rates, validate that divisors derived from hardware are non-zero.
*   **Atomic/Sleep Mismatch:** Flag any `clk_prepare` call found inside a `spin_lock` critical section or an `irq` handler as a critical bug.

## 5. Registration & Teardown
*   **Publication:** Clocks are registered via `devm_clk_hw_register()`. The framework manages the `clk_core` lifecycle.
*   **Synchronous Cancellation:** If your driver uses clock notifiers (`clk_notifier_register`), you must unregister them in the `remove()` path to prevent the framework from calling into freed memory.
*   **Hardware Parking:** If a clock mux requires "parking" on a safe source before disabling, ensure this is implemented in the `clk_ops->disable` callback.

## 6. Summary Checklist for Reviewers
1.  **Is it `devm_`?** If not, is there a manual `clk_put` in `remove`?
2.  **Is it `optional`?** If so, is `NULL` handled correctly?
3.  **Is the context correct?** Are `prepare` calls outside of atomic sections?
4.  **Are all error paths covered?** Does the error label unwind the clock state?
5.  **Is the clock enabled before register access?** Check the order of operations in `probe`.
6.  **Are divisors validated?** Check for potential division-by-zero in rate calculations.
