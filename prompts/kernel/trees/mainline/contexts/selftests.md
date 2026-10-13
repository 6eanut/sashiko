# Kernel Selftests Build, KVM, and Netns Mechanics (`tools/testing/selftests/`)

## Selftests Makefile Installation Variables (`lib.mk`)

| Variable | Purpose |
| :--- | :--- |
| `TEST_PROGS` | Executable scripts invoked directly by the test runner |
| `TEST_FILES` | Non-built supporting files installed alongside tests (sourced `.sh` libraries, imported `.py` modules, data files) |
| `TEST_GEN_PROGS` | Compiled executable test binaries run directly by the test runner |
| `TEST_GEN_FILES` | Compiled helper binaries or generated files invoked by tests but not run directly as top-level tests |

- Any shell file referenced via `source <file>` or `. <file>` and any Python module referenced via `import <mod>` **must** be listed in `TEST_FILES`, or the test will pass in-tree and fail with `No such file or directory` after `make install`.

## KVM Selftests: `vm_create_with_one_vcpu()` and IRQ Chip Finalization

- `vm_create(nr_runnable_vcpus)` allocates the VM and memory regions (`kvm_arch_vm_post_create()`) but does **not** create vCPUs or call `kvm_arch_vm_finalize_vcpus()`. On arm64, vGIC finalization (`KVM_DEV_ARM_VGIC_CTRL_INIT`) requires all vCPUs to exist first, and on architectures without a default in-kernel IRQ chip (`riscv`, `loongarch`), IRQ chip ioctls return `-ENODEV`.
- Any KVM selftest using `KVM_IRQFD`, `KVM_IRQ_LINE`, or IRQ routing must:
  1. Gate execution with `TEST_REQUIRE(kvm_arch_has_default_irqchip());`
  2. Create the VM and vCPU(s) via `vm_create_with_one_vcpu()` or `__vm_create_with_vcpus()` (not bare `vm_create()`) before issuing IRQ ioctls.

## Network Namespace Selftests: `init_net` IPv4 Config Inheritance

- At the default `net.core.devconf_inherit_init_net = 0`, newly created network namespaces use compiled defaults for IPv6 (`addrconf_init_net()`), but **copy `conf/all` and `conf/default` from `init_net` for IPv4** (`devinet_init_net()`).
- On developer or container hosts where `net.ipv4.conf.all.forwarding=1` (or custom `rp_filter`, `accept_local`, etc.), new netns devices inherit those host settings unless the test explicitly writes `/proc/sys/net/ipv4/conf/{all,default}/...` inside the netns.
- In negative tests asserting that an operation fails with a specific errno, verify that a kernel missing the targeted check would not return the exact same errno due to an unconfigured precondition (such as a missing route, address, or disabled interface).
