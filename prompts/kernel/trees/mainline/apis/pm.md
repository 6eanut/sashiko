# Power Management & Runtime PM API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for the Linux Runtime PM and System Sleep APIs. Adherence to these patterns is mandatory for preventing concurrency, reference-counting, and state-machine bugs.

## 1. Runtime PM Return Value Contracts (`drivers/base/power/runtime.c`)

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

## 2. Synchronous vs. Asynchronous Runtime PM

- `pm_runtime_put(dev)` queues an asynchronous idle request (`RPM_ASYNC`), which can be cancelled by `pm_runtime_disable()` (`__pm_runtime_barrier()`) during device teardown before the device actually suspends.
- Use `pm_runtime_put_sync(dev)` (or `pm_runtime_put_sync_suspend(dev)`) when followed immediately by `device_del()`, `device_unregister()`, `auxiliary_device_delete()`, or `pm_runtime_disable()`, or when hardware ordering requires synchronous suspension.

## 3. Runtime PM in Interrupt Handlers

- **`pm_runtime_get_noresume()`**: Only increments `power.usage_count` atomically without checking whether the device is `RPM_ACTIVE`. Calling `pm_runtime_get_noresume()` in an IRQ handler allows register reads on a powered-off (`RPM_SUSPENDED`) device.
- **Mandatory:** In IRQ handlers (especially `IRQF_SHARED`), use `pm_runtime_get_if_active(dev)` (`> 0` means the device is `RPM_ACTIVE` and the usage count was incremented; `<= 0` means not active, so return `IRQ_NONE`), and call `synchronize_irq()` in the driver's runtime suspend callback before cutting power on `IRQF_SHARED` lines.

## 4. Hibernation, Hybrid Sleep, and State Predicates

- `pm_hibernate_is_recovering()` returns `true` only when hibernation image creation failed (`PM_EVENT_RECOVER`).
- **`pm_hibernation_mode_is_suspend()`**: Returns `true` when the system is configured for hybrid sleep (hibernation image written, followed by S3/s2idle suspend).
- In hybrid sleep, waking from suspend calls `.thaw()` with `pm_hibernate_is_recovering() == false` and `pm_hibernation_mode_is_suspend() == true`.
- **Invariant:** Any `.thaw()` callback that skips device resume after image creation must check both predicates:
  ```c
  if (!pm_hibernate_is_recovering() && !pm_hibernation_mode_is_suspend())
      return 0;
  ```

## 5. Conditional Compilation and Managed APIs

- **Wrappers:** Wrap sleep callbacks (`.suspend`, `.resume`, `.freeze`, `.thaw`, `.poweroff`, `.restore`) in `struct dev_pm_ops` with `pm_sleep_ptr()` (`CONFIG_PM_SLEEP`), and wrap the `.driver.pm` pointer (and runtime PM callbacks) with `pm_ptr()` (`CONFIG_PM`), so unused callbacks are dead-code eliminated without `#ifdef` guards or unused-function warnings.
- **Managed APIs:** Use `devm_pm_runtime_enable()` to automate cleanup during driver unbind, reducing the risk of manual teardown errors.

## 6. High-Frequency Bug Patterns (Review Invariants)

1.  **The "Unbalanced Put" Guard:** Every `pm_runtime_get_sync()` must have a corresponding `pm_runtime_put()` on *every* exit path, including error labels.
2.  **The "Probe/Remove" Symmetry:** If `pm_runtime_enable()` is called in `probe()`, `pm_runtime_disable()` must be called in `remove()`.
3.  **The "Autosuspend" Teardown:** If `pm_runtime_use_autosuspend()` was called, `pm_runtime_dont_use_autosuspend()` must be called during teardown.
4.  **The "Memory Allocation" Guard:** If runtime PM callbacks perform memory allocation, call `pm_runtime_set_memalloc_noio(dev, true)` during probe to prevent deadlocks.
5.  **The "Barrier" Guard:** During `remove()`, use `pm_runtime_barrier()` to ensure all pending asynchronous requests are flushed before proceeding with hardware teardown.
6.  **The "No-Resume" Constraint:** `pm_runtime_get_noresume(dev)` is strictly for contexts where the device is known to be active or where the caller will handle the power state manually. Never use it to "wake" a device.
7.  **The "IRQ Safety" Guard:** If a device is `pm_runtime_irq_safe()`, it cannot be suspended/resumed in a way that requires sleeping. Ensure the driver does not perform blocking I/O in its runtime callbacks if this flag is set.
8.  **The "Reference Handoff":** When passing a device to an async worker, ensure the worker holds a reference via `pm_runtime_get_sync()` to prevent the device from suspending while the worker is active.
9.  **The "Partial Failure" Rule:** If `pm_runtime_enable()` fails or is not called, ensure that the driver does not attempt to call `pm_runtime_put()` or `pm_runtime_suspend()` on the device, as this will trigger a kernel warning.
10. **The "Locking Hierarchy":** `dev->power.lock` is a spinlock. Never perform sleeping operations (e.g., `mutex_lock`, `msleep`, `GFP_KERNEL` allocations) inside runtime PM callbacks unless `pm_runtime_set_memalloc_noio()` has been correctly configured.
