# ARM GICv3 / GICv4 Subsystem Context (`drivers/irqchip/irq-gic-v3*`, `arch/arm64/kvm/vgic/`)

## ITS Command Queue (`drivers/irqchip/irq-gic-v3-its.c`)

- **Queue publish barrier (`its_flush_cmd()`):** The canonical sequence is `its_allocate_entry()` -> builder -> `its_flush_cmd()` -> optional SYNC/VSYNC entry -> sample `GITS_CREADR` -> `its_post_commands()` (`GITS_CWRITER`) -> unlock -> `its_wait_for_range_completion()`. `its_flush_cmd()` must branch on `ITS_FLAGS_CMDQ_NEEDS_FLUSHING` between `gic_flush_dcache_to_poc()` and `dsb(ishst)` before writing the `GITS_CWRITER` doorbell.
- **Two-level ITS table publication:** A write setting `Valid = 1` on a Level-1 ITS table entry must be globally visible before queuing any ITS command that references that Level-2 table.
- **Sampling `GITS_CREADR` before ringing:** Sample `rd_idx = readl_relaxed(its->base + GITS_CREADR)` *before* `its_post_commands(its)` while holding `its->lock`, then wait outside the lock. Ring-offset comparisons must linearize wrap-around (`ITS_CMD_QUEUE_SZ`).
- **Endianness fixup (`its_fixup_cmd()`):** Every command builder return path (including early exits via `goto out;`) must pass through `its_fixup_cmd()` (`cpu_to_le64()`).
- **Queue full & stall handling:** `its_queue_full()` stops one entry short of wrapping onto `ridx` (`((widx + 1) % ITS_CMD_QUEUE_NR_ENTRIES) == ridx`). On a command error, the architecture permits CONSTRAINED UNPREDICTABLE choice of ignoring, stalling (`GITS_CREADR.Stalled == 1`, restarted via `GITS_CWRITER.Retry`), or treating as valid; any ITS emulation that can set `Stalled` must implement `GITS_CWRITER.Retry`.
- **Command Function Builder:** Use `BUILD_SINGLE_CMD_FUNC` to define standard command emission wrappers.

## Completion: `SYNC` (Physical) vs `VSYNC` (Virtual)

- `its_send_single_command()` appends `SYNC` (physical interrupts on a target Redistributor `RDbase`); `its_send_single_vcommand()` appends `VSYNC` (virtual operations targeting a `vPEID`).
- **Builder return value controls sync emission:** Returning `NULL` from a command builder elides the trailing `SYNC`/`VSYNC`.
- **GICv4.1 `VMAPP` unmap is self-synchronizing:** On GICv4.1 (`is_v4_1(its)`), `VMAPP` with `{V, Alloc} == {0, x}` is architecturally self-synchronizing; issuing a trailing `VSYNC` is a command error (`VSYNC_VCPU_INVALID`). `its_build_vmapp_cmd()` returns `NULL` on the unmap path only when `is_v4_1(its)` is true.
- **`INV` and `INVALL` require `SYNC`; `INVDB` and `VSGI` require `VSYNC`** (automatically appended by `its_send_single_vcommand()`).

## LPI Configuration Table Sharing (`GICR_TYPER.CommonLPIAff`)

- **Shared `GICR_PROPBASER`, per-RD `GICR_PENDBASER`:** Redistributors in the same `GICR_TYPER.CommonLPIAff` group must share the same LPI Configuration table (`GICR_PROPBASER`) and, on GICv4.1, the same vPE Configuration table. The LPI Pending table (`GICR_PENDBASER`) is strictly per-Redistributor and must **never** be shared.
- **`CommonLPIAff` mask keeps high bits (`compute_common_aff()`):** Grouping by leading affinity fields masks off low bits: `aff & ~(GENMASK(31, 0) >> (clpiaff * 8))`.
- **Publishing LPI config updates:** Requires global visibility followed by `GICR_CTLR.EnableLPIs` (`0 -> 1`), `GICR_INVLPIR`/`GICR_INVALLR`, or ITS `INV`/`INVALL` + `SYNC`.
- **Policy cap on LPI ID width:** Host `lpi_id_bits` is capped to `ITS_MAX_LPI_NRBITS` (`16`) unless tables were pre-allocated by firmware.

## Redistributor & Distributor Write Completion (`RWP` vs Read-Back)

- **`GICD_CTLR` and `GICD_CTLR_RWP`:** The `GICD_CTLR` register (offset `0x0000`) contains the `GICD_CTLR_RWP` (bit 31) status. Always pair distributor base with `GICD_CTLR_RWP` (`gic_dist_wait_for_rwp()`) and redistributor base with `GICR_CTLR_RWP` (`gic_redist_wait_for_rwp()`).
- **What `RWP` tracks:** `GICD_CTLR.RWP` tracks group-enable transitions, `ARE`/`DS` transitions, and `GICD_ICENABLER<n>`. `GICR_CTLR.RWP` tracks `GICR_ICENABLER0`, `GICR_CTLR` `DPG*`, `EnableLPIs` `1 -> 0`, and on GICv4.1 `GICR_VPROPBASER.Valid` `1 -> 0`.
- **Non-tracked writes:** Enabling interrupts (`ISENABLER*`), group `0 -> 1` transitions, and priority/routing writes are **not** tracked by `RWP`; do not flag missing `RWP` polls after them.
- **Set-active / set-pending completion:** Requires MMIO read-back to ensure completion (e.g., `gic_irq_set_irqchip_state()`); do not flag this read-back as dead/redundant.

