# KVM Core Helper APIs (`include/linux/kvm_host.h`)

## Memslot HVA/PFN Resolution Helpers

* **SRCU Precondition**: Calling `gfn_to_hva*()`, `gfn_to_pfn*()`, or accessing `kvm_memslots()` / `kvm_vcpu_memslots()` requires holding `srcu_read_lock(&kvm->srcu)` or `kvm->slots_lock`.
* **Write-Intent vs Read-Only Memslots (`KVM_MEM_READONLY`)**:
  * `gfn_to_hva()`, `gfn_to_hva_many()`, and `gfn_to_hva_memslot()` request *writable* translations and return `KVM_HVA_ERR_RO_BAD` on valid `KVM_MEM_READONLY` slots (such as ROM or ARM64 UEFI flash regions).
  * Read-access fault, prefetch, and emulation paths must use `gfn_to_hva_memslot_prot(..., &writable)` or read-intent variants so read-only memslots succeed.

## vCPU Attachment and Request Helpers

* **`vcpu_load()` / `vcpu_put()`**: Must be paired around any ioctl or code path that accesses physical-CPU-dependent vCPU state (hardware registers, timers, preempt notifiers). Omitting `vcpu_put()` leaks preempt notifiers and crashes in `kvm_sched_out()`.
* **`kvm_make_request()` / `kvm_check_request()`**: `kvm_make_request()` issues `smp_wmb()` before setting the request bit, paired with `smp_mb__after_atomic()` in `kvm_check_request()` (and `smp_mb()` + `IN_GUEST_MODE` in `kvm_vcpu_kick()`). Do not add redundant manual memory barriers around `kvm_make_request()`.
