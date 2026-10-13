# Generic Power Domains (`drivers/pmdomain/`, `include/linux/pm_domain.h`)

## 1. `genpd->stay_on`, `sync_state`, and `GENPD_FLAG_NO_STAY_ON` (`drivers/pmdomain/core.c`)

- When `CONFIG_PM_GENERIC_DOMAINS_OF` is enabled and a domain is initialized as powered on (`is_off = false`), `pm_genpd_init()` calls `genpd_set_stay_on()`, setting `genpd->stay_on = true` unless `GENPD_FLAG_NO_STAY_ON` is set in `genpd->flags`.
- While `genpd->stay_on` is `true`, `genpd_power_off()` refuses to power off the domain. `stay_on` is cleared only when `genpd_provider_sync_state()` / `of_genpd_sync_state()` runs after all consumer device links reach `DL_STATE_ACTIVE` (or after `deferred_probe_timeout` when `fw_devlink.sync_state=timeout` is active).
- **`genpd_power_off_unused()` vs `regulator_init_complete()`:** Both run at `late_initcall_sync`. `genpd_power_off_unused()` skips domains with `stay_on == true`, while `regulator_init_complete_work` disables unused regulators after a 30-second delay. On platforms where a regulator supplies a power domain and some DT consumers may never probe, leaving `stay_on` set can cause the supply regulator to be disabled while the power domain is still active. Set `GENPD_FLAG_NO_STAY_ON` on providers that must power off unused domains without waiting for `sync_state` (such as Rockchip, Renesas R-Car/R-Mobile, and Tegra BPMP).

## 2. Platform Bootloader Handover and Default States

- **Early reset before `pm_genpd_init()`:** Bootloader handover workarounds (such as splash-screen or pre-enabled domain resets) must be gated on the affected platform (`IS_ENABLED()`, `of_device_is_compatible()`, or DT properties) and executed in probe *before* `pm_genpd_init()` registers the domain on `gpd_list`.
- **Targeted vs provider-wide power-off:** `of_genpd_sync_state()` iterates and powers off *all* domains belonging to a provider; when only specific domains should be powered off, use targeted driver helpers (such as `exynos_pd_power_off()`).
- **Vendor default-off flags:** Preserve platform default-off mechanisms (such as MediaTek `MTK_SCPD_KEEP_DEFAULT_OFF` passing `is_off = true` to `pm_genpd_init()`, or `GENPD_FLAG_NO_STAY_ON`) when modifying core genpd initialization or power-off timing.

## 3. Subsystem Architecture, Locking, and Lifetimes

- **Locking Abstraction:** `genpd` uses `genpd_lock_ops` to support both sleepable and atomic domains.
    - **`GENPD_FLAG_IRQ_SAFE`:** If set, `power_on()`/`power_off()` callbacks must not sleep. Mandatory if any attached device has `pm_runtime_is_irq_safe()` set.
    - **Hierarchy:** If a master domain is `IRQ_SAFE`, all subdomains must be `IRQ_SAFE`.
- **Locking Hierarchy:** Use `genpd_lock(genpd)`. When traversing the domain tree (master/subdomain), acquire locks in a consistent order (child to parent) to avoid circular dependencies.
- **Atomic vs. Sleepable:**
    - If `GENPD_FLAG_IRQ_SAFE` is **not** set, callbacks may sleep. Do not invoke from `spin_lock` or interrupt context.
    - If `GENPD_FLAG_IRQ_SAFE` **is** set, callbacks are invoked in atomic context. No blocking operations (e.g., `mutex_lock`, `msleep`, `kmalloc(GFP_KERNEL)`) are permitted.
- **Object Lifecycle:** `pm_genpd_init()` registers the domain. It must be called *after* setting flags but *before* any device attachment. Never manually free a `generic_pm_domain` registered in `gpd_list`.

## 4. Initialization, Registration, and Error Handling

- **Premature Publication:** A domain is visible to the system as soon as `pm_genpd_init()` completes. Ensure all hardware-specific fields (e.g., `power_on`, `power_off`, `attach_dev`) are initialized *before* this call.
- **Error Unwind:** Use `devm_` managed resources. If manual registration is required, ensure `pm_genpd_remove()` is called on all error paths.
- **Unbalanced Refcounts:** Every `pm_genpd_add_subdomain()` must have a corresponding `pm_genpd_remove_subdomain()`.
- **Optional Resources:** When using `dev_pm_domain_attach_list()`, check if the returned list is empty or if specific domains are missing; do not treat the absence of an optional power domain as a fatal probe error.

## 5. High-Frequency Bug Patterns

- **Use-after-free in `sync_state`:** Ensure all `genpd` providers are fully initialized before `sync_state` triggers. If a driver is built as a module, ensure it does not rely on `sync_state` for critical hardware configuration.
- **Unbalanced Runtime PM:** If `pm_runtime_enable()` was called, `pm_runtime_disable()` must be called in the error path.
- **Stale State:** Always verify the return value of `genpd->power_on()`. If it fails, the domain status must remain `GENPD_STATE_OFF`.
- **`devm` Ordering:** Ensure that any work items or timers registered via `devm` are explicitly cancelled before the driver's private data is freed.

## 6. Power Management & Runtime Transitions

- **Hardware State:** Before accessing registers, verify the domain is powered on (`genpd_status_on(genpd)`).
- **System Suspend:** During `suspend`, `genpd` may power off domains. Drivers attached to these domains must not attempt register access in `suspend` callbacks unless they have requested the domain to stay on via `pm_runtime_get_sync()`.
- **Wakeup Path:** If `GENPD_FLAG_ACTIVE_WAKEUP` is set, the domain will not power off if any attached device is configured for system wakeup. Ensure this flag is set only if the hardware supports it.
- **CPU Domains:** When `GENPD_FLAG_CPU_DOMAIN` is set, the backend must implement the "last-man-standing" algorithm. Ensure that CPU-specific power-down sequences are atomic and do not conflict with runtime PM transitions of non-CPU devices attached to the same domain.
- **Always-On Domains:** `GENPD_FLAG_ALWAYS_ON` and `GENPD_FLAG_RPM_ALWAYS_ON` bypass standard power-off logic. Use these only for critical infrastructure (e.g., system controllers) where powering off would cause an immediate kernel panic or hardware hang.
- **Validation of Hardware State:** Always verify that the hardware power domain state matches the software `genpd->status` after a power transition. If a domain fails to power on, the driver must propagate the error to prevent subsequent register access to unpowered hardware.
- **Atomic Context Constraints:** When `GENPD_FLAG_IRQ_SAFE` is set, ensure that no `GFP_KERNEL` allocations occur within the `power_on` or `power_off` callbacks. Use `GFP_ATOMIC` or pre-allocated memory pools if necessary.
- **Device Link Management:** When using `PD_FLAG_NO_DEV_LINK`, the driver assumes full responsibility for managing the power dependency between the supplier and consumer. Ensure that the consumer's runtime PM state is correctly synchronized with the supplier's power domain state.
- **Registration Invariants:** Always verify that `pm_genpd_init` is called only after all `genpd` flags are finalized. Modifying `genpd->flags` after registration is a race condition that can lead to inconsistent power-off logic.
- **Callback Safety:** Ensure that `power_on` and `power_off` callbacks are re-entrant if the domain supports multiple concurrent device attachments or nested subdomains. Use `genpd_lock` to protect internal state transitions.