## GICv4 vPE Residency, Affinity, and Doorbells (`drivers/irqchip/irq-gic-v4.c`)

- **`GICR_VPENDBASER` rules:** Never modify any field other than `Valid` while `Valid == 1`. Wait for `GICR_VPENDBASER.Dirty == 0` before clearing `Valid` via `its_clear_vpend_valid()`. If `Dirty` times out, force `PendingLast = true`.
- **Lock hierarchy:** `vmapp_lock -> vpe_lock -> vmovp_lock`. Reading/updating `vpe->col_idx` requires `vpe_lock`. `its_send_vmovp()` serializes sequence numbers across all ITS instances under `vmovp_lock`.
- **Unmapped vs lazily-mapped vPE affinity (`its_vpe_set_affinity()`):** Guard `VMOVP` emission on `vpe->its_vm->vlpi_count` / `vmapp_count`.
- **WFI doorbell race:** KVM records WFI intent in an explicit vCPU flag under `preempt_disable()` and keys doorbell requests on that flag. Use `vcpu_load()` and `vcpu_put()` to manage residency.

## vGIC LPI Refcounting, XArray, and AP-List Migration (`arch/arm64/kvm/vgic/`)

- **Lookup under RCU/lock:** Lookups in `dist->lpi_xa` must call `vgic_try_get_irq_ref(irq)` *inside* `rcu_read_lock()`.
- **Atomic decrement + eviction:** `vgic_put_irq()` uses `refcount_dec_and_lock_irqsave(&irq->refcount, &dist->lpi_xa.xa_lock, flags)`.
- **Raw-spinlock callers (`vgic_put_irq_norelease()`):** Callers holding `raw_spinlock_t` (e.g., `vgic_cpu->ap_list_lock`) must use `__must_check` return of `vgic_put_irq_norelease()` and call `vgic_release_deleted_lpis()` after dropping the lock. Orphans are identified by `refcount == 0`.
- **Concurrent cache draining:** Only call `vgic_put_irq()` on the non-NULL pointer returned by `xa_erase(&its->translation_cache, idx)`.
- **AP-list lock-drop revalidation (`vgic_prune_ap_list()`):** Hold `vgic_get_irq()` reference across lock drops; re-verify target vCPU and `irq->vcpu` ownership. Two-vCPU `ap_list_lock` acquisition is ordered by ascending `vcpu_id`.
- **EOIcount deactivation walk:** Must start after the marker for the last interrupt loaded into a List Register, not from the head of `ap_list_head`. In nested vGIC mode, physical deactivation is performed by the LR `HW` bit; do not perform duplicate physical deactivation.

## List Registers, Mapped Interrupts, and ITS Table Save/Restore

- **LR uniqueness (`ICH_LR<n>_EL2`):** Two non-Invalid LRs must never hold the same `vINTID` or `pINTID`. An LR with `HW == 1` in active/pending state requires the physical interrupt to be active at the distributor.
- **Mapped interrupts & `ICH_VMCR_EL2` sync:** Deactivating a mapped level interrupt requires resampling the physical pending state. Sync `ICH_VMCR_EL2` via `vgic_v3_get_vmcr()` and `vgic_v3_put()` before evaluating pending interrupts for WFI.
- **ITS guest-memory table save/restore:** Restore handlers must enforce validation (e.g., `EventID` width caps). Unmapping a device/event must clear the valid bit in guest memory. Two-level table scans must check `byte_offset >= len` before subtraction.
- **vGIC readiness & `config_lock`:** Set `dist->ready` with `smp_store_release()` after MMIO registration; never hold `kvm->arch.config_lock` across `kvm_io_bus_unregister_dev()`.
- **LPI free-range list sorting:** `its_free_lpi_list` must be kept sorted ascending and merged with both neighbors.

## Priority Scales, Pseudo-NMI, and GICv3-on-GICv5 Compatibility

- **Acknowledge barrier:** `isb()` after reading `ICC_IAR1_EL1` must be unconditional, regardless of `supports_deactivate_key` (`ICC_EOIR1_EL1`).
- **IPI barriers:** `ICC_SGI1R_EL1` writes require `dsb(ishst)` before and `isb()` after; extract 4-bit SGI `INTID` with `0xf` mask at bits `[27:24]`.
- **Distributor vs CPU-interface priority scale (`gic_prio_init()`):** When `GICD_CTLR.DS == 0` and Group 0 is routed to EL3, `IPRIORITYR` writes use the non-secure shifted view (`dist_prio_irq` / `dist_prio_nmi`), whereas `ICC_PMR_EL1` and `ICC_RPR_EL1` comparisons use unshifted `GICV3_PRIO_*` constants.
- **GICv3 guest on GICv5 host (`FEAT_GCIE_LEGACY`):** Host provides a GICv3-compatible virtual CPU interface; host hardware is GICv5, and guest MMIO is emulated by KVM `vgic-v3`.
