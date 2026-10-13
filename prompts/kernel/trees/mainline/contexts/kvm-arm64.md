# ARM64 KVM (Host / EL1) Code Context (`arch/arm64/kvm/`)

This guide provides authoritative technical invariants for the `arch/arm64/kvm/` subsystem in Linux v7.3-rc5, ensuring synchronization, lifecycle, and architectural correctness.

## 1. VM and vCPU Initialization Ordering

*   **First-Run Setup (`kvm_arch_vcpu_run_pid_change`)**: Before the first vCPU entry, software must initialize `VMPIDR_EL2`/`VPIDR_EL2` (which reset to UNKNOWN on warm reset), finalize vector/tagging features via `kvm_arm_vcpu_finalize()`, map VGIC resources via `kvm_vgic_map_resources()`, and compute traps via `kvm_calculate_traps()`.
*   **`vcpu_has_run_once(vcpu)` vs `kvm_vcpu_initialized(vcpu)` (`arch/arm64/include/asm/kvm_host.h`)**:
    *   `kvm_vcpu_initialized(vcpu)` only checks whether `KVM_ARM_VCPU_INIT` has completed and remains true forever after.
    *   Any capability, ioctl, or sysreg write check whose intent is "reject once the guest has started executing" **must** check `vcpu_has_run_once(vcpu)` (`!!vcpu->pid`).
*   **pKVM Hyp Objects Created at First `KVM_RUN`**: `pkvm_create_hyp_vm()` and `pkvm_create_hyp_vcpu()` run inside `kvm_arch_vcpu_run_pid_change()` at first `KVM_RUN`; before that, `vcpu_load()`'s `__pkvm_vcpu_load` hypercall is a no-op. Any new ioctl/cap callable before first `KVM_RUN` that issues guest hypercalls (`__pkvm_host_share_guest`, `__pkvm_host_mkyoung_guest`, etc.) must handle `pkvm_get_loaded_hyp_vcpu()` returning `NULL` in EL2 without triggering a `WARN_ON`.
*   **UAPI `ID_AA64*` Exposure**: Exposing a new `ID_AA64*` feature field to userspace requires matching writable masks (`kvm_id_reg_rw_mask` / RESx handling), MTE filtering by hardware support and VM protection type, and trap/enablement setup in `HCR_EL2`, `CPTR_EL2`, or `MDCR_EL2`.

## 2. Architectural State, Stage-2 Faults, and `enum vcpu_sysreg`

*   **PSCI Warm Reset (`CPU_ON`)**: Explicitly initialize EL2 trap registers (`HCR_EL2`, `CPTR_EL2`, `CNTHCTL_EL2`) and `HCR_EL2.RW = 1` for AArch64 guests; preserve `ELR_EL1`/`SPSR_EL1` ordering across exception injection and synchronous abort/SError paths; reject non-unique guest CPU affinities (`MPIDR_EL1`).
*   **Stage-2 Teardown and Faults (`user_mem_abort`)**:
    *   Serialize stage-2 page-table teardown against concurrent `mmu_notifier` walkers.
    *   In `user_mem_abort()`, every error path after resolving a PFN must release the PFN reference, `kvm_mmu_memory_cache` pointers must be initialized before use, and `vma_shift` must be re-validated after acquiring `mmu_lock`.
*   **`HCR_EL2` TLB-Cached Bits**: Modifying TLB-cached `HCR_EL2` fields (`RW`, `NV1`, `NV`, `E2H`, `FWB`, `DCT`) requires TLB invalidation (`ERET` alone does not flush them).
*   **Sparse `enum vcpu_sysreg` Numbering (`arch/arm64/include/asm/kvm_host.h`)**: VNCR-mapped entries in `enum vcpu_sysreg` are numbered by their VNCR byte offset (`VNCR(r) = __VNCR_START__ + VNCR_r / 8`), not in declaration order. **Never** use numeric range comparisons (`i >= REG_A && i <= REG_B`) over `enum vcpu_sysreg` values (e.g., `[CNTVOFF_EL2, CNTP_CTL_EL0]` accidentally spans `SCTLR_EL1`, `TCR_EL1`, and `MAIR_EL1`); use an explicit per-register switch/allowlist.

## 3. VGIC CPU Interface, LPIs/vLPIs, and `ICH_HCR_EL2`

*   **Virtual GIC CPU Interface**: Only write `ICV_AP<0|1>R<n>_EL1` during context switch/power management (in order AP0 before AP1, using the last read value or all-zeros); access to memory-mapped `GICV_*` is gated by `ICC_SRE_EL1_NS.SRE` (not `GICD_CTLR.ARE`).
*   **VGIC LPI Locking & GICv4 vLPIs**:
    *   Do not acquire `lpi_xa` lock while holding `vgic_irq->irq_lock` (or drop/re-acquire `irq_lock` under `lpi_xa` lock) unless following documented lock ordering, and never call `vgic_put_irq()` from a raw spinlock context while holding the LPI xarray lock.
    *   Gate GICv4 vLPI forwarding setup on vPE allocation availability, and `WARN` on any vLPI unmap failure.
*   **`ICH_HCR_EL2` Trap Ordering**: Modifying `ICH_HCR_EL2.En` requires a forced guest exit to flush the pipeline, and guest entry must resynchronize `ICH_HCR_EL2` trap bits before loading LRs or VMCR.

## 4. Implementation Invariants and Synchronization

*   **Register Updates**: Always use read-modify-write sequences to preserve reserved bits.
*   **Arithmetic Safety**: Use `check_add_overflow` for address calculations involving `VA_BITS` or guest memory ranges.
*   **Assembly/Macro Safety**: Parenthesize all macro arguments in expressions (e.g., `~(val)`). Verify assembly size suffixes match operand types.
*   **State Leakage**: Include all task-specific hardware configuration bits (e.g., MTE tags) in the state mask used during context switching.
*   **False-Positive Guard**: If a `WARN_ON` triggers during pKVM initialization, verify `vcpu_has_run_once` before assuming a state machine violation; many warnings are expected during early ioctl setup.
*   **Interrupt Locking**: Use `irq_lock` to protect shared IRQ state. Ensure that `irq_lock` is never held while performing blocking operations or acquiring `mmu_lock` to prevent deadlocks.
*   **Memory Management**: `kvm_hyp_memcache` must be topped up via `topup_hyp_memcache()` before any operation that might trigger a page table walk or allocation in EL2.
*   **Feature ID Space**: Use `in_feat_id_space()` to validate register access in the ID register range; ensure `sysreg_visibility()` is checked before exposing registers to userspace via `get_user`/`set_user`.
*   **Trace Initialization**: `kvm_hyp_trace_init()` must be called during early boot if `CONFIG_NVHE_EL2_TRACING` is enabled; otherwise, it is a no-op.
*   **System Register Access**: Use `sysreg_hidden()` or `sysreg_visible_as_raz()` to determine if a register should be masked from userspace or guest access, preventing information leakage of host-specific features.
*   **Memory Cache Lifecycle**: `__free_hyp_memcache()` must be used to release pages in the `kvm_hyp_memcache` during teardown, ensuring that `to_va` translation functions are correctly applied to physical addresses.
*   **Request Handling**: All `KVM_REQ_*` flags (e.g., `KVM_REQ_RELOAD_GICv4`, `KVM_REQ_NESTED_S2_UNMAP`) must be processed in the vCPU loop before re-entering the guest to ensure architectural state consistency.
