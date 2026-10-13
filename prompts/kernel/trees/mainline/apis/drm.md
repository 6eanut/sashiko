# DRM, GPU Driver, & Overflow/Property Helper APIs (v7.3-rc5)

This guide provides authoritative rules for the Linux v7.3-rc5 kernel, focusing on memory safety, object lifecycles, and subsystem-specific invariants for DRM and GPU drivers.

---

## 1. Overflow & Allocation Safety (`include/linux/overflow.h`)

- **`struct_size(p, member, count)` & `array_size(a, b)`:** These helpers return `size_t` and **saturate to `SIZE_MAX`** on overflow.
- **Verification:** Checking `if (sz > SIZE_MAX)` is dead code. Callers must either check `if (sz == SIZE_MAX)` or pass the result directly to `kzalloc(..., GFP_KERNEL | __GFP_NOWARN)`, which fails cleanly on `SIZE_MAX`.
- **Arithmetic:** Always use `check_add_overflow()` or `check_sub_overflow()` for buffer offsets; never manually compare against `SIZE_MAX`.

## 2. Property & Firmware Node Helpers (`include/linux/property.h`)

- **Error Convention:** `fwnode_create_software_node()` returns `ERR_PTR(...)` on error, **never `NULL`**. Checking `if (!fwnode)` instead of `if (IS_ERR(fwnode))` fails to catch allocation errors and dereferences an error pointer.
- **NULL Sentinels:** `fwnode_graph_get_*()` helpers return `NULL` on failure. `software_node_register()` returns `int`.

## 3. Stack Depot Initialization (`include/linux/stackdepot.h`)

- **Prerequisite:** Any code path calling `stack_depot_save()` (e.g., under `CONFIG_DRM_XE_DEBUG_GUC`) requires `stack_depot_init()` to have been called during subsystem initialization (e.g., in `xe_guc_ct_init_noalloc()`) under the identical Kconfig guard, unless the Kconfig selects `STACKDEPOT_ALWAYS_INIT`.

## 4. Driver-Specific Accessor & Allocation Contracts

### Intel Xe GT (`drivers/gpu/drm/xe/`)
- `xe_device_get_gt(xe, gt_id)` returns `NULL` for invalid or absent `gt_id`; callers must check for `NULL` before dereferencing.
- `xe_root_mmio_gt(xe)` returns the root tile's primary GT for non-GT MMIO and does not return `NULL` on initialized devices.

### Intel i915/Xe CX0 PHY (`drivers/gpu/drm/i915/display/intel_cx0_phy.c`)
- Calls to `intel_cx0_read()`, `intel_cx0_write()`, or `intel_cx0_rmw()` must run inside an active `wakeref = intel_cx0_phy_transaction_begin(encoder)` / `intel_cx0_phy_transaction_end(encoder, wakeref)` scope.
- Accessing internal C10 PHY registers (e.g., `PHY_CMN1_CONTROL`) via MsgBus when `intel_encoder_is_c10phy(encoder)` is true requires setting `C10_VDR_CTRL_MSGBUS_ACCESS` in `PHY_C10_VDR_CONTROL(1)` first.

### MSM DRM VM (`drivers/gpu/drm/msm/`)
- `ctx->vm` is lazily initialized and may be `NULL` in early ioctl paths or after `VM_BIND` opt-in before first mapping. Use `msm_context_vm(dev, ctx)` rather than dereferencing `ctx->vm` directly.

### AMDGPU Kernel BO Allocation (`drivers/gpu/drm/amd/amdgpu/amdgpu_object.c`)
- `amdgpu_bo_create_kernel(adev, size, align, domain, bo_ptr, gpu_addr, cpu_addr)` only allocates a new BO if `*bo_ptr == NULL`.
- **Invariant:** Any wrapper accepting an opaque `void **buf_obj` must initialize `*bo = NULL` before calling `amdgpu_bo_create_kernel()` or `amdgpu_bo_create_reserved()`.

### AMDGPU VM Page Directory (`drivers/gpu/drm/amd/amdgpu/`)
- **`amdgpu_gmc_pd_addr()`:** Debugfs/diagnostic interfaces exporting VM root or page-directory addresses for tools like UMR must use `amdgpu_gmc_pd_addr(vm.root.bo)`, not `amdgpu_bo_gpu_offset()`.

---

## 5. Engineering Principles for Reviewers

1. **Object Lifecycle:** When a lookup yields a shared object, verify if the caller assumes ownership of a new reference. If initialization fails, ensure the rollback order decrements usage counters in the exact reverse of the acquisition order.
2. **Execution Context:** Operations involving `intel_cx0_phy` or hardware register programming must not yield the processor. If a blocking operation (e.g., firmware load) is required, ensure it is not called from an atomic context.
3. **Concurrency:** When iterating over shared lists (e.g., RCU), ensure objects are not prematurely destroyed. Use `rcu_read_lock()` and verify that the object is not freed until the grace period expires.
4. **Registration:** Never expose a device or object to userspace until its internal state is fully validated and initialized. Use `device_add()` only after all mandatory properties and state structures are configured.
5. **Boundary Validation:** Always validate user-provided lengths against buffer capacity. Use `struct_size()` and `array_size()` to calculate allocation sizes to prevent integer overflows.
6. **Error Propagation:** Always propagate error codes from hardware interactions back to the user-space boundary. Use `IS_ERR()` for pointer returns and check for `NULL` on lookup functions like `xe_device_get_gt()`.
7. **State Idempotency:** Ensure that hardware-specific state registers are explicitly cleared when an object is disabled or released to prevent stale state interference.
8. **Async Work:** Use managed cleanup to ensure all background tasks are stopped before device destruction to prevent use-after-free vulnerabilities.
9. **Atomic State:** When comparing atomic states, ensure the comparison is between the old and new state objects, never against the state itself.
10. **Interrupt Robustness:** Ensure interrupt handlers are robust against hardware power-gating (e.g., PSR/IPS) that might mask expected signals.
