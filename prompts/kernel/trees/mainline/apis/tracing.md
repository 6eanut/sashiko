# Kernel Tracepoints & `TRACE_EVENT` API (`include/linux/tracepoint.h`, `include/trace/define_trace.h`)

## `TRACE_EVENT` Declarations and `__assign_str()`

- **`TP_STRUCT__entry()` and `TP_fast_assign()` string helpers:**
  - `__string(name, src)` declares a dynamic-length string field and captures the `src` expression in stage 5 (`__data_offsets`).
  - In `TP_fast_assign()`, `__assign_str(name)` takes **only a single argument (`name`)**—passing the source pointer as a second argument fails to compile on modern kernels.
  - `__vstring(name, fmt, ap)` pairs with `__assign_vstr(name, fmt, va)`.
- **No side effects in `TP_fast_assign()`:** `TP_fast_assign()` executes only when the tracepoint is enabled; never place state-mutating expressions inside it.
- **`TRACE_EVENT_CONDITION` / `TP_CONDITION`:** Use `TRACE_EVENT_CONDITION()` when tracepoint arguments require dereferences or checks that must be skipped before ring-buffer reservation when the condition is false.
- **`CREATE_TRACE_POINTS`:** Exactly one `.c` translation unit per tracepoint header must `#define CREATE_TRACE_POINTS` before including the trace header.

## Tracepoint Probe Contexts, Registration, and Kconfig Dependencies

- **Execution context:**
  - Standard tracepoints execute under `guard(preempt_notrace)` (`preempt_disable_notrace()`); probe callbacks must not sleep or allocate with `GFP_KERNEL`.
  - Syscall/faultable tracepoints (`DECLARE_TRACE_SYSCALL` / `TRACE_EVENT_SYSCALL`) execute under `guard(rcu_tasks_trace)` and may fault/sleep.
- **Unregistration synchronization (`tracepoint_synchronize_unregister()`):** After calling `tracepoint_probe_unregister()` or `rv_detach_trace_probe()`, call `tracepoint_synchronize_unregister()` (which waits for both `synchronize_rcu_tasks_trace()` and `synchronize_rcu()`) before freeing probe state or completing module unload.
- **Kconfig dependencies for tracepoint consumers:** Code registering probes via `tracepoint_probe_register()` or `rv_attach_trace_probe()` must `depends on` the Kconfig symbols that build the `.c` file defining `CREATE_TRACE_POINTS` for that tracepoint (for example, `page_fault_user` / `page_fault_kernel` in `arch/riscv/mm/fault.c` require `depends on MMU`).
