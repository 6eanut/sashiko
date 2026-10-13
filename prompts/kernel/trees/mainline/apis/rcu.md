# Read-Copy-Update (RCU) API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for using the RCU subsystem. RCU is a synchronization mechanism that allows readers to access data concurrently with writers without locks, provided the writer follows the "Remove-Before-Reclaim" protocol.

## 1. Read-Side Critical Sections, Variants, and Pointer Publication

| Variant | Read-Side API | Sleepable | Notes |
| :--- | :--- | :--- | :--- |
| Classic RCU | `rcu_read_lock()` / `rcu_read_unlock()` | No | Preemptible only under `CONFIG_PREEMPT_RCU`; never block/sleep |
| SRCU | `srcu_read_lock()` / `srcu_read_unlock()` | Yes | Requires `struct srcu_struct` |
| Tasks RCU | Implicit | No | Implicitly active across any region without a voluntary context switch |
| Tasks Trace RCU | `rcu_read_lock_trace()` / `rcu_read_unlock_trace()` | Yes | Used by faultable tracepoints and BPF sleepable programs |

- **Pointer publication and reading:** Pair `rcu_assign_pointer(p, v)` (release semantics) with `rcu_dereference(p)` (or `rcu_dereference_protected(p, lockdep_is_held(...))` on update paths).
- **In-place Mutation:** Never modify an RCU-protected object in-place if readers can see it. Create a copy, modify the copy, and swap the pointer using `rcu_assign_pointer()`.
- **Legacy Note:** `INIT_RCU_HEAD` is deprecated/removed; use standard initialization for `struct rcu_head`.

## 2. Remove-Before-Reclaim Ordering

`call_rcu()`, `kfree_rcu()`, and `synchronize_rcu()` only wait for RCU readers that existed when the grace period started; they do not prevent new readers from finding an object that is still linked into a shared structure:
1. **Unlink:** Remove the object from all RCU-traversed structures first (`list_del_rcu()`, `hlist_del_rcu()`, `rhashtable_remove_fast()`, `rcu_assign_pointer(..., NULL)`).
2. **Wait:** Wait for a grace period (`call_rcu(&obj->rcu, cb)`, `kfree_rcu(obj, rcu)`, or `synchronize_rcu()`).
3. **Reclaim:** Free the object in the RCU callback or after `synchronize_rcu()` returns.

Unlinking an object *inside* the `call_rcu()` callback or calling `kfree_rcu()` while the object is still reachable by readers is a use-after-free bug. At module unload or subsystem teardown, call `rcu_barrier()` to wait for all pending `call_rcu()` callbacks to finish.

## 3. `kvfree_call_rcu()` / `kfree_rcu()` Raw-Lock Calling Context

- `kfree_rcu(ptr, rhf)` / `kvfree_rcu(ptr, rhf)` invoke `kvfree_call_rcu()`.
- `kfree_rcu_mightsleep(ptr)` / `kvfree_rcu_mightsleep(ptr)` (one-argument head-less form) may sleep and can only be called from sleepable context.
- **Lock Nesting:** `kvfree_call_rcu()` is invoked from hardirq context and while holding `raw_spinlock_t`. Therefore, `kvfree_call_rcu()` and all of its callees must never acquire `spinlock_t`, `local_lock`, or `local_trylock`—even when guarded by `!IS_ENABLED(CONFIG_PREEMPT_RT)`—because `CONFIG_PROVE_RAW_LOCK_NESTING` checks static lock wait-types (`LD_WAIT_CONFIG` vs `LD_WAIT_SPIN`) and triggers `[ BUG: Invalid wait context ]`.

## 4. High-Frequency Bug Patterns & Review Invariants

| Bug Pattern | Review Invariant |
| :--- | :--- |
| **Missing Read Lock** | Every `rcu_dereference()` must be inside `rcu_read_lock()` or protected by `lockdep_is_held()`. |
| **Premature Free** | Ensure `synchronize_rcu()` or `call_rcu()` is called *after* unlinking and *before* `kfree()`. |
| **Deadlock** | Never acquire a mutex or sleepable lock inside an `rcu_read_lock()` critical section if that lock could be held by a writer waiting for a grace period. |
| **Uncancelled Work** | Ensure all timers/workqueues referencing an object are cancelled before the RCU grace period begins. |

### False-Positive Guards
- `rcu_dereference_protected(p, lockdep_is_held(...))` is the correct way to access RCU pointers when the writer lock is already held; this suppresses sparse/lockdep warnings.
- `rcu_preempt_depth()` is only valid under `CONFIG_PREEMPT_RCU`. Do not use it for logic that must run on all kernel configurations.

## 5. Lifecycle and Initialization
- **SRCU Cleanup:** Every `init_srcu_struct()` must be paired with `cleanup_srcu_struct()` during module/subsystem teardown to prevent memory leaks of the SRCU internal state.
- **Device-Managed Cleanup:** RCU structures are not managed by `devm_*` helpers. You must manually call `cleanup_srcu_struct()` in the driver's `remove` or `error` path.
- **Reference Counting:** When an RCU-protected object is passed to an asynchronous worker, the worker must either hold a reference count (if the object supports it) or ensure the RCU read-side critical section covers the entire duration of the object access.
- **Error Paths:** If an initialization sequence fails, ensure all RCU-related resources (like `srcu_struct`) are cleaned up before returning the error code to prevent resource leaks.
- **Memory Ordering:** Use `rcu_assign_pointer` for publication to ensure that all initialization writes are visible to readers before the pointer itself becomes visible.
