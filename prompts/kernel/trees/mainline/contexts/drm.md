# DRM / GPU Subsystem Context (`drivers/gpu/drm/`)

## Atomic Context in Display Hardware Programming
Display driver paths reachable during non-blocking atomic commits, VBLANK interrupts, page-flip completions, CRTC/plane/encoder atomic callbacks, or AMD display hardware sequencer (`hwseq` / `hw_sequencer`) hooks must not sleep:

- **Prohibited:** `fsleep()`, `msleep()`, `usleep_range()`, `mutex_lock()`, or `GFP_KERNEL` allocations.
- **Required:** Use `udelay()` / `ndelay()` in atomic display commit paths.
- **Verification:** When replacing a busy-wait hardware polling helper (such as `wait_for_blank_complete()`) with a delay, verify whether callers run in atomic context before introducing sleeping delays.

## `drm_gpuvm` Mandatory IOMMU Requirement
`drm_gpuvm` does not support physical-address/no-IOMMU fallback modes. When converting a driver (e.g., `msm_kms_init_vm()`, `mdp4_kms_init()`) to `drm_gpuvm`, any legacy path that continued with `vm = NULL` when `!mmu` must instead fail and return an error (`ERR_PTR(-ENODEV)`).

## Intel i915 & Xe Driver Invariants
- **System PM vs Runtime PM (`drivers/gpu/drm/xe/xe_pm.c`):** System suspend/resume (`xe_pm_suspend()`, `xe_pm_resume()`) always loses device power regardless of D3cold policy. Never gate hardware reinitialization in system PM paths on runtime PM flags such as `xe->d3cold.allowed` or `xe->d3cold.capable`.
- **Pcode Mailbox Read-Modify-Write (`xe_pcode_*`):** Pcode mailbox registers pack multiple fields (e.g., `PWR_LIM_EN`, `PWR_LIM_VAL`, `PWR_LIM_TIME`). Updating a single field after `xe_pcode_read()` requires masked RMW (`val0 = (val0 & ~mask) | field_val`), never direct assignment (`val0 = uval`).
- **SR-IOV VF Probe Guards (`IS_SRIOV_VF(xe)`):** PF-only hardware resources (I2C controllers, direct `SOC_BASE` MMIO, root-device interrupts, direct PCI config space, GuC firmware loading) are inaccessible from a Virtual Function; new `xe_*_probe()` functions touching PF-only hardware must guard entry with `if (IS_SRIOV_VF(xe)) return 0;`.
- **Platform vs Subplatform Registration (`include/drm/intel/pciids.h`, `intel_display_device.c`, `xe_pci.c`):**
  - When new PCI device IDs share a major platform architecture but differ in IP version, PHY topology, or PCH configuration (e.g., WCL vs PTL under `display->platform.pantherlake`), they require a distinct PCI ID macro and subplatform registration in both `i915` and `xe`.
  - Do not widen a shared platform-flag condition (e.g., `intel_encoder_is_c10phy()`) across ports/PHYs unless verified safe for every hardware variant sharing that platform flag (accounting for VBT port enumeration).
- **AUXLess ALPM Register Access:** Functions programming AUXLess ALPM PHY registers must return early before touching PHY registers when `!intel_alpm_is_alpm_aux_less(...)`, rather than writing `0` to the register bits unconditionally.

## AMD AMDGPU & Display Invariants
- **Ring Write Pointer (`wptr`) Width:** `ring->wptr` and `fence->wptr` are `u64`. All helper parameters and local variables representing ring write pointers must use `u64` (never `u32`, which truncates large rings).
- **Fence Sequence Wrap-Around (`amdgpu_fence_driver`):** `sync_seq` is a cyclic `u32` counter indexing `fences[seq & num_fences_mask]`. Never iterate pending fences with `for (i = seqno + 1; i <= ring->fence_drv.sync_seq; ++i)` (which fails when `sync_seq` wraps around `0xFFFFFFFF`); advance and compare masked indices (`last_seq = (last_seq + 1) & mask; ... while (last_seq != seq)`).
- **PSP GFX Command Version Guards:** Submitting a newer `GFX_CMD_ID_*` command via `psp_cmd_submit_buf()` or `psp_get_fw_reservation_info()` requires checking **both** hardware IP version (`amdgpu_ip_version(adev, MP0_HWIP, 0)`) and SOS firmware version (`adev->psp.sos.fw_version`), since newer hardware can boot with older PSP firmware.
- **Automatic Display Scaling Default (`RMX_*`):** When enabling automatic panel scaling for non-native resolutions without an explicit userspace scaling request, default to `RMX_ASPECT` (aspect-ratio preserving), never `RMX_FULL` (which stretches and distorts content).

## DisplayPort DPCD Probing & DRM Scheduler KUnit Tests
- **DPCD Link Training Status Registers (`0x202`–`0x207`):** Reading `DP_LANE0_1_STATUS` (`0x202`) or `DP_LANE2_3_STATUS` (`0x203`) outside link training can trigger sink state-machine side effects on non-compliant panels. Generic DPCD probing (`drm_dp_dpcd_probe()`) should use side-effect-free capability/config registers such as `DP_DPCD_REV` (`0x000`) or `DP_TRAINING_PATTERN_SET` (`0x102`).
- **DRM Scheduler Mock Timeout Flags (`drivers/gpu/drm/scheduler/tests/mock_scheduler.c`):** Because tests waiting across multiple `MOCK_TIMEOUT` intervals can invoke `mock_sched_timedout_job()` multiple times, timeout handlers must never clear input control flags (e.g., `DRM_MOCK_SCHED_JOB_DONT_RESET`); instead keep control flags immutable and set a separate sticky status flag (e.g., `DRM_MOCK_SCHED_JOB_RESET_SKIPPED`).

## Synchronization and Concurrency Contracts
- **Lock Hierarchy:** DRM drivers must adhere to the `drm_modeset_lock` hierarchy. When acquiring multiple locks, use `drm_modeset_lock_all_ctx()` to prevent circular dependencies.
- **Interrupt Safety:** Use `spin_lock_irqsave()` when accessing shared hardware state (e.g., ring buffers, interrupt masks) that may be modified by both process-context ioctls and hard IRQ handlers.
- **Atomic Context:** Never perform `mutex_lock` or `GFP_KERNEL` allocations in `drm_atomic_helper_commit_tail` or any callback invoked from the atomic commit path.

## Object Lifecycle and Teardown
- **Reference Counting:** Always balance `drm_gem_object_get()` with `drm_gem_object_put()`. In error paths, ensure that `drm_gem_object_put()` is called if the object was successfully initialized but failed to attach to the device.
- **Teardown Quiescing:** Before destroying a `drm_device`, ensure all pending `drm_sched_job` instances are canceled and the GPU is quiesced. Use `drm_sched_entity_destroy()` to ensure no further jobs are submitted during teardown.
- **Error Unwinding:** Use `devm_` managed resources where possible to simplify error paths, but ensure that manual cleanup (e.g., `drm_mode_config_cleanup`) is performed in the exact reverse order of initialization to avoid use-after-free on shared structures.
- **Resource Lookups:** Always validate the result of a GEM object lookup from userspace handles (e.g., `drm_gem_object_lookup()`) for `NULL` before dereferencing, as the handle may be invalid or already closed.
- **Accelerator Context:** For `drm_accel` drivers, ensure `DEFINE_DRM_ACCEL_FOPS` is used to correctly bind `THIS_MODULE` to the file operations, preventing module unloading while the device is in use.
