# Kernel Selftests Build, KVM, and Netns Mechanics (`tools/testing/selftests/`)

## 1. Selftests Makefile Installation Variables (`lib.mk`)

| Variable | Purpose |
| :--- | :--- |
| `TEST_PROGS` | Executable scripts invoked directly by the test runner |
| `TEST_FILES` | Non-built supporting files installed alongside tests (sourced `.sh` libraries, imported `.py` modules, data files) |
| `TEST_GEN_PROGS` | Compiled executable test binaries run directly by the test runner |
| `TEST_GEN_FILES` | Compiled helper binaries or generated files invoked by tests but not run directly as top-level tests |

- **Dependency Invariant**: Any shell file referenced via `source <file>` or `. <file>` and any Python module referenced via `import <mod>` **must** be listed in `TEST_FILES`. Failure to do so results in `No such file or directory` errors after `make install`.
- **Build Targets**: All new test subdirectories must be explicitly added to the parent `Makefile`'s `TARGETS` list to ensure inclusion in CI coverage.

## 2. KVM Selftests: `vm_create_with_one_vcpu()` and IRQ Chip Finalization

- `vm_create(nr_runnable_vcpus)` allocates the VM and memory regions (`kvm_arch_vm_post_create()`) but does **not** create vCPUs or call `kvm_arch_vm_finalize_vcpus()`. On arm64, vGIC finalization (`KVM_DEV_ARM_VGIC_CTRL_INIT`) requires all vCPUs to exist first, and on architectures without a default in-kernel IRQ chip (`riscv`, `loongarch`), IRQ chip ioctls return `-ENODEV`.
- Any KVM selftest using `KVM_IRQFD`, `KVM_IRQ_LINE`, or IRQ routing must:
  1. Gate execution with `TEST_REQUIRE(kvm_arch_has_default_irqchip());`
  2. Create the VM and vCPU(s) via `vm_create_with_one_vcpu()` or `__vm_create_with_vcpus()` (not bare `vm_create()`) before issuing IRQ ioctls.

## 3. Network Namespace Selftests: `init_net` IPv4 Config Inheritance

- At the default `net.core.devconf_inherit_init_net = 0`, newly created network namespaces use compiled defaults for IPv6 (`addrconf_init_net()`), but **copy `conf/all` and `conf/default` from `init_net` for IPv4** (`devinet_init_net()`).
- On developer or container hosts where `net.ipv4.conf.all.forwarding=1` (or custom `rp_filter`, `accept_local`, etc.), new netns devices inherit those host settings unless the test explicitly writes `/proc/sys/net/ipv4/conf/{all,default}/...` inside the netns.
- In negative tests asserting that an operation fails with a specific errno, verify that a kernel missing the targeted check would not return the exact same errno due to an unconfigured precondition (such as a missing route, address, or disabled interface).

## 4. Harness and Resource Management (`kselftest_harness.h`)

- **Lifecycle**: `FIXTURE_SETUP` and `FIXTURE_TEARDOWN` execute per `TEST_F`. The `self` pointer is unique to each instance. Objects allocated in `FIXTURE_SETUP` must be freed in `FIXTURE_TEARDOWN`.
- **Error Handling**: Every `malloc` or `open` must have a corresponding `free` or `close` on all exit paths, including `ASSERT_*` failures. Initialize local structs to `{0}` to prevent use of uninitialized stack memory in ioctl or netlink responses.
- **Concurrency**: Selftests are single-threaded by default. If spawning threads, implement explicit synchronization. Use dynamic timeouts or `poll()`/`select()` instead of hardcoded `sleep()` to account for background kernel activity (e.g., page reclaim).
- **Logging**: Use `TH_LOG()` for diagnostic output. If a test requires specific hardware or kernel config, use `ksft_test_result_skip()` to report the skip correctly to the TAP runner.

## 5. Subsystem Invariants and Verification Contracts

- **Initialization Sequencing**: A newly allocated object must be fully configured before being published to global lists or user-space handles. Use `ksft_set_plan()` to define the test scope accurately; mismatches between the plan and actual assertions cause harness failures.
- **Memory Safety**: When parsing complex nested structures from untrusted sources (e.g., netlink, ioctl buffers), validate all length fields and array indices against the target object's dynamic limits to prevent out-of-bounds access.
- **Asynchronous Teardown**: Ensure all pending deferred work, timers, and callbacks are synchronously canceled before the memory backing their execution context is released.
- **Error Propagation**: When a lower-layer function returns an error pointer sentinel (e.g., `ERR_PTR`), distinguish this from `NULL` before evaluation. Ensure error codes are propagated without being masked by subsequent cleanup operations.
- **State Machine Integrity**: Validate all state transitions against current hierarchy constraints (e.g., cgroup partitions) before application. Ensure that lookup keys are uniquely derived from all relevant attributes to prevent hash table collisions.
- **Clock Helpers**: When using `clock_name()` or `CLOCK_AUX` constants from `clock-helpers.h`, ensure the clockid is within the valid range `[CLOCK_AUX, CLOCK_AUX_LAST]` to avoid invalid clock lookups.
- **Synchronization**: Use `__kselftest_memset_safe` for clearing sensitive buffers to prevent compiler optimization of security-critical zeroing.
- **Test Plan Integrity**: The `ksft_count` struct tracks test results; ensure `ksft_finished()` is called to finalize the TAP output, preventing partial test reports in CI pipelines.
- **Atomic Contexts**: Avoid `malloc` or blocking syscalls inside `TEST_F` if the test logic is intended to simulate atomic kernel contexts; use pre-allocated buffers in `FIXTURE_SETUP`.
- **Resource Cleanup**: Always use `ksft_exit_fail_perror()` when a critical resource (e.g., `/dev/kvm`) fails to open, ensuring the test runner captures the specific system error.
- **Input Validation**: When testing ioctls, verify that the `struct` size matches the kernel's expectation to prevent `EFAULT` or `EINVAL` due to padding mismatches across architectures.
- **Tracepoint Consistency**: Ensure that any tracepoints used for test verification are enabled via `tracefs` before the test trigger, and disabled in `FIXTURE_TEARDOWN` to avoid polluting global trace buffers.
- **Pointer Safety**: Always check for `NULL` after `malloc` and before dereferencing, even in test code, to avoid segmentation faults that mask the actual test failure.
- **Alignment**: Ensure that buffers passed to `read()` or `write()` on character devices are aligned to the device's required block size to avoid `EINVAL` on strict hardware.
