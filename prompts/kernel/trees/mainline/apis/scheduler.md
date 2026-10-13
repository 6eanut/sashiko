# Task State, Wakeup, and Migration API (`include/linux/sched.h`)

This guide provides authoritative rules for the Linux v7.3-rc5 scheduler API. Adherence to these invariants is mandatory to prevent deadlocks, race conditions, and memory corruption.

## 1. Voluntary Sleep and Wakeup Ordering
*   **Set task state before checking the sleep condition:** `set_current_state(TASK_UNINTERRUPTIBLE)` (or `TASK_INTERRUPTIBLE`) issues `smp_store_mb()`, ordering the `p->__state` write before the subsequent condition load. Checking the condition before calling `set_current_state()` allows a concurrent `wake_up_process()` / `try_to_wake_up()` to fire in the window between the check and the state store, losing the wakeup and sleeping forever:
    ```c
    set_current_state(TASK_UNINTERRUPTIBLE);
    if (!condition)
        schedule();
    __set_current_state(TASK_RUNNING);
    ```
*   **`wake_up_process(p)`:** Invokes `try_to_wake_up(p, TASK_NORMAL, 0)` and is safe to call regardless of whether `p` is currently sleeping (returning `0` if `p` was already running).
*   **Preemption and lock rules for `schedule()`:** Never call `schedule()` (or any sleeping helper) with preemption disabled (`preempt_disable()`) or while holding a spinlock, `raw_spinlock_t`, or RCU read lock.

## 2. CPU Affinity and Migration Control
*   **`migrate_disable()` / `migrate_enable()`:** Pins the current task to its current CPU across the region without disabling preemption or blocking; tasks inside a `migrate_disable()` section may still be preempted and may sleep on `spinlock_t` (under `PREEMPT_RT`) or mutexes. Check `is_migration_disabled(p)` before attempting to migrate a task.
*   **`set_cpus_allowed_ptr(p, new_mask)`:** Must be called with a valid `cpumask_t`. Ensure the mask is not empty and contains at least one online CPU. Always check the return value; if an error occurs, the rollback path must restore the previous state.
*   **`kthread_bind(p, cpu)`:** May only be called before the kthread is first woken up.
*   **`cpumask_subset(mask1, mask2)`:** Any task migration must respect `cpumask_subset()` against active/online CPUs to ensure the target mask is valid within the system topology.

## 3. Execution Context and Atomic Constraints
*   **Sleeping Constraints:** Never call `schedule()` or blocking primitives (e.g., `mutex_lock`, `wait_event`) while holding a `spinlock_t` or `raw_spinlock_t`, or while in an interrupt context (`in_interrupt()`).
*   **Atomic Allocations:** If memory allocation is required while holding a non-sleeping lock, use `GFP_ATOMIC`. Never use `GFP_KERNEL` in atomic contexts.
*   **Per-CPU Access:** Accessing per-CPU variables (e.g., `this_cpu_ptr`) requires that preemption be disabled to prevent the task from migrating to another CPU between the address calculation and the memory access.
*   **RT Invariants:** On `PREEMPT_RT` kernels, `local_bh_disable()` does not disable preemption. If code relies on `local_bh_disable()` to protect per-CPU data, it is broken on RT; use `migrate_disable()` or explicit locking instead.

## 4. High-Frequency Bug Patterns & Review Invariants
*   **CPU Hotplug:** Subsystems using per-CPU data must handle CPU hotplug events. Always validate the CPU index (e.g., `cpu_online(cpu)`) before accessing per-CPU structures. Use `cpuhp_state_add_instance()` to serialize initialization/teardown with the hotplug state machine.
*   **Task Lifecycle:** If a task is initialized (e.g., `kthread_create`), it must be either started via `wake_up_process()` or explicitly cleaned up via `kthread_stop()` to prevent task leaks.
*   **Error Paths:** Does the rollback path overwrite the original error code? Ensure `ret = ...` is preserved.
*   **Context Accounting:** Ensure accounting hooks use the actual execution context rather than the scheduling context when they differ (e.g., workqueue processing).
*   **Re-entrancy:** Does the per-CPU data structure account for re-entrancy or preemption scenarios that might occur within the same execution context?
*   **Shutdown:** Are all background tasks, timers, or IPI-generating mechanisms explicitly disabled or cancelled during the system shutdown sequence?

## 5. Summary Checklist for Reviewers
1.  **Context:** Is `schedule()` called in an atomic context? (Check for `spin_lock`, `preempt_disable`, or `in_interrupt()`).
2.  **Ordering:** Is `set_current_state()` called before the condition check?
3.  **Hotplug:** Does the code handle CPU hotplug? Are per-CPU resources cleaned up on `CPU_DOWN`?
4.  **Migration:** Is `migrate_disable()` used correctly to protect per-CPU access?
5.  **Error Paths:** Does the rollback path overwrite the original error code?
6.  **Validation:** Is the CPU index validated before being used to access per-CPU data structures?
7.  **Synchronization:** Are you holding the appropriate hotplug locks when performing operations that depend on the stability of the online CPU set?
8.  **Initialization:** Is `kthread_bind()` called strictly before the first `wake_up_process()`?
9.  **Affinity:** Does `set_cpus_allowed_ptr()` verify the target mask against `cpu_online_mask`?
