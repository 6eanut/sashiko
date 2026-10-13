# Scheduler Internals (`kernel/sched/`)

## Runqueue Locking and Task State Consistency (`kernel/sched/core.c`)

- **`raw_spinlock_t` and multi-RQ ordering:** Runqueue locks (`rq->__lock`) and `p->pi_lock` are `raw_spinlock_t`. Use `double_rq_lock(rq1, rq2)` / `double_rq_unlock(rq1, rq2)` (which order via `rq_order_less()`) when locking two runqueues, and assert ownership with `lockdep_assert_rq_held(rq)`.
- **`task_rq(p)` stability:** `task_rq(p)` reads `cpu_rq(task_cpu(p))`. Unless `p->pi_lock` or the task's `rq` lock is held (via `task_rq_lock()`), `p` can migrate immediately after `task_cpu(p)` is read. Ensure `set_task_cpu()` is only called during migration with the required locks held.
- **Runqueue unlock invariant:** Never drop `rq->__lock` while `p->on_rq`, RB-tree membership, and `p->__state` disagree:
  - In RB-tree with `p->on_rq == 0`: `try_to_wake_up()` calls `activate_task()` again, corrupting the RB-tree via double enqueue.
  - `p->on_rq == 1` but not in RB-tree: `try_to_wake_up()` skips enqueue while `pick_next_task()` never finds `p`.
  - `p->__state == TASK_RUNNING` with `p->on_rq == 0` (and not running): `try_to_wake_up()` returns early and the task hangs permanently.

## Fair Scheduler (EEVDF), PELT, and CFS Bandwidth (`kernel/sched/fair.c`)

- **EEVDF (`__pick_eevdf()`, `place_entity()`):** Selects the eligible entity (`vruntime_eligible()`, lag $\ge 0$) with the earliest virtual deadline `vd = vruntime + calc_delta_fair(slice, se)`. The CFS RB-tree is keyed by `se->deadline` and augmented with `se->min_vruntime` for $O(\log n)$ subtree eligibility pruning.
- **Entity state fields (`struct sched_entity`):**
  - `se->vruntime`: Must be normalized across CPU migrations.
  - `se->vlag` / `vprot`: Preserves lag (`V - vruntime`) across dequeue/enqueue in `place_entity()`; aliased with `vprot` while `se->on_rq && cfs_rq->curr == se`.
  - Delayed dequeue (`sched_delayed`): A logically dequeued entity can remain `se->on_rq == 1` until its positive lag is consumed.
- **PELT (`update_load_avg()`):** Call `update_load_avg()` *before* any entity state change (enqueue, dequeue, or `DO_DETACH` / `DO_ATTACH` migration) to keep hierarchical load averages consistent.
- **CFS bandwidth:** Group throttling (`cfs_rq->runtime_remaining`) must symmetrically dequeue on throttle and enqueue on unthrottle across the hierarchy.

## RT, Deadline, Priority Inheritance, `sched_ext`, and CPU Hotplug

- **Real-Time (`SCHED_FIFO` / `SCHED_RR`):** Internal priority is inverted (`MAX_RT_PRIO - 1 - rt_priority`), throttled by `sched_rt_runtime_us` / `sched_rt_period_us`.
- **Deadline (`SCHED_DEADLINE`):** Enforces `runtime <= deadline <= period` and root-domain admission control (`dl_bw_cpus()`) via Constant Bandwidth Server (CBS).
- **Priority inheritance (`pi_lock`):** RT-mutex PI chain traversal and updates must be serialized under `pi_lock` and checked for deadlock cycles.
- **`sched_ext` (`SCHED_EXT`, `kernel/sched/ext.c`):** Tracks task ownership via atomic `p->scx.ops_state` (`NONE -> QUEUEING -> QUEUED -> DISPATCHING`); dispatch queues (DSQs) cannot mix FIFO and PRIQ (`vtime`) ordering on the same DSQ.
- **CPU hotplug (`sched_cpu_deactivate()` / `sched_cpu_dying()`):** Per-CPU scheduler timers (such as `dl_server`'s `dl_timer` stopped by `dl_server_stop()` in `sched_cpu_dying()`) must be cancelled before the CPU goes offline and gated by `cpu_online()` in arming paths (`dl_server_start()`), or firing after `cpu_present_mask` removal triggers warnings in `cpudl_set()`.
