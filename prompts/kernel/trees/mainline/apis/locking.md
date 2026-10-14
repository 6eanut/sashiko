# Kernel Locking, Concurrency, and Memory Ordering APIs

## Lock Context and `PREEMPT_RT` Wait-Type Compatibility

| Lock Variant | Wait Type | vs Process | vs Softirq | vs Hardirq | Sleeps |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `raw_spin_lock` | `LD_WAIT_SPIN` | Yes | No | No | No |
| `raw_spin_lock_irqsave` | `LD_WAIT_SPIN` | Yes | Yes | Yes | No |
| `spin_lock` | `LD_WAIT_CONFIG` | Yes | No | No | No (non-RT) / Yes (RT) |
| `spin_lock_bh` | `LD_WAIT_CONFIG` | Yes | Yes | No | No (non-RT) / Yes (RT) |
| `spin_lock_irq` / `spin_lock_irqsave` | `LD_WAIT_CONFIG` | Yes | Yes | Yes | No (non-RT) / Yes (RT) |
| `local_lock` / `local_trylock` | `LD_WAIT_CONFIG` | Per-CPU | No | No | No (non-RT) / Yes (RT) |
| `mutex` / `rwsem` | `LD_WAIT_SLEEP` | Yes | No | No | Yes |

* **Context Masking**: If a lock is shared with hardirq context, all process/softirq acquisitions must mask IRQs (`spin_lock_irqsave()` / `raw_spin_lock_irqsave()`). If shared with softirq context, process acquisitions must use `spin_lock_bh()`.
* **Wait-Type Nesting (`CONFIG_PROVE_RAW_LOCK_NESTING=y`)**: Inner lock wait type must be $\le$ outer lock wait type on both RT and non-RT kernels.
  * Holding `raw_spinlock_t` **forbids** acquiring `spinlock_t`, `local_lock`, `local_trylock`, `mutex`, or `rwsem` (unless overridden via `DEFINE_WAIT_OVERRIDE_MAP(map, LD_WAIT_CONFIG)` + `lock_map_acquire_try(&map)`).
  * Holding `spinlock_t` or `local_lock` **forbids** acquiring `mutex` or `rwsem`, or calling `GFP_KERNEL` allocations (`__GFP_DIRECT_RECLAIM`).
* **`PREEMPT_RT` Specifics**:
  * On `PREEMPT_RT`, `spinlock_t` is a sleepable `rt_mutex` (`spin_lock_irq()` does not disable hardware IRQs on RT), and `local_lock` maps to a per-CPU `spinlock_t` + `migrate_disable()`.
  * Guard `local_lock` or sleep-sensitive paths with `!preemptible()`, never `in_nmi() || in_hardirq()` (which misses `preempt_disable()` sections and BPF/tracepoint contexts).

## Preemption, Migration, and CPU Hotplug (`include/linux/cpu.h`)

* Neither `preempt_disable()`, `migrate_disable()`, nor `local_irq_disable()` prevents CPU hot-unplug teardown.
* When code switches from `get_cpu_ptr()` to `raw_cpu_ptr()` to call sleepable APIs on per-CPU state allocated in `cpuhp_setup_state()` callbacks, serialize against CPU hotplug via `cpus_read_lock()` (sleepable `percpu_rw_semaphore`), a per-CPU mutex synchronized with the hotplug teardown callback, or a refcount.

## Seqlocks, Lock Pinning, and Reclaim Ordering

* **Seqlocks (`include/linux/seqlock.h`)**:
  * Read-side sections (`read_seqbegin()` .. `read_seqretry()`) may re-execute arbitrarily if a writer overlaps: they must have no side effects (no allocations, shared writes, or I/O) and must use `rcu_dereference()` when following pointers that writers can replace.
  * Write-side `write_seqcount_begin()` and `write_seqcount_end()` must be strictly balanced on all paths.
