# Kernel Timers (`timer_list`, `hrtimer`, `delayed_work`) (`include/linux/timer.h`, `include/linux/hrtimer.h`)

## 1. Timer Types, Modern Setup APIs, and Execution Contexts

| Timer Type | Setup Helper | Cancel / Teardown Helpers | Callback Context | Sleeping / `GFP_KERNEL` |
| :--- | :--- | :--- | :--- | :--- |
| `struct timer_list` | `timer_setup(t, cb, flags)` / `DEFINE_TIMER()` | `timer_delete()`, `timer_delete_sync()`, `timer_shutdown_sync()` | Softirq | Prohibited (`GFP_ATOMIC` only) |
| `struct hrtimer` | `hrtimer_setup(t, cb, clk, mode)` | `hrtimer_try_to_cancel()`, `hrtimer_cancel()` | Hardirq (default) or Softirq (`HRTIMER_MODE_SOFT`) | Prohibited |
| `struct delayed_work` | `INIT_DELAYED_WORK(dw, cb)` | `cancel_delayed_work()`, `cancel_delayed_work_sync()`, `disable_delayed_work_sync()` | Process (workqueue) | Allowed |

- **Initialization:** Always use `timer_setup()` or `hrtimer_setup()`. Legacy `init_timer()`, `setup_timer()`, and `hrtimer_init()` are removed. Use `DEFINE_TIMER()` for static declarations.
- **Stack Allocation:** For stack-allocated timers, use `timer_setup_on_stack()` or `hrtimer_setup_on_stack()` and invoke `timer_destroy_on_stack()` or `destroy_hrtimer_on_stack()` before the stack frame exits.
- **Blocking Operations:** Never perform blocking operations (e.g., `mutex_lock`, `wait_event`, `msleep`) inside `timer_list` or `hrtimer` callbacks. Offload blocking tasks to a `workqueue` using `schedule_work()`.

## 2. Synchronous Cancellation, Teardown, and Locking Hazards

- **Freeing timer-containing structs:** Always use `timer_shutdown_sync(&obj->timer)` (or `timer_delete_sync()` / `hrtimer_cancel()` / `cancel_delayed_work_sync()`) before `kfree(obj)`. `timer_delete()` only dequeues a pending timer and returns while the callback is still executing on another CPU. `timer_shutdown_sync()` both waits for in-flight callbacks and permanently prevents future `mod_timer()` / `add_timer()` calls from re-arming the timer during teardown.
- **Legacy APIs:** `del_timer()` and `del_timer_sync()` are removed; use `timer_delete()` and `timer_delete_sync()` respectively.
- **Deadlock with callback locks:** Never call `timer_delete_sync()`, `timer_shutdown_sync()`, or `hrtimer_cancel()` from inside the timer's own callback, or while holding a lock that the timer callback also acquires.
- **`spin_lock_bh`:** When accessing timer state from process context, use `spin_lock_bh()` to prevent deadlocks with the softirq-based timer expiry path.
- **`timer_pending()` is not synchronization:** `!timer_pending(t)` returns `true` while the timer's callback is actively running on another CPU; never use `timer_pending()` to guard object teardown.

## 3. Return Values and Expiry Logic

- **`hrtimer` callback return values:** Must return `HRTIMER_NORESTART` or (after advancing the expiry with `hrtimer_forward_now()`) `HRTIMER_RESTART`.
- **Expiry Arithmetic:** Use `ktime_add_ns()` or `jiffies` arithmetic. Never perform raw unsigned arithmetic on time values to avoid underflow/overflow. Always validate that timer intervals are non-negative.

## 4. High-Frequency Bug Patterns & Review Invariants

| Bug Pattern | Prevention Rule |
| :--- | :--- |
| **Use-after-free on teardown** | Always use `_sync` variants before `kfree()`. |
| **Timer re-arming during teardown** | Ensure the event source (e.g., hardware interrupt) is disabled *before* calling `timer_shutdown_sync()`. |
| **Stale timer reuse** | Always validate the timer state or re-initialize with `timer_setup()` before re-arming. |
| **Nested lock deadlock** | Ensure the lock hierarchy is: `Global Lock` -> `Timer Lock`. Never acquire a lock in the callback that is held during `timer_delete_sync()`. |

## 5. Review Checklist for Automated Patch Review

1. **Does the `kfree()` path include a `_sync` timer/work cancellation?** (Mandatory)
2. **Is the timer callback free of `GFP_KERNEL` allocations?** (Mandatory)
3. **If `hrtimer` is used, is `hrtimer_forward_now()` called before `HRTIMER_RESTART`?** (Mandatory)
4. **Is the object initialized before being exposed to the system?** (Check for `timer_setup` before `add_timer`).
5. **Are all `spin_lock` calls in timer-related paths using `_bh` variants to avoid softirq deadlocks?** (Mandatory)
6. **Are all legacy `del_timer` calls replaced with `timer_delete`?** (Mandatory)
7. **Are timer callbacks re-entrant safe?** (Ensure no shared state is modified without proper locking).
8. **Is the monotonic clock used for durations?** (Avoid `CLOCK_REALTIME` for timeouts to prevent NTP-induced jumps).
9. **Are timer inputs clamped?** (Validate that user-provided timeouts are non-negative and within reasonable bounds).
10. **Is the timer destroyed on stack?** (Ensure `timer_destroy_on_stack` is called for all stack-allocated timers).
11. **Are `hrtimer_sleeper` objects properly initialized?** (Use `hrtimer_setup_sleeper_on_stack` for stack-based sleepers).
12. **Is the `timer_list` flag `TIMER_IRQSAFE` used only when necessary?** (Avoid abuse; monitor for performance impact).
13. **Are `workqueue` items properly drained?** (Use `cancel_delayed_work_sync()` to ensure the work item is not running).
14. **Is the `timer_container_of` macro used for type-safe callback access?** (Mandatory for clean struct access).
15. **Are all `hrtimer` modes explicitly defined?** (Use `HRTIMER_MODE_ABS` or `HRTIMER_MODE_REL` as appropriate).
