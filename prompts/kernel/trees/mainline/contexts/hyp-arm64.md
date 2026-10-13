# ARM64 Hyp (EL2 / pKVM / nVHE) Code Context

## pKVM Threat Model and Fatal Panic Reachability

* **Guaranteed**: Guest confidentiality/integrity vs. Host EL1; Hypervisor (EL2) integrity vs. Host and Guests; Host availability vs. Guests.
* **Not Guaranteed**: Host availability vs. privileged Host EL1 self-DoS; recovery from EL3/SMMU/GIC hardware or firmware faults.
* **Fatal EL2 Primitives (`BUG`, `BUG_ON`, `WARN_ON`, `hyp_panic`)**: At EL2 nVHE/pKVM, both `BUG_ON()` and `WARN_ON()` expand to `BRK` (via `__BUG_FLAGS` in `arch/arm64/include/asm/bug.h`) and terminate in `hyp_panic()` — there is no "warn and continue" execution path. Plain `BUG()` and `hyp_panic()` are equivalent at EL2 nVHE (`BUG()` is the house style in `arch/arm64/kvm/hyp/nvhe/`).
  * Triggerable by a **Guest** (via DMA, hypercall side effects, or shared memory) or by **Host userspace** (via syscall/ioctl -> host kernel -> hypercall): **Bug**.
  * Triggerable only by **privileged Host EL1** code paths: hardening improvement, not a bug.
  * `WARN_ON(cond)` is only appropriate for EL2-internal invariant violations; any `if (WARN_ON(err)) goto out;` recovery branch at EL2 is dead code because `WARN_ON` panics.

## EL2 Execution Context

* An nVHE/pKVM hypercall or trap handler executes as a single **atomic, non-preemptible unit on the trapping CPU** with physical interrupts masked until `eret`.
* There is no scheduler, preemption, workqueue, softirq, RCU callback, or mutex at EL2; locking uses `hyp_spin_lock`. Do not flag single-CPU preemption races inside an EL2 handler, while still checking cross-CPU concurrency on shared EL2 structures.

## Host De-Privilege Boundary (`finalize_pkvm`)

* `finalize_pkvm()` (`arch/arm64/kvm/pkvm.c`, registered at `device_initcall_sync`) calls `pkvm_drop_host_privileges()` and enables the `kvm_protected_mode_initialized` static key (`is_pkvm_initialized()`).
* **Pre-de-privilege**: Initcalls before `device_initcall_sync` (including `kvm_arm_init` at `device_initcall`), `__init`/`__initdata` functions, and `__pkvm_init` / `__pkvm_init_finalise` (`arch/arm64/kvm/hyp/nvhe/setup.c`). Host memory is still trusted.
* **Post-de-privilege**: `late_initcall` and runtime hypercalls. Note that `is_protected_kvm_enabled()` is a boot-time mode predicate, whereas `is_kvm_arm_initialised()` / `is_pkvm_initialized()` test whether de-privilege has occurred.
* **Hypercall ID Bands (`enum __kvm_host_smccc_func`)**: IDs before `__KVM_HOST_SMCCC_FUNC_MIN_PKVM` are init-only; between `MIN_PKVM` and `__KVM_HOST_SMCCC_FUNC_PKVM_ONLY` are always-on; after `PKVM_ONLY` are pKVM-finalised-only (`handle_host_hcall` in `arch/arm64/kvm/hyp/nvhe/hyp-main.c`).
* Runtime hypercall handlers must never reference `__init` or `__initdata` symbols.

## Post-De-Privilege Trust Boundary and `kern_hyp_va`

* **Copy-Then-Validate on Host Memory**: Post-de-privilege, host memory dereferenced via `kern_hyp_va()` is subject to TOCTOU and double-fetch attacks. Copy host-shared structures into EL2-private memory (e.g., stack local via `memcpy`, avoiding large ~4KB stack structs like `struct kvm_vcpu`) once before validating or acting on them. Scalars passed in registers (`DECLARE_REG`) are already EL2-private.
* **Allocation Sources**: EL2 must never allocate from a free list or memcache whose head pointer resides in host memory (such as `stage2_teardown_mc`, which is only a sink for host-bound reclaim pages).
* **`struct pkvm_hyp_vm` and `struct pkvm_hyp_vcpu` Back-Pointers**: `hyp_vm->kvm` and `hyp_vcpu->vcpu` are embedded EL2-private copies (trusted); `hyp_vm->host_kvm` and `hyp_vcpu->host_vcpu` point to untrusted host memory.
* **`kern_hyp_va()` Idempotence**: `__kern_hyp_va` (`arch/arm64/include/asm/kvm_mmu.h`) is a mask-and-tag operation with no offset accumulation.
  * It is **idempotent (a harmless no-op)** on any pointer that already carries the hyp tag: the output of a prior `kern_hyp_va()` and any EL2 linear-map pointer (`hyp_phys_to_virt`, `hyp_page_to_virt`, `pkvm_hyp_vm`, `pkvm_hyp_vcpu`, `&hyp_vm->kvm`). Do not flag double `kern_hyp_va()` on linear-map pointers as memory corruption.
  * It **corrupts** EL2 private-VA-range pointers allocated via `pkvm_alloc_private_va_range()` (`nvhe/mm.c`): `hyp_vmemmap`, fixmap slots, MMIO mappings, and hyp stacks.

