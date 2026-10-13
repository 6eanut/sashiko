# Read-Copy-Update (RCU) API (`include/linux/rcupdate.h`, `include/linux/srcu.h`)

## Read-Side Critical Sections, Variants, and Pointer Publication

| Variant | Read-Side API | Sleepable | Notes |
| :--- | :--- | :--- | :--- |
| Classic RCU | `rcu_read_lock()` / `rcu_read_unlock()` | No | Preemptible only under `CONFIG_PREEMPT_RCU`; never block/sleep |
| SRCU | `srcu_read_lock()` / `srcu_read_unlock()` | Yes | Requires `struct srcu_struct` |
| Tasks RCU | Implicit | No | Implicitly active across any region without a voluntary context switch |
| Tasks Trace RCU | `rcu_read_lock_trace()` / `rcu_read_unlock_trace()` | Yes | Used by faultable tracepoints and BPF sleepable programs |

- **Pointer publication and reading:** Pair `rcu_assign_pointer(p, v)` (release semantics) with `rcu_dereference(p)` (or `rcu_dereference_protected(p, lockdep_is_held(...))` on update paths).
- **Removed legacy macro:** `INIT_RCU_HEAD()` has been removed from the kernel.

## Remove-Before-Reclaim Ordering (`call_rcu`, `kfree_rcu`, `synchronize_rcu`)

`call_rcu()`, `kfree_rcu()`, and `synchronize_rcu()` only wait for RCU readers that existed when the grace period started; they do not prevent new readers from finding an object that is still linked into a shared structure:
1. Unlink the object from all RCU-traversed structures first (`list_del_rcu()`, `hlist_del_rcu()`, `rhashtable_remove_fast()`, `rcu_assign_pointer(..., NULL)`).
2. Wait for a grace period (`call_rcu(&obj->rcu, cb)`, `kfree_rcu(obj, rcu)`, or `synchronize_rcu()`).
3. Reclaim/free the object in the RCU callback or after `synchronize_rcu()` returns.

Unlinking an object *inside* the `call_rcu()` callback (or calling `kfree_rcu()` while the object is still reachable by readers) is a use-after-free bug.
- At module unload or subsystem teardown, call `rcu_barrier()` to wait for all pending `call_rcu()` callbacks to finish executing.

## `kvfree_call_rcu()` / `kfree_rcu()` Raw-Lock Calling Context

- `kfree_rcu(ptr, rhf)` / `kvfree_rcu(ptr, rhf)` invoke `kvfree_call_rcu()`, while `kfree_rcu_mightsleep(ptr)` / `kvfree_rcu_mightsleep(ptr)` (one-argument head-less form) may sleep and can only be called from sleepable context.
- `kvfree_call_rcu()` is invoked from hardirq context and while holding `raw_spinlock_t` (such as `p->pi_lock` in `kernel/sched/core.c`). Therefore, `kvfree_call_rcu()` and all of its callees must never acquire `spinlock_t`, `local_lock`, or `local_trylock`—even when guarded by `!IS_ENABLED(CONFIG_PREEMPT_RT)`—because `CONFIG_PROVE_RAW_LOCK_NESTING` (enabled by default) checks static lock wait-types (`LD_WAIT_CONFIG` vs `LD_WAIT_SPIN`) and triggers `[ BUG: Invalid wait context ]`.
