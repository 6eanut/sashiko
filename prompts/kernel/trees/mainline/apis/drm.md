# DRM, GPU Driver, & Overflow/Property Helper APIs

## `struct_size()` & `array_size()` Overflow Semantics (`include/linux/overflow.h`)

- `struct_size(p, member, count)` and `array_size(a, b)` return `size_t` and **saturate to `SIZE_MAX`** on overflow.
- Checking `if (sz > SIZE_MAX)` (even when stored in a `u64`) is **always false (dead code)**. Callers must either check `if (sz == SIZE_MAX)` or pass the result directly to `kzalloc(..., GFP_KERNEL | __GFP_NOWARN)` (which fails cleanly on `SIZE_MAX`).

## `fwnode_create_software_node()` Error Convention (`include/linux/property.h`)

- `fwnode_create_software_node()` returns `ERR_PTR(...)` on error, **never `NULL`**. Checking `if (!fwnode)` instead of `if (IS_ERR(fwnode))` fails to catch allocation errors and dereferences an error pointer.
- By contrast, `fwnode_graph_get_*()` helpers return `NULL` on failure, and `software_node_register()` returns `int`.

## `stack_depot_save()` Initialization Prerequisite (`include/linux/stackdepot.h`)

- Any code path calling `stack_depot_save()` (for example under a debug Kconfig such as `CONFIG_DRM_XE_DEBUG_GUC`) requires `stack_depot_init()` to have been called during subsystem initialization (e.g., in `xe_guc_ct_init_noalloc()`) under the identical Kconfig guard, unless the Kconfig selects `STACKDEPOT_ALWAYS_INIT`.

## Driver-Specific Accessor & Allocation Contracts (`drivers/gpu/drm/`)

- **Intel Xe GT lookup (`drivers/gpu/drm/xe/`):**
  - `xe_device_get_gt(xe, gt_id)` returns `NULL` for invalid or absent `gt_id`; callers must check for `NULL` before dereferencing.
  - `xe_root_mmio_gt(xe)` returns the root tile's primary GT for non-GT MMIO and does not return `NULL` on initialized devices.
- **Intel i915/Xe CX0 PHY transactions (`drivers/gpu/drm/i915/display/intel_cx0_phy.c`):**
  - Calls to `intel_cx0_read()`, `intel_cx0_write()`, or `intel_cx0_rmw()` must run inside an active `wakeref = intel_cx0_phy_transaction_begin(encoder)` / `intel_cx0_phy_transaction_end(encoder, wakeref)` scope.
  - Accessing internal C10 PHY registers (e.g., `PHY_CMN1_CONTROL`) via MsgBus when `intel_encoder_is_c10phy(encoder)` is true requires setting `C10_VDR_CTRL_MSGBUS_ACCESS` in `PHY_C10_VDR_CONTROL(1)` first.
- **MSM DRM VM lazy creation (`drivers/gpu/drm/msm/`):**
  - `ctx->vm` is lazily initialized on demand and may be `NULL` in early ioctl paths or after `VM_BIND` opt-in before first mapping. Use `msm_context_vm(dev, ctx)` rather than dereferencing `ctx->vm` directly in ioctl entry/validation paths.
- **AMDGPU kernel BO allocation (`drivers/gpu/drm/amd/amdgpu/amdgpu_object.c`):**
  - `amdgpu_bo_create_kernel(adev, size, align, domain, bo_ptr, gpu_addr, cpu_addr)` only allocates a new BO if `*bo_ptr == NULL`; if `*bo_ptr != NULL`, it attempts to pin the existing pointer. Any wrapper accepting an opaque `void **buf_obj` from external callers to allocate a new BO must initialize `*bo = NULL` before calling `amdgpu_bo_create_kernel()` or `amdgpu_bo_create_reserved()`.
- **AMDGPU VM Page Directory address (`amdgpu_gmc_pd_addr()` vs `amdgpu_bo_gpu_offset()`):**
  - Debugfs/diagnostic interfaces exporting VM root or page-directory addresses for tools like UMR must use `amdgpu_gmc_pd_addr(vm.root.bo)`, not `amdgpu_bo_gpu_offset()`.
