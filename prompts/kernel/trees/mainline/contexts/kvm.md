# KVM Subsystem Code Context (`virt/kvm/`, `arch/x86/kvm/`)

## 1. Locking Hierarchy (`Documentation/virt/kvm/locking.rst`)

* **Outer-to-Inner Order**: `cpus_read_lock()` -> `kvm_lock` -> `kvm->lock` -> `vcpu->mutex` / `kvm->slots_lock` -> `srcu_read_lock(&kvm->srcu)` -> `kvm->mmu_lock`.
* **Forbidden Nestings**:
    * Never call `cpus_read_lock()` while holding `kvm_lock`.
    * Never acquire `kvm->lock` while holding `vcpu->mutex`.
    * `synchronize_srcu(&kvm->srcu)` is called under `kvm->lock`, `vcpu->mutex`, and `kvm->slots_lock`; **never** acquire any of those three mutexes inside `srcu_read_lock(&kvm->srcu)` (the arch-specific `kvm->slots_arch_lock` does not synchronize SRCU and may be taken inside `kvm->srcu`).
    * MMU notifier callbacks (`invalidate_range_start` / `invalidate_range_end`) must never acquire `kvm->slots_lock` or `kvm->slots_arch_lock`.
    * Never sleep (`mutex_lock`, `copy_from_user`, `GFP_KERNEL` allocations) while holding `kvm->mmu_lock`.

## 2. MMU Notifier Invalidation Retry Protocol

* **Mandatory Page-Fault Sequence**:
    1. Capture `mmu_invalidate_seq = kvm->mmu_invalidate_seq` (before resolving PFN).
    2. Resolve guest GFN/HVA to host PFN (outside `kvm->mmu_lock`).
    3. Acquire `kvm->mmu_lock`.
    4. Check staleness under `kvm->mmu_lock` (e.g., x86 `is_page_fault_stale()`, which checks root shadow-page obsolescence, `KVM_REQ_MMU_FREE_OBSOLETE_ROOTS`, and `mmu_invalidate_retry_gfn()`). Pre-lock `mmu_invalidate_retry_gfn_unsafe()` is a legitimate fast-path optimization but never replaces the locked check.
    5. Install the PTE without dropping `kvm->mmu_lock` after the check.
    6. Release `kvm->mmu_lock`.
    * *Exception*: Fault handlers that pin pages via `pin_user_pages()` (`FOLL_PIN`) and manage hyp page ownership directly (such as ARM64 `pkvm_mem_abort()`) do not use MMU notifier retry.
* **Notifier Start/End Pairing**: Every `invalidate_range_start()` must be balanced by exactly one `invalidate_range_end()` that observes the same memslots. `kvm->mn_active_invalidate_count` blocks memslot updates between the two calls so that conditional locking takes the same decision in both; any change that lets memslots change in between, or that skips one side on an early return, unbalances `kvm->mmu_invalidate_in_progress` and makes the retry check report "no retry needed" for a stale PFN.
* **`gfn_to_pfn_cache` Refresh**: Must re-check `mmu_invalidate_seq` after pinning the page to close the TOCTOU window.
* **Private vs Public Memslots (`KVM_SET_USER_MEMORY_REGION2`)**: Private (`KVM_MEM_GUEST_MEMFD`) and public memslots must never overlap in GPA space, and memslot flag/range updates must go through `kvm_set_memory_region()`.

## 3. x86 Shadow Page `sp->role.invalid` and Dirty Tracking

* **Shadow Page `sp->role.invalid` (`arch/x86/kvm/mmu/mmu.c`)**:
    * `sp->role.invalid` marks a `struct kvm_mmu_page` as obsolete (`is_obsolete_sp()` returns true regardless of `mmu_valid_gen`), not freed.
    * An invalid page must never remain on `kvm->arch.active_mmu_pages` (`kvm_zap_obsolete_pages()` warns with `WARN_ON_ONCE(sp->role.invalid)` and skips it) or be reachable from a live parent SPTE.
    * `kvm_mmu_child_role()` inherits `role.invalid` from the parent, and `unaccount_shadowed()` in `__kvm_mmu_prepare_zap_page()` is skipped when `sp->role.invalid` is set — so creating or linking a child under an already-invalid parent leaks accounting and places an invalid page onto `active_mmu_pages`.
* **Leaf SPTE Hardware Bits**: Updates to leaf SPTE bits written by hardware page walkers (Accessed/Dirty) must use atomic single-instruction operations (e.g., `xchg`, atomic AND, or `cmpxchg`) so concurrent hardware updates are not clobbered.
* **Dirty Ring & Dirty Bitmap**: When a dirty ring is full, flush entries unconditionally to the backup dirty bitmap before clearing the ring; `KVM_CLEAR_DIRTY_LOG` ranges must be 64-bit aligned.

## 4. Object Lifetimes and Invariants

* **Reference Counting**: Objects retrieved from lookup tables (e.g., `kvm_get_vcpu()`) must hold a reference count for the duration of access to prevent UAF during concurrent VM teardown or hotplug.
* **Publication**: Objects must be fully initialized and linked into parent lists before being made visible to ioctl paths or other vCPUs.
* **Integer Overflows**: Use `check_add_overflow()` for all GPA/HVA range calculations to prevent wrap-around.
* **State Consistency**: If an operation (e.g., `KVM_SET_USER_MEMORY_REGION2`) fails, the VM state must be reverted to the last known good configuration; partial updates are forbidden.
* **Generation Numbers**: `KVM_MEMSLOT_GEN_UPDATE_IN_PROGRESS` (bit 63) must be set during `kvm_swap_active_memslots()` to force cache invalidation across all vCPUs.
* **`mmu_valid_gen`**: This field tracks the generation of the MMU root; `is_obsolete_sp()` checks this against the current `kvm->arch.mmu_valid_gen` to determine if a shadow page is stale due to a global MMU invalidation event (e.g., memslot deletion or CR3 change).

## 5. Synchronization and Concurrency Contracts

* **Atomic Contexts**: KVM operations involving `kvm->mmu_lock` are strictly non-blocking. Any operation requiring memory allocation or user-space access must be performed before acquiring the lock.
* **Interrupt Masking**: When modifying shared hardware registers (e.g., VMCS/VMCB fields), ensure local interrupts are masked if the operation is not already protected by a spinlock, preventing reentrancy deadlocks.
* **Cache Invalidation**: When memory ownership or encryption state changes (e.g., SEV/TDX transitions), all relevant CPU caches must be explicitly flushed or invalidated to prevent stale data usage.
* **Error Propagation**: All failure paths returning to userspace must populate the appropriate exit status fields (e.g., `vcpu->run->exit_reason`) to avoid ambiguous error reporting.
* **Hardware Write Ordering**: Control register writes must be strictly ordered after data register writes to ensure hardware does not observe incomplete state.
* **Memory Accounting**: All dynamic memory allocations (e.g., `kvm_mmu_memory_cache`) must be accounted to the caller's `memcg` to prevent memory leaks and resource exhaustion.
* **Signal Handling**: Operations interrupted by signals (e.g., `KVM_PFN_ERR_SIGPENDING`) must be distinguished from hardware errors; the return path must ensure the operation is either safely aborted or correctly re-queued.
