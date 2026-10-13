# RISC-V Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides architectural invariants and bug-prevention rules for `arch/riscv/`. Adherence is mandatory for memory model compliance, SBI interface usage, and trap handling stability in Linux v7.3-rc5.

## 1. SBI Interface and CSR Access
*   **SBI Gateway:** Use `sbi_ecall()` for all firmware-assisted operations. **Invariant:** Never assume SBI extension availability; verify via `sbi_probe_extension()` before invocation.
*   **CSR Wrappers:** Use `csr_read()`, `csr_write()`, `csr_set()`, and `csr_clear()`. These include `barrier()` to prevent reordering of CSR modifications relative to memory operations.
*   **SBI Error Handling:** `sbi_ecall` returns `struct sbiret`. Always check `sbiret.error`. Map SBI error codes to standard Linux error codes (e.g., `SBI_ERR_NOT_SUPPORTED` to `-ENOTSUPP`).
*   **Context Attribution:** Ensure context switching logic correctly identifies the source of state (kernel vs. user vs. guest) to prevent register corruption during `sbi_ecall` transitions.

## 2. Synchronization and Concurrency
*   **Memory Model (RVWMO):** RISC-V uses a weak memory model. Atomic operations must use explicit memory ordering (`acquire`/`release`) via `smp_mb__before_atomic()` or `smp_mb__after_atomic()` when synchronizing across hart boundaries.
*   **Interrupt Masking:** When modifying shared hardware state (e.g., PLIC/AIA registers or TLB flush queues), use `local_irq_save()` and `local_irq_restore()`.
*   **Atomic Context:** Sleeping is strictly forbidden in trap handlers (`handle_exception`) and low-level TLB flush paths.
*   **Per-CPU State:** Use `raw_cpu_ptr()` or `this_cpu_ptr()` in non-preemptible contexts. If migration is possible, use `get_cpu_var()`/`put_cpu_var()` to disable preemption.
*   **Serialization:** Hardware state updates must be serialized. Producer indices for command queues must only be advanced after successful writes to prevent race conditions.
*   **Lock Hierarchy:** When multiple locks are required, acquire them in the order defined by the subsystem's lock dependency graph to prevent circular waits.
*   **Reentrancy:** Ensure that interrupt handlers do not attempt to re-acquire locks held by the interrupted thread, which would lead to immediate deadlock.

## 3. Initialization and Resource Management
*   **Boot Sequence:** Follow the established order: `setup_arch()` -> `sbi_init()` -> `paging_init()` -> `trap_init()`.
*   **Feature Publication:** Do not expose features (e.g., `hwprobe`, vector support) to userspace until the `cpu_feature` bit is set and state is fully initialized.
*   **Resource Unwind:** Use `devm_*` managed resources. For manual allocations, ensure symmetric `kfree()` or `iounmap()` on all error paths.
*   **Initialization Order:** Ensure asynchronous initialization (e.g., unaligned access probes) is synchronized with consumers (e.g., vDSO data access) to prevent exposure of incomplete data.
*   **Error Propagation:** All internal functions must propagate error codes (e.g., `-ENOMEM`, `-EINVAL`) to the caller; never swallow errors or return partial state.

## 4. Trap Handling and Boundary Validation
*   **User Access:** In trap handlers, verify `user_mode(regs)` before dereferencing. Use `copy_from_user()`/`copy_to_user()` exclusively; direct dereference of user pointers is prohibited.
*   **Stack Frames:** Avoid hardcoded stack offsets in assembly. Use `struct pt_regs` offsets to maintain ABI compatibility.
*   **Overlapping Instrumentation:** Correctly handle overlapping instrumentation (e.g., Ftrace vs. Kprobes) on the same instruction address to prevent kernel warnings.
*   **Pointer Validation:** Always validate pointers for `NULL` before dereferencing, especially in interrupt contexts where the kernel cannot recover from a page fault.

## 5. High-Frequency Bug Patterns
*   **Sign Extension:** On RV64, ensure explicit sign extension for 32-bit values. Use `sign_extend64()` or cast to `s64` before arithmetic to prevent invalid address calculations.
*   **Bitwise Operations:** Use `BIT_ULL()` for 64-bit masks. `BIT()` truncates to 32 bits on RV32, causing silent corruption.
*   **Integer Overflows:** Use `check_add_overflow()` or `check_mul_overflow()` when calculating buffer sizes (e.g., `hwprobe`, `kexec`).
*   **Alignment:** Ensure all page tables and critical structures are aligned to `PAGE_SIZE` using `__aligned(PAGE_SIZE)`.
*   **JIT Branching:** BPF JIT branch offsets must be calculated based on absolute target addresses, not relative offsets, to avoid stale pass-specific data corruption.
*   **Memory Safety:** Optimized loops (e.g., `strnlen`) must not perform memory accesses beyond the requested range before verifying boundary conditions to prevent page faults.
*   **Topology Limits:** Static array sizes must be derived from `NR_CPUS` or other hardware-defined limits rather than hardcoded constants to prevent buffer overflows in high-core-count systems.

## 6. Hardware, DMA, and MMIO
*   **Cache Coherency:** RISC-V lacks hardware-managed I/O coherency. Use `dma_map_*` and `dma_sync_*` APIs. Manual cache flushing (`sfence.vma`, `cbo.flush`) is restricted to low-level architecture code.
*   **MMIO Access:** Use `readl()`/`writel()` for MMIO to ensure necessary memory barriers are applied.
*   **Hardware Limits:** Static array sizes must be derived from maximum possible hardware topology limits (e.g., `NR_CPUS`) rather than hardcoded constants.
*   **Reset Sequencing:** Quiesce hardware and stop memory transfers before system reboot or halt to prevent in-flight transaction corruption.
*   **Register Validation:** Validate all register base addresses against the device tree or silicon specification before performing any read/write operations.
*   **Atomic Ordering:** When implementing custom atomic primitives, ensure `smp_mb()` is used to enforce the RISC-V weak memory model ordering requirements.
*   **Kconfig Integrity:** Verify that all `select` statements in `arch/riscv/Kconfig` have their dependencies met to prevent build-time symbol resolution failures.
*   **Interrupt Context:** Never perform blocking operations (e.g., `mutex_lock`, `kmalloc(GFP_KERNEL)`) inside `handle_exception` or any path reachable by an interrupt handler.
