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
* **Wait-Type Nesting (`CONFIG_PROVE_RAW_LOCK_NESTING=y`)**: Inner lock wait type must be $\le$ outer lock wait type on both RT and non-RT kernels (`!IS_ENABLED(CONFIG_PREEMPT_RT)` does *not* silence lockdep).
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

## Memory Ordering and Reference-Budget UAF Verification

* **Publish / Subscribe Ordering**: Use `smp_store_release(&ptr, data)` (or `rcu_assign_pointer()`) after initializing `data->fields` to publish a pointer, paired with `smp_load_acquire(&ptr)` (or `rcu_dereference()`) on the consumer. Plain `atomic_read()` and `atomic_set()` are relaxed (unordered), whereas value-returning RMW atomics (`atomic_cmpxchg()`, `atomic_add_return()`) provide full barriers.
* **Reference-Budget Check Before Reporting UAF**:
  * Before reporting a race or use-after-free across async work, RCU callbacks, or workqueues, trace every pin keeping the object alive (refcount, held lock, open RCU read-side section, or synchronous drain before free).
  * Returning pooled/permanently-mapped storage to a pool is a logical free: writes after logical free are always bugs (corrupting the next owner), while reads after logical free are bugs when the stale value drives control flow or state.
* **Lock Drop & Re-Acquire**: Dropping and re-acquiring a lock invalidates all state validated before the drop; pointers, flags, and conditions must be re-checked after re-acquisition.
