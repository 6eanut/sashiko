# Kernel Tracepoints & `TRACE_EVENT` API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for using the Linux kernel tracing infrastructure. Adherence to these patterns is mandatory to prevent common concurrency, memory safety, and lifecycle bugs.

## 1. `TRACE_EVENT` Declarations and `__assign_str()`

*   **`TP_STRUCT__entry()` and `TP_fast_assign()` string helpers:**
    *   `__string(name, src)` declares a dynamic-length string field and captures the `src` expression in stage 5 (`__data_offsets`).
    *   In `TP_fast_assign()`, `__assign_str(name)` takes **only a single argument (`name`)**—passing the source pointer as a second argument fails to compile on modern kernels.
    *   `__vstring(name, fmt, ap)` pairs with `__assign_vstr(name, fmt, va)`.
*   **No side effects in `TP_fast_assign()`:** `TP_fast_assign()` executes only when the tracepoint is enabled; never place state-mutating expressions inside it.
*   **`TRACE_EVENT_CONDITION` / `TP_CONDITION`:** Use `TRACE_EVENT_CONDITION()` when tracepoint arguments require dereferences or checks that must be skipped before ring-buffer reservation when the condition is false.
*   **`CREATE_TRACE_POINTS`:** Exactly one `.c` translation unit per tracepoint header must `#define CREATE_TRACE_POINTS` before including the trace header.

## 2. Execution Contexts and Synchronization

*   **Execution context:**
    *   Standard tracepoints execute under `guard(preempt_notrace)` (`preempt_disable_notrace()`); probe callbacks must not sleep or allocate with `GFP_KERNEL`.
    *   Syscall/faultable tracepoints (`DECLARE_TRACE_SYSCALL` / `TRACE_EVENT_SYSCALL`) execute under `guard(rcu_tasks_trace)` and may fault/sleep.
*   **Unregistration synchronization (`tracepoint_synchronize_unregister()`):** After calling `tracepoint_probe_unregister()` or `rv_detach_trace_probe()`, call `tracepoint_synchronize_unregister()` (which waits for both `synchronize_rcu_tasks_trace()` and `synchronize_rcu()`) before freeing probe state or completing module unload.
*   **Kconfig dependencies for tracepoint consumers:** Code registering probes via `tracepoint_probe_register()` or `rv_attach_trace_probe()` must `depends on` the Kconfig symbols that build the `.c` file defining `CREATE_TRACE_POINTS` for that tracepoint (for example, `page_fault_user` / `page_fault_kernel` in `arch/riscv/mm/fault.c` require `depends on MMU`).

## 3. High-Frequency Bug Patterns & Review Invariants

### Memory Safety & Concurrency
*   **Registration Ownership:** The tracepoint infrastructure stores a pointer to your callback. You must unregister any probe before the module or object containing the callback code is freed.
*   **Registration Failure:** Always check the return value of `tracepoint_probe_register()`. A non-zero return indicates failure.
*   **Rollback:** If a multi-step registration fails, perform a full rollback of all previously successful registrations using a `goto` cleanup pattern.
*   **Recursion Protection:** Trace handlers must be re-entrant safe. If a probe triggers another tracepoint that could call the same probe, implement a recursion guard (e.g., `this_cpu_inc_return()` checks).
*   **RCU Safety:** If a probe accesses data structures that can be freed, ensure access is within an `rcu_read_lock()` section or protected by the appropriate RCU grace period.
*   **Deferred Cleanup:** When deferring object cleanup (e.g., via `call_rcu`), ensure the callback holds a valid reference or that the object is guaranteed to exist until the grace period expires.

### Initialization & Teardown
*   **Initialization Races:** Ensure the tracepoint is fully initialized before it is exposed to the system. Do not register probes in a module's `init` function until all internal data structures are fully allocated and validated.
*   **Stale Statistics:** If a tracer maintains per-session counters, explicitly reset them in the `init` or `reset` path. Do not rely on zero-initialization if the tracer can be re-enabled.
*   **Double-Free Guard:** When using `rv_attach_trace_probe()`, ensure the `monitor` and `tp` are valid. If registration fails, the error path must not attempt to detach the probe.
*   **Atomic Context:** For non-faultable tracepoints, use `call_tracepoint_unregister_atomic()` to ensure proper grace period handling without violating atomic context constraints.

## 4. Summary Checklist for Reviewers
1.  **Is `tracepoint_synchronize_unregister()` present** in the module exit path?
2.  **Are `GFP_KERNEL` allocations absent** in non-syscall tracepoints?
3.  **Is `__assign_str()` used correctly** (single argument)?
4.  **Are all registration return values checked?**
5.  **Is there a complete rollback path** for multi-step registrations?
6.  **Are all pointer dereferences guarded** by null checks or RCU read locks?
7.  **Is the tracepoint placed at the commit point** (after hardware/firmware submission)?
8.  **Are `page_fault_kernel` and `page_fault_user` dependencies** correctly satisfied by `MMU` Kconfig?
9.  **Are per-CPU data accesses protected** against task migration or preemption?
10. **Are all asynchronous callbacks** fully synchronized before resource release?
11. **Are field type assumptions validated** before performing operations that depend on the field's internal structure?
12. **Are error codes properly checked** before being used as loop bounds or array indices?
