# Power Management & Runtime PM API (`include/linux/pm.h`, `include/linux/pm_runtime.h`, `include/linux/suspend.h`)

## Runtime PM Return Value Contracts (`drivers/base/power/runtime.c`)

- **`__pm_runtime_suspend()` (`pm_runtime_suspend()`, `pm_runtime_autosuspend()`, `pm_runtime_put_sync_suspend()`, `pm_runtime_put_sync_autosuspend()`, `pm_runtime_put_autosuspend()`):**
  - Returns `1` (a non-error success code) when the device was already `RPM_SUSPENDED`.
  - Returns `0` on successful suspension or queued async request.
  - Returns negative errno (`-EAGAIN`, `-EBUSY`, `-EACCES`, etc.) on failure.
- **`__pm_runtime_idle()` (`pm_runtime_idle()`, `pm_request_idle()`, `pm_runtime_put()`, `pm_runtime_put_sync()`):**
  - `pm_runtime_idle()` and `pm_request_idle()` (without `RPM_GET_PUT`) never return `1`—if the device is already `RPM_SUSPENDED`, `rpm_idle()` returns `-EAGAIN`.
  - `pm_runtime_put()` and `pm_runtime_put_sync()` (with `RPM_GET_PUT`) can return `1` when the device is already `RPM_SUSPENDED`, or `0` on success.
  - Concurrent `pm_runtime_get()` or child device resumes can legitimately cause `rpm_idle()` / `rpm_suspend()` to return `-EAGAIN`, `-EBUSY`, or `-EINPROGRESS` under `dev->power.lock`.
- **`__pm_runtime_resume()` (`pm_runtime_resume()`, `pm_request_resume()`, `pm_runtime_get_sync()`, `pm_runtime_get()`):**
  - Returns `1` when the device was already `RPM_ACTIVE`, `0` on success, or negative errno on failure. Prefer `pm_runtime_resume_and_get(dev)` when a negative return should automatically drop the usage reference.

## Synchronous vs Asynchronous Runtime PM Put

- `pm_runtime_put(dev)` queues an asynchronous idle request (`RPM_ASYNC`), which can be cancelled by `pm_runtime_disable()` (`__pm_runtime_barrier()`) during device teardown before the device actually suspends.
- Use `pm_runtime_put_sync(dev)` (or `pm_runtime_put_sync_suspend(dev)`) when followed immediately by `device_del()`, `device_unregister()`, `auxiliary_device_delete()`, or `pm_runtime_disable()`, or when hardware ordering requires synchronous suspension.

## Runtime PM in Interrupt Handlers

- `pm_runtime_get_noresume(dev)` only increments `power.usage_count` atomically without checking whether the device is `RPM_ACTIVE`. Calling `pm_runtime_get_noresume()` in an IRQ handler allows register reads on a powered-off (`RPM_SUSPENDED`) device.
- In IRQ handlers (especially `IRQF_SHARED`), use `pm_runtime_get_if_active(dev)` (`> 0` means the device is `RPM_ACTIVE` and the usage count was incremented; `<= 0` means not active, so return `IRQ_NONE`), and call `synchronize_irq()` in the driver's runtime suspend callback before cutting power on `IRQF_SHARED` lines.

## Hibernation `thaw()` and Hybrid Sleep (`pm_hibernate_is_recovering` / `pm_hibernation_mode_is_suspend`)

- `pm_hibernate_is_recovering()` returns `true` only when hibernation image creation failed (`PM_EVENT_RECOVER`).
- In hybrid sleep (hibernation image written, followed by S3/s2idle suspend rather than poweroff), waking from suspend calls `.thaw()` with `pm_hibernate_is_recovering() == false` and `pm_hibernation_mode_is_suspend() == true`.
- Any `.thaw()` callback that skips device resume after image creation must check **both** predicates:
  ```c
  if (!pm_hibernate_is_recovering() && !pm_hibernation_mode_is_suspend())
      return 0;
  ```

## Conditional Compilation Wrappers (`pm_ptr` and `pm_sleep_ptr`)

- Wrap sleep callbacks (`.suspend`, `.resume`, `.freeze`, `.thaw`, `.poweroff`, `.restore`) in `struct dev_pm_ops` with `pm_sleep_ptr()` (`CONFIG_PM_SLEEP`), and wrap the `.driver.pm` pointer (and runtime PM callbacks) with `pm_ptr()` (`CONFIG_PM`), so unused callbacks are dead-code eliminated without `#ifdef` guards or unused-function warnings.
