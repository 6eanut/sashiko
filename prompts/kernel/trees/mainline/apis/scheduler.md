# Task State, Wakeup, and Migration API (`include/linux/sched.h`)

## Voluntary Sleep and Wakeup Ordering (`set_current_state`, `schedule`, `wake_up_process`)

- **Set task state before checking the sleep condition:** `set_current_state(TASK_UNINTERRUPTIBLE)` (or `TASK_INTERRUPTIBLE`) issues `smp_store_mb()`, ordering the `p->__state` write before the subsequent condition load. Checking the condition before calling `set_current_state()` allows a concurrent `wake_up_process()` / `try_to_wake_up()` to fire in the window between the check and the state store, losing the wakeup and sleeping forever:
  ```c
  set_current_state(TASK_UNINTERRUPTIBLE);
  if (!condition)
      schedule();
  __set_current_state(TASK_RUNNING);
  ```
- **`wake_up_process(p)`:** Invokes `try_to_wake_up(p, TASK_NORMAL, 0)` and is safe to call regardless of whether `p` is currently sleeping (returning `0` if `p` was already running).
- **Preemption and lock rules for `schedule()`:** Never call `schedule()` (or any sleeping helper) with preemption disabled (`preempt_disable()`) or while holding a spinlock, `raw_spinlock_t`, or RCU read lock.

## CPU Affinity and Migration Control (`migrate_disable`, `set_cpus_allowed_ptr`, `kthread_bind`)

- **`migrate_disable()` / `migrate_enable()`:** Pins the current task to its current CPU across the region without disabling preemption or blocking; tasks inside a `migrate_disable()` section may still be preempted and may sleep on `spinlock_t` (under `PREEMPT_RT`) or mutexes. Check `is_migration_disabled(p)` before attempting to migrate a task.
- **`set_cpus_allowed_ptr(p, new_mask)` and `kthread_bind(p, cpu)`:** `kthread_bind()` may only be called before the kthread is first woken up. Any task migration must respect `cpumask_subset()` against active/online CPUs.
