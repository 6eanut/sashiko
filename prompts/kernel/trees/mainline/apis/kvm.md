# KVM Core Helper APIs (`include/linux/kvm_host.h`)

## Memslot HVA/PFN Resolution Helpers

*   **SRCU Precondition**: Calling `gfn_to_hva*()`, `gfn_to_pfn*()`, or accessing `kvm_memslots()` / `kvm_vcpu_memslots()` requires holding `srcu_read_lock(&kvm->srcu)` or `kvm->slots_lock`.
*   **Write-Intent vs Read-Only Memslots (`KVM_MEM_READONLY`)**:
    *   `gfn_to_hva()`, `gfn_to_hva_many()`, and `gfn_to_hva_memslot()` request *writable* translations and return `KVM_HVA_ERR_RO_BAD` on valid `KVM_MEM_READONLY` slots (such as ROM or ARM64 UEFI flash regions).
    *   Read-access fault, prefetch, and emulation paths must use `gfn_to_hva_memslot_prot(..., &writable)` or read-intent variants so read-only memslots succeed.
*   **PFN Validation Sentinels**: Use `is_error_pfn()`, `is_noslot_pfn()`, and `is_error_noslot_pfn()` to distinguish between translation failures and missing slots. `KVM_PFN_ERR_MASK` and `KVM_PFN_NOSLOT` are used internally to encode these states; never compare raw PFNs against these masks directly.

## vCPU Attachment and Request Helpers

*   **`vcpu_load()` / `vcpu_put()`**: Must be paired around any ioctl or code path that accesses physical-CPU-dependent vCPU state (hardware registers, timers, preempt notifiers). Omitting `vcpu_put()` leaks preempt notifiers and crashes in `kvm_sched_out()`.
*   **`kvm_make_request()` / `kvm_check_request()`**: `kvm_make_request()` issues `smp_wmb()` before setting the request bit, paired with `smp_mb__after_atomic()` in `kvm_check_request()` (and `smp_mb()` + `IN_GUEST_MODE` in `kvm_vcpu_kick()`). Do not add redundant manual memory barriers around `kvm_make_request()`.
*   **Request Flags**: Use `KVM_REQUEST_WAIT` to force synchronous completion of cross-vCPU requests (e.g., `KVM_REQ_TLB_FLUSH`).

## Reference-Counting and Lifecycle Invariants

*   **Ownership**: `struct kvm` and `struct kvm_vcpu` are reference-counted. Subsystem lookups (e.g., `kvm_get_vcpu()`) yield a reference that must be explicitly dropped via `kvm_put_kvm()` or equivalent.
*   **Initialization Rollback**: Error paths in multi-step initialization must perform "reverse-order" cleanup. Failure to invoke teardown helpers (e.g., `kvm_destroy_vm`) on partial failure results in memory leaks or dangling pointers.
*   **Teardown**: Asynchronous work (timers, workqueues) must be synchronously cancelled (e.g., `cancel_delayed_work_sync()`) before final object destruction.

## Concurrency and Memory Ordering

*   **Lock Hierarchy**: Enforce `kvm->lock` --> `kvm->slots_lock` --> `kvm->irq_lock`.
*   **Sleeping Constraints**: `srcu_read_lock(&kvm->srcu)` is a sleeping lock. Never invoke in atomic context (hard IRQ or `spin_lock` critical sections).
*   **Memslot Updates**: The `KVM_MEMSLOT_GEN_UPDATE_IN_PROGRESS` flag prevents cache hits during updates.
*   **Publication**: Use `rcu_assign_pointer()` or `smp_wmb()` to ensure initialization writes are visible before exposing objects to other threads.

## Error Handling and Validation

*   **Pointer Sentinels**: Use `kvm_is_error_hva()` for HVA validation.
*   **Bounds Checking**: Validate all user-provided indices (vCPU IDs, interrupt IDs) against `KVM_MAX_VCPUS` using the same integer width as the allocation/indexing to prevent truncation.
*   **Security/State**:
    *   Zero-initialize structures before copying to userspace/guest to prevent stack information leaks.
    *   Explicitly flush/invalidate caches when memory ownership or encryption state (SEV/TDX) changes.
    *   Do not use `WARN_ON()` for expected runtime failures (e.g., guest-triggered faults); use `pr_debug()` or tracepoints.
    *   Ensure objects are fully initialized before exposure to userspace; use state flags to block interaction during the "in-progress" window.

## Architectural and State Machine Invariants

*   **State Consistency**: When a hardware acceleration feature (e.g., AVIC, nested virtualization) is inhibited, all associated security-critical intercepts and state configurations must be updated immediately to prevent L1/L2 privilege escalation.
*   **Cache Maintenance**: When memory ownership or encryption state changes (e.g., SEV migration), perform explicit cache maintenance. Do not assume hardware coherency for non-cacheable mappings.
*   **Register Synchronization**: All necessary hardware registers (e.g., debug registers) must be synchronized when switching context to the hypervisor.
*   **Atomic Bitfields**: When using atomic operations to modify state bitfields, ensure the operation preserves the integrity of surrounding bits in the same word.
*   **Input Sanitization**: Always validate input parameters (e.g., interrupt IDs) before performing state-modifying operations. Use `is_error_pfn()` and related helpers to distinguish between "not in slot" and "translation failure" to avoid incorrect error propagation.
*   **Idempotency**: Ensure that state transitions (e.g., VM reset, vCPU hotplug) are idempotent. Repeatedly calling a transition function must not result in leaked resources or inconsistent hardware state.
*   **TLB Invalidation**: Always perform required architectural state updates (e.g., TLB flushes) even when a transition operation fails, as stale translations can lead to security vulnerabilities.
*   **Memory Alignment**: Ensure shared data structures are properly aligned and padded to avoid cache line sharing and coherency issues, particularly when accessed with the MMU disabled.
