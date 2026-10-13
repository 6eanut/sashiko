# Workqueue Lifecycle & Teardown API (`include/linux/workqueue.h`, `kernel/workqueue.c`)

## 1. Queued Work & Teardown Guarantees
- Once a `struct work_struct` is queued via `queue_work()` / `schedule_work()`, synchronous workqueue teardown helpers (`flush_work()`, `cancel_work_sync()`, `disable_work_sync()`, `flush_workqueue()`, `destroy_workqueue()`) wait for any in-flight or pending execution to finish before returning.
- Do not flag a queued `work_struct` as leaked or dangling across teardown when the teardown path drains or cancels the work item (`cancel_work_sync()`, `disable_work_sync()`, or `flush_work()` when re-queuing is already prevented) or destroys its workqueue before freeing the enclosing structure.

## 2. Core API Ownership & Initialization
- **Ownership:** A `struct work_struct` is owned by the entity that allocates it. The workqueue subsystem does not take ownership of the memory containing the `work_struct`.
- **Initialization:** Every `work_struct` must be initialized via `INIT_WORK()` or `INIT_DELAYED_WORK()` before being queued. Re-initializing an active (queued or running) work item is a critical bug that corrupts internal list structures.
- **Lifecycle:** The `work_struct` must remain valid until the work item has finished execution. If the work item is embedded in a larger structure, the object must not be freed until the work is guaranteed to be idle.

## 3. Calling Context, Locking & Synchronization
- **Sleeping:** Workqueue callbacks are permitted to sleep (e.g., `mutex_lock()`, `msleep()`). If the workqueue is created with `WQ_MEM_RECLAIM`, the callback must avoid deadlocks involving memory allocation.
- **Atomic Context:** Standard `queue_work()` is safe in atomic context, but the callback itself must be aware of its execution environment.
- **Deadlock Prevention:** Never wait for the completion of a work item (e.g., `flush_work()`) while holding a lock that the work item itself needs to acquire.
- **Memory Ordering:** When publishing a newly initialized object to a shared data structure, ensure proper memory barriers are used if the work item is queued on a different CPU.

## 4. Return Values & Error Handling
- **`queue_work()` / `schedule_work()`:** Returns `true` if the work was successfully queued, `false` if it was already pending.
- **Allocation Failures:** When creating custom workqueues via `alloc_workqueue()`, always check for `NULL` return values. Dereferencing a `NULL` workqueue pointer in `queue_work()` triggers a kernel panic.
- **Error Propagation:** If a multi-step configuration fails, ensure that any work items scheduled during the partial setup are cancelled before returning the error code.

## 5. High-Frequency Bug Patterns & Review Invariants
- **The "Uncancelled Work" Anti-Pattern:** Before `kfree()` or `devm_kfree()` of an object containing a `work_struct`, you must call `cancel_work_sync()` or `cancel_delayed_work_sync()`.
- **Teardown Symmetry:** If the initialization path uses `INIT_WORK`, the teardown path must use `cancel_work_sync()`.
- **Re-arming Races:** A work callback that checks a condition and re-arms itself must be protected against device removal.
    - *Review Invariant:* Ensure the "stop" signal (e.g., `dev->removed = true`) is set *before* calling `cancel_work_sync()`.
- **Initialization Ordering:** Never expose a device or object to the system (e.g., `device_add()`) until the `work_struct` and its associated workqueue are fully initialized.
- **Format Strings:** When using `alloc_workqueue()`, the name argument must be a literal string. Passing a dynamic format string is a security risk and a violation of kernel coding standards.

## 6. Advanced Concurrency & State Management
- **Worker Pools:** The subsystem manages worker pools per CPU and for unbound workqueues. Accessing internal pool state (e.g., `struct worker_pool`) requires holding `pool->lock`.
- **Work Bits:** The `data` field in `work_struct` encodes both the `pwq` (pool workqueue) pointer and status flags. Direct manipulation of these bits is forbidden; use the provided API helpers.
- **Affinity:** For unbound workqueues, `struct workqueue_attrs` defines the `cpumask` and `affn_strict` policy. Changes to these attributes via `apply_workqueue_attrs()` are expensive and should not be performed in hot paths.
- **BH Workqueues:** `BH_WORKER_JIFFIES` and `BH_WORKER_RESTARTS` define the constraints for BH-context work execution. These are strictly non-sleeping and must not block.

## 7. Summary Checklist for Reviewers
1. **Cancellation:** Is `cancel_work_sync()` or `cancel_delayed_work_sync()` present in the `remove` or `exit` path?
2. **Lifetime:** Does the work callback access pointers that might be freed while the work is running?
3. **Re-arming:** If the work item calls `queue_work()` on itself, is there a mechanism to prevent this after device shutdown?
4. **Allocations:** Are blocking memory allocations performed inside a workqueue that might be triggered under memory pressure?
5. **Initialization:** Are there `NULL` checks after `alloc_workqueue()`?
6. **Context:** Does the work item perform operations that are illegal in the current execution context (e.g., sleeping in an atomic-only path)?
7. **State Machine:** Is the object state fully validated before the work item is queued to prevent race conditions during initialization?
8. **Resource Management:** Are all resources (e.g., IRQs, memory, locks) held by the work item properly released or synchronized during teardown?
9. **Re-initialization:** Is `INIT_WORK` called only once during object setup, never on a work item that is currently queued or running?
10. **BH Context:** If using BH workqueues, are all operations strictly non-blocking?
