# KVM Subsystem Code Context (`virt/kvm/`, `arch/x86/kvm/`)

## Locking Hierarchy (`Documentation/virt/kvm/locking.rst`)

* **Outer-to-Inner Order**: `cpus_read_lock()` -> `kvm_lock` -> `kvm->lock` -> `vcpu->mutex` / `kvm->slots_lock` -> `srcu_read_lock(&kvm->srcu)` -> `kvm->mmu_lock`.
* **Forbidden Nestings**:
  * Never call `cpus_read_lock()` while holding `kvm_lock`.
  * Never acquire `kvm->lock` while holding `vcpu->mutex`.
  * `synchronize_srcu(&kvm->srcu)` is called under `kvm->lock`, `vcpu->mutex`, and `kvm->slots_lock`; **never** acquire any of those three mutexes inside `srcu_read_lock(&kvm->srcu)` (the arch-specific `kvm->slots_arch_lock` does not synchronize SRCU and may be taken inside `kvm->srcu`).
  * MMU notifier callbacks (`invalidate_range_start` / `invalidate_range_end`) must never acquire `kvm->slots_lock` or `kvm->slots_arch_lock`.
  * Never sleep (`mutex_lock`, `copy_from_user`, `GFP_KERNEL` allocations) while holding `kvm->mmu_lock`.

## MMU Notifier Invalidation Retry Protocol

* **Mandatory Page-Fault Sequence**:
  1. Capture `mmu_invalidate_seq = kvm->mmu_invalidate_seq` (before resolving PFN).
  2. Resolve guest GFN/HVA to host PFN (outside `kvm->mmu_lock`).
  3. Acquire `kvm->mmu_lock`.
  4. Check staleness under `kvm->mmu_lock` (e.g., x86 `is_page_fault_stale()`, which checks root shadow-page obsolescence, `KVM_REQ_MMU_FREE_OBSOLETE_ROOTS`, and `mmu_invalidate_retry_gfn()`). Pre-lock `mmu_invalidate_retry_gfn_unsafe()` is a legitimate fast-path optimization but never replaces the locked check.
  5. Install the PTE without dropping `kvm->mmu_lock` after the check.
  6. Release `kvm->mmu_lock`.
  * *Exception*: Fault handlers that pin pages via `pin_user_pages()` (`FOLL_PIN`) and manage hyp page ownership directly (such as ARM64 `pkvm_mem_abort()`) do not use MMU notifier retry.
* **`gfn_to_pfn_cache` Refresh**: Must re-check `mmu_invalidate_seq` after pinning the page to close the TOCTOU window.
* **Private vs Public Memslots (`KVM_SET_USER_MEMORY_REGION2`)**: Private (`KVM_MEM_GUEST_MEMFD`) and public memslots must never overlap in GPA space, and memslot flag/range updates must go through `kvm_set_memory_region()`.

## x86 Shadow Page `sp->role.invalid` and Dirty Tracking

* **Shadow Page `sp->role.invalid` (`arch/x86/kvm/mmu/mmu.c`)**:
  * `sp->role.invalid` marks a `struct kvm_mmu_page` as obsolete (`is_obsolete_sp()` returns true regardless of `mmu_valid_gen`), not freed.
  * An invalid page must never remain on `kvm->arch.active_mmu_pages` (`kvm_zap_obsolete_pages()` warns with `WARN_ON_ONCE(sp->role.invalid)` and skips it) or be reachable from a live parent SPTE.
  * `kvm_mmu_child_role()` inherits `role.invalid` from the parent, and `unaccount_shadowed()` in `__kvm_mmu_prepare_zap_page()` is skipped when `sp->role.invalid` is set — so creating or linking a child under an already-invalid parent leaks accounting and places an invalid page onto `active_mmu_pages`.
* **Leaf SPTE Hardware Bits**: Updates to leaf SPTE bits written by hardware page walkers (Accessed/Dirty) must use atomic single-instruction operations (e.g., `xchg`, atomic AND, or `cmpxchg`) so concurrent hardware updates are not clobbered.
* **Dirty Ring & Dirty Bitmap**: When a dirty ring is full, flush entries unconditionally to the backup dirty bitmap before clearing the ring; `KVM_CLEAR_DIRTY_LOG` ranges must be 64-bit aligned.
