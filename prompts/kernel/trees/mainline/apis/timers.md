# Kernel Timers (`timer_list`, `hrtimer`, `delayed_work`) (`include/linux/timer.h`, `include/linux/hrtimer.h`)

## Timer Types, Modern Setup APIs, and Execution Contexts

| Timer Type | Setup Helper | Cancel / Teardown Helpers | Callback Context | Sleeping / `GFP_KERNEL` |
| :--- | :--- | :--- | :--- | :--- |
| `struct timer_list` | `timer_setup(t, cb, flags)` / `DEFINE_TIMER()` | `timer_delete()`, `timer_delete_sync()`, `timer_shutdown_sync()` | Softirq | Prohibited (`GFP_ATOMIC` only) |
| `struct hrtimer` | `hrtimer_setup(t, cb, clk, mode)` | `hrtimer_try_to_cancel()`, `hrtimer_cancel()` | Hardirq (default) or Softirq (`HRTIMER_MODE_SOFT`) | Prohibited (`spin_lock_bh` also prohibited in hardirq mode) |
| `struct delayed_work` | `INIT_DELAYED_WORK(dw, cb)` | `cancel_delayed_work()`, `cancel_delayed_work_sync()`, `disable_delayed_work_sync()` | Process (workqueue) | Allowed |

- **Removed legacy APIs:** `del_timer()`, `del_timer_sync()`, `setup_timer()`, `init_timer()`, and `hrtimer_init()` have been removed from the kernel; use `timer_delete()`, `timer_delete_sync()`, `timer_shutdown_sync()`, `timer_setup()`, and `hrtimer_setup()`.
- **`hrtimer` callback return values:** Must return `HRTIMER_NORESTART` or (after advancing the expiry with `hrtimer_forward_now()`) `HRTIMER_RESTART`.

## Synchronous Cancellation, Teardown, and Locking Hazards

- **Freeing timer-containing structs:** Always use `timer_shutdown_sync(&obj->timer)` (or `timer_delete_sync()` / `hrtimer_cancel()` / `cancel_delayed_work_sync()`) before `kfree(obj)`. `timer_delete()` only dequeues a pending timer and returns while the callback is still executing on another CPU. `timer_shutdown_sync()` both waits for in-flight callbacks and permanently prevents future `mod_timer()` / `add_timer()` calls from re-arming the timer during teardown.
- **Deadlock with callback locks:** Never call `timer_delete_sync()`, `timer_shutdown_sync()`, or `hrtimer_cancel()` from inside the timer's own callback, or while holding a lock that the timer callback also acquires.
- **`timer_pending()` is not synchronization:** `!timer_pending(t)` returns `true` while the timer's callback is actively running on another CPU; never use `timer_pending()` to guard object teardown.