## EL2 Memory Management, Ownership, and Interfaces

* **`hyp_vmemmap` Zero-Initialization**: Page ownership metadata in `hyp_vmemmap` (`arch/arm64/kvm/hyp/include/nvhe/memory.h`) uses complement encoding where zero-filled memory evaluates to `PKVM_NOPAGE`. Never test ownership by comparing raw state with zero.
* **EL2 Buddy Allocator (`struct hyp_pool`, `nvhe/page_alloc.c`)**:
  * `pool->lock` protects both the buddy tree and `struct hyp_page::{refcount,order}`. Never read or mutate `refcount` or `order` outside `pool->lock`.
  * Tail pages of high-order blocks have `order == HYP_NO_ORDER`.
  * `hyp_alloc_pages()` returning `NULL` (`-ENOMEM`) is a normal runtime outcome and must not trigger `WARN_ON`/`BUG_ON`.
  * Pages must be freed (`hyp_put_page`) to the same pool (`hpool` vs `hyp_vm->pool`) from which they were allocated; `__hyp_attach_page` intentionally accepts donated pages outside `[range_start, range_end)` at order 0.
* **Per-CPU Fixmap (`hyp_fixmap_map` / `hyp_fixmap_unmap`, `nvhe/mm.c`)**: Non-nestable per-CPU slot; `hyp_fixmap_unmap()` performs the required TLB invalidation and must be reached on every exit path.
* **Page-Ownership Transitions**: Validate base/size against current ownership metadata before mutating state; roll back partial metadata updates if a subsequent mapping call (`kvm_pgtable_stage2_map`, `pkvm_create_mappings_locked`) fails; enumerate dying guest pages from ownership metadata rather than walking stage-2 page tables.
* **Stage-2 Huge Pages**: Validate block-mapping granule against the requested fault granule for protected VMs, and compute fault ranges from the IPA and page-table level rather than the host-supplied VMA shift.
* **FF-A Validation**: Bounds-check host-supplied `offset` and `length` fields in FF-A memory-sharing descriptors against the buffer size, enforce negotiated FF-A versions with acquire/release ordering, and mask unimplemented optional interfaces in `FFA_FEATURES`.
* **SMC Forwarding**: Reject host `SMC` instructions with non-zero `imm16` (`imm16 != 0`) before forwarding to EL3.

## Traps, `HCR_EL2`, FPSIMD/SVE, and Sync Seams

* **Trap Registers & FGT**: Initialize trap registers (`HFGRTR_EL2`, `MDCR_EL2`, etc.) from EL2-private state. Note that FGT registers (`HFGRTR_EL2`, `HFGWTR_EL2`, `HFGITR_EL2`, `HDFGRTR_EL2`, `HDFGWTR_EL2`, `HAFGRTR_EL2`) UNDEF at EL1/EL0 (absent nested virt), so reading them back at EL2 is safe; modifying FGT registers requires a Context Synchronization Event (`isb` or exception return) to take effect.
* **Protected vCPU `HCR_EL2` Allowlist**: Regime/configuration bits (`VM`, `RW`, `TGE`, `IMO`, `FMO`, `AMO`, `TSC`, `E2H`, `TID*`, `TACR`) are fixed at hyp vCPU init (`pkvm_vcpu_reset_hcr`, `pvm_init_traps_hcr`) and must never be sourced from the host. Conversely, the per-entry host-owned runtime signal allowlist (`HCR_TWI | HCR_TWE | HCR_VSE` in `flush_hyp_vcpu`) must include `HCR_VSE` so host-injected virtual SErrors reach the guest.
* **FPSIMD/SVE Save Regime (`kvm_hyp_handle_fpsimd`, `hyp/switch.h`)**: Keyed off the global `is_protected_kvm_enabled()` mode, not per-VM protection. When pKVM is enabled, `kvm_hyp_save_fpsimd_host()` eagerly saves host FP/SVE state at `kvm_host_sve_max_vl` for every guest (protected or not). When pKVM is disabled (standard nVHE), the host's lazy FPSIMD machinery handles host state and `fpsimd_lazy_switch_to_guest()` / `fpsimd_lazy_switch_to_host()` are used instead.
* **Per-Load vs Per-Entry Seams**:
  * **Per-load (`handle___pkvm_vcpu_load` -> `pkvm_load_hyp_vcpu`)**: Copies coarse state such as `arch.fgt` for non-protected guests.
  * **Per-entry (`flush_hyp_vcpu` / `sync_hyp_vcpu`)**: Syncs fine-grained per-entry state (`hcr_el2`, `mdcr_el2`, `arch.iflags`). Do not flag per-load state as "missing" from `flush_hyp_vcpu`.
* **Speculative Engine Synchronization**: When disabling asynchronous profiling/trace engines (e.g., SPE/TRBE) before switching translation regimes (`VTTBR_EL2` / `TTBRn_ELx`), execute the required architectural synchronization sequence (e.g., `PSB CSYNC` / `TSB CSYNC`) followed by a speculative execution barrier (`SB` or `DSB` + `ISB`).