* **Lockdep Lock Pinning (`lockdep_pin_lock` / `lockdep_unpin_lock`)**:
  * For `ww_mutex`-based locks (such as `dma_resv`) where multiple instances share a single `held_lock` entry via `nest_lock` reference folding, `match_held_lock()` falls back to lock-class matching. Pinning one `dma_resv` and then locking/unlocking another `dma_resv` instance in the same `nest_lock` scope (e.g., inside `ttm_bo_validate()`) triggers a false/fatal "releasing a pinned lock" lockdep warning.
* **Reclaim-Reachable Lock Ordering**: Any lock held across a `GFP_KERNEL` (`__GFP_FS` / `__GFP_IO`) allocation orders above filesystem/block reclaim locks (`->writepages`, shrinkers, `folio_lock()`). Use `memalloc_nofs_save()` / `memalloc_noio_save()` when holding locks that reclaim can acquire.

## Memory Ordering and Atomic Operations

* **Atomic API (`include/linux/atomic.h`)**:
  * `atomic_read()` and `atomic_set()` are relaxed (unordered).
  * `atomic_cmpxchg()` and `atomic_add_return()` provide full memory barriers.
* **Publish / Subscribe Ordering**: Use `smp_store_release(&ptr, data)` (or `rcu_assign_pointer()`) after initializing `data->fields` to publish a pointer, paired with `smp_load_acquire(&ptr)` (or `rcu_dereference()`) on the consumer.
* **Reference-Budget Check Before Reporting UAF**:
  * Before reporting a race or use-after-free across async work, RCU callbacks, or workqueues, trace every pin keeping the object alive (refcount, held lock, open RCU read-side section, or synchronous drain before free).
  * Report only if the free can happen while the using side still holds a live pointer with no pin. It is not a use-after-free when the stored pointer is itself counted (its holder took a reference when saving it, so the free is merely deferred), or when the freeing side waits for the user to finish before freeing.
  * Returning pooled/permanently-mapped storage to a pool is a logical free: writes after logical free are always bugs (corrupting the next owner), while reads after logical free are bugs when the stale value drives control flow or state.
  * Record the result as one line: `budget: acquires=[...], releases=[...], refs_live_at_free=..., free_reachable_without_pin=[proof | no]; logical_free=[storage-free | pool-return], use=[read-driving-decision | write]`
* **Lock Drop & Re-Acquire**: Dropping and re-acquiring a lock invalidates all state validated before the drop; pointers, flags, and conditions must be re-checked after re-acquisition.
* **Synchronous Cancellation**: During teardown, ensure all asynchronous work (timers, workqueues) is cancelled via `cancel_work_sync()` or `cancel_delayed_work_sync()` *before* the object is freed.

## Object Lifecycle and Ownership Rules

* **Ownership Transfer**: When a lookup yields a shared object, the caller must determine if they own a reference (e.g., `refcount_inc()`) or if the reference is bound to a held lock.
* **Rollback**: If initialization fails, decrement usage counters in the exact reverse order of acquisition.
* **Async Handoff**: When passing an object to an asynchronous worker, the caller must duplicate the reference (e.g., `kref_get()`) before scheduling the work to ensure the object persists until the worker executes.
* **Iteration**: Iteration helpers (e.g., `list_for_each_entry_rcu`) require an RCU read-side section; if the loop terminates early, ensure no dangling references remain.
* **Teardown**: Ensure all cached pointers are invalidated before final deallocation. The parent framework is responsible for destruction unless the child driver explicitly takes ownership.
* **Signal Abort**: If an operation is interrupted by a signal, the abort path must balance all usage counters acquired prior to the interruption.

## Error Handling and Partial-Failure Recovery

* **Pointer Sentinels**: Use `IS_ERR()` and `PTR_ERR()` to check for error codes embedded in pointers. Never check for `NULL` if the API returns error pointers.
* **Partial Success**: If a multi-step configuration fails, the recovery path must restore hardware/software state to the previous condition.
* **Resource Cleanup**: If a validation step fails after resource acquisition, release the resource using a specific error-handling label (e.g., `goto out_free_res`).
* **Error Propagation**: Propagate error codes from nested hardware interactions back to the user-space boundary without masking the original failure context.
