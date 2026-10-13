# KVM Subsystem Maintainer Notes

## Feature Enumeration, Immutability, and Assertions

* **New Guest-Visible Features Default Off and Are Enumerable**: Any new guest-observable behavior (ioctl, VM/vCPU capability, exit reason, or emulated instruction) must default to off and be discoverable via the architecture's enumeration interface (`KVM_CHECK_EXTENSION` / `KVM_CAP_*`, `KVM_GET_SUPPORTED_CPUID2` on x86, or ID-register feature bits on ARM64) to preserve live migration compatibility.
* **New Memslot and vCPU Flags Default to Immutable**: New memslot flags (e.g., `KVM_MEM_GUEST_MEMFD`) and post-init vCPU configuration flags should be set-once unless mutability is explicitly justified and handled across all state transitions.
* **No Adversary-Reachable `WARN_ON` / `BUG_ON`**: Never place a `WARN_ON` or `BUG_ON` on a condition reachable from untrusted guest actions or unprivileged host userspace ioctls; return an error code or use `pr_warn_once()` if logging is needed.
