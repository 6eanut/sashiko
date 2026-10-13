# Linux v7.3-rc5 Subsystem Code Context Guide: Clock Framework

This guide provides technical invariants and architectural requirements for the Linux Clock Framework (`drivers/clk/`, `include/linux/clk-provider.h`).

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **Core Structures:** The framework centers on `struct clk_core` (internal) and `struct clk_hw` (provider). Providers must embed `struct clk_hw` within private structures and use `container_of()` to retrieve them.
*   **Object Lifetime:** `clk_hw` objects are registered via `clk_hw_register()` or `devm_clk_hw_register()`.
    *   **Managed Resources:** Use `devm_` variants for automatic cleanup. Manual registration requires explicit `clk_hw_unregister()` during teardown to prevent dangling pointers in the global clock tree.
    *   **Device Tree:** Providers registered via `of_clk_add_hw_provider()` must be removed via `of_clk_del_provider()` on error paths or module exit.
*   **Invariants:** A `clk_hw` must be fully initialized (including `clk_init_data`) before registration. `clk_init_data` must not be modified after registration.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Global Locks:** 
    *   `prepare_lock` (mutex): Protects `prepare`/`unprepare` operations (sleepable).
    *   `enable_lock` (spinlock): Protects `enable`/`disable` operations (atomic).
*   **Context Constraints:**
    *   `clk_enable()`/`clk_disable()` are atomic; they must not sleep.
    *   `clk_prepare()`/`clk_unprepare()` can sleep; they must not be called from atomic context or interrupt handlers.
    *   **Atomic SCMI:** If `SCMI_CLK_ATOMIC_SUPPORTED` is enabled, `clk_ops` callbacks must not sleep, as they execute under the global `enable_lock`.
*   **Deadlock Prevention:** Do not acquire a mutex inside `enable`/`disable` callbacks. If hardware access requires a mutex, the clock must be marked as non-atomic, delegating serialization to the `prepare_lock`.

## 3. Initialization, Registration & Teardown Ordering

*   **Publication:** Clocks are visible system-wide immediately upon registration. Configure hardware state (muxes, dividers) before calling `clk_hw_register()`.
*   **Error Unwinding:** Use reverse-order cleanup. If `clk_hw_register` fails, release all previously allocated resources (MMIO, IRQs, parent references).
*   **Bootloader State:** Verify if a clock is already enabled before applying default configurations to prevent disabling critical system clocks (e.g., console clocks).

## 4. Error Handling, Resource Recovery & Boundary Validation

*   **IS_ERR vs NULL:** Always use `IS_ERR()` to validate return values from `clk_get()` or `devm_clk_get()`. `NULL` is not a failure indicator.
*   **Resource Leaks:** Do not manually free resources registered with `devm_` helpers.
*   **Arithmetic Integrity:** Use `u64` for rate calculations (e.g., `clk_fractional_divider_general_approximation`) to prevent integer overflow during intermediate multiplications.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **Register Configuration:**
    *   Verify mask and value order in `regmap_update_bits()` or `readl_relaxed`/`writel_relaxed` sequences.
    *   Use `usleep_range()` or `ndelay()` for hardware-required delays; do not rely on implicit bus latency.
*   **Clock Flags:**
    *   `CLK_IS_CRITICAL`: Use sparingly; it prevents gating, which may break power management.
    *   `CLK_SET_RATE_PARENT`: Ensure the parent clock supports rate changes to avoid propagation failures.
*   **False-Positive Guards:**
    *   **Dummy Clocks:** Provide dummy clock entries for missing DT nodes to maintain compatibility.
    *   **Identifiers:** Prefer DT-based index lookups over string-based name matching for clock parents to ensure uniqueness.

## 6. Power Management & Hardware State

*   **Runtime PM:** Use `pm_runtime_get_sync()` and `pm_runtime_put()` within `clk_ops` to manage power domain state.
*   **State Preservation:** If hardware loses register context during power collapse, re-apply configuration upon resume or utilize `RETAIN_FF_ENABLE` if supported by the SoC power controller.
