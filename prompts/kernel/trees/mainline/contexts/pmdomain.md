# Generic Power Domains (`drivers/pmdomain/`, `include/linux/pm_domain.h`)

## `genpd->stay_on`, `sync_state`, and `GENPD_FLAG_NO_STAY_ON` (`drivers/pmdomain/core.c`)

- When `CONFIG_PM_GENERIC_DOMAINS_OF` is enabled and a domain is initialized as powered on (`is_off = false`), `pm_genpd_init()` calls `genpd_set_stay_on()`, setting `genpd->stay_on = true` unless `GENPD_FLAG_NO_STAY_ON` is set in `genpd->flags`.
- While `genpd->stay_on` is `true`, `genpd_power_off()` refuses to power off the domain. `stay_on` is cleared only when `genpd_provider_sync_state()` / `of_genpd_sync_state()` runs after all consumer device links reach `DL_STATE_ACTIVE` (or after `deferred_probe_timeout` when `fw_devlink.sync_state=timeout` is active).
- **`genpd_power_off_unused()` vs `regulator_init_complete()`:** Both run at `late_initcall_sync`. `genpd_power_off_unused()` skips domains with `stay_on == true`, while `regulator_init_complete_work` disables unused regulators after a 30-second delay. On platforms where a regulator supplies a power domain and some DT consumers may never probe, leaving `stay_on` set can cause the supply regulator to be disabled while the power domain is still active. Set `GENPD_FLAG_NO_STAY_ON` on providers that must power off unused domains without waiting for `sync_state` (such as Rockchip, Renesas R-Car/R-Mobile, and Tegra BPMP).

## Platform Bootloader Handover and Default States

- **Early reset before `pm_genpd_init()`:** Bootloader handover workarounds (such as splash-screen or pre-enabled domain resets) must be gated on the affected platform (`IS_ENABLED()`, `of_device_is_compatible()`, or DT properties) and executed in probe *before* `pm_genpd_init()` registers the domain on `gpd_list`.
- **Targeted vs provider-wide power-off:** `of_genpd_sync_state()` iterates and powers off *all* domains belonging to a provider; when only specific domains should be powered off, use targeted driver helpers (such as `exynos_pd_power_off()`).
- **Vendor default-off flags:** Preserve platform default-off mechanisms (such as MediaTek `MTK_SCPD_KEEP_DEFAULT_OFF` passing `is_off = true` to `pm_genpd_init()`, or `GENPD_FLAG_NO_STAY_ON`) when modifying core genpd initialization or power-off timing.
