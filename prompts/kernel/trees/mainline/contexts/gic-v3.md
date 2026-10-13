# ARM GICv3 / GICv4 Subsystem Context (`drivers/irqchip/irq-gic-v3*`, `arch/arm64/kvm/vgic/`)

## ITS Command Queue (`drivers/irqchip/irq-gic-v3-its.c`)

The canonical command-queue sequence in `BUILD_SINGLE_CMD_FUNC` (`its_send_single_command()` / `its_send_single_vcommand()`) is: `its_allocate_entry()` -> builder -> `its_flush_cmd()` -> optional SYNC/VSYNC entry -> sample `GITS_CREADR` -> `its_post_commands()` (`GITS_CWRITER`) -> unlock -> `its_wait_for_range_completion()`.

- **Queue publish barrier (`its_flush_cmd()`):** Branches on `ITS_FLAGS_CMDQ_NEEDS_FLUSHING` between `gic_flush_dcache_to_poc()` and `dsb(ishst)` before writing the `GITS_CWRITER` doorbell (Normal-memory queue store vs Device-memory doorbell). Never hardcode only one arm or omit `its_flush_cmd()`. (This ordering comes from the base ARM architecture, not a GIC spec clause; do not demand a GIC spec citation for it.)
- **Two-level ITS table publication:** A write setting `Valid = 1` on a Level-1 ITS table entry must be globally visible before queuing any ITS command that references that Level-2 table.
- **Sampling `GITS_CREADR` before ringing:** Sample `rd_idx = readl_relaxed(its->base + GITS_CREADR)` *before* `its_post_commands(its)` while holding `its->lock`, then wait outside the lock. Ring-offset comparisons must linearize wrap-around (`ITS_CMD_QUEUE_SZ`).
- **Endianness fixup (`its_fixup_cmd()`):** Every command builder return path (including early exits via `goto out;`, as in `its_build_vmapp_cmd()`) must pass through `its_fixup_cmd()` (`cpu_to_le64()`).
- **Queue full & stall handling:** `its_queue_full()` stops one entry short of wrapping onto `ridx` (`((widx + 1) % ITS_CMD_QUEUE_NR_ENTRIES) == ridx`). On a command error, the architecture permits CONSTRAINED UNPREDICTABLE choice of ignoring, stalling (`GITS_CREADR.Stalled == 1`, restarted via `GITS_CWRITER.Retry`), or treating as valid; never assume hardware simply ignores a bad command, and any ITS emulation that can set `Stalled` must implement `GITS_CWRITER.Retry`.

## Completion: `SYNC` (Physical) vs `VSYNC` (Virtual)

- `its_send_single_command()` appends `SYNC` (physical interrupts on a target Redistributor `RDbase`); `its_send_single_vcommand()` appends `VSYNC` (virtual operations targeting a `vPEID`). Routing a virtual command through the physical send path uses the wrong synchronizer.
- **Builder return value controls sync emission:** Returning `NULL` from a command builder elides the trailing `SYNC`/`VSYNC`.
- **GICv4.1 `VMAPP` unmap is self-synchronizing (Do NOT flag):** On GICv4.1 (`is_v4_1(its)`), `VMAPP` with `{V, Alloc} == {0, x}` is architecturally self-synchronizing, and issuing a trailing `VSYNC` for the unmapped vPE is a command error (`VSYNC_VCPU_INVALID` / SError). `its_build_vmapp_cmd()` deliberately returns `NULL` on the unmap path **only** when `is_v4_1(its)` is true; GICv4.0 still requires the `VSYNC`.
- **`INV` and `INVALL` require `SYNC`; `INVDB` and `VSGI` require `VSYNC`** (automatically appended by `its_send_single_vcommand()`).

## LPI Configuration Table Sharing (`GICR_TYPER.CommonLPIAff`)

- **Shared `GICR_PROPBASER`, per-RD `GICR_PENDBASER`:** Redistributors in the same `GICR_TYPER.CommonLPIAff` group (`0b00` = all RDs; `0b01` = same `Aff3`; `0b10` = same `Aff3.Aff2`; `0b11` = same `Aff3.Aff2.Aff1`) must share the same LPI Configuration table (`GICR_PROPBASER`) and, on GICv4.1, the same vPE Configuration table. The LPI Pending table (`GICR_PENDBASER`) is strictly per-Redistributor and must **never** be shared.
- **`CommonLPIAff` mask keeps high bits (`compute_common_aff()`):** Because `Aff3` is in bits `[31:24]`, grouping by leading affinity fields masks off low bits: `aff & ~(GENMASK(31, 0) >> (clpiaff * 8))`. Using `aff & (GENMASK(31, 0) >> ...)` is inverted and groups the wrong redistributors.
- **Publishing LPI config updates:** Requires global visibility followed by `GICR_CTLR.EnableLPIs` (`0 -> 1`), `GICR_INVLPIR`/`GICR_INVALLR`, or ITS `INV`/`INVALL` + `SYNC`.
- **Policy cap on LPI ID width:** Host `lpi_id_bits` is capped to `ITS_MAX_LPI_NRBITS` (`16`) unless tables were pre-allocated by firmware (where `GICR_PROPBASER.IDBITS` is preserved).

## Redistributor & Distributor Write Completion (`RWP` vs Read-Back)

- **`GICD_CTLR_RWP` (bit 31) vs `GICR_CTLR_RWP` (bit 3):** Both registers sit at offset `0x0000` (`GICR_CTLR` aliases `GICD_CTLR`), but their `RWP` bits are at different bit positions. Always pair distributor base with `GICD_CTLR_RWP` (`gic_dist_wait_for_rwp()`) and redistributor base with `GICR_CTLR_RWP` (`gic_redist_wait_for_rwp()`).
- **What `RWP` tracks:**
  - `GICD_CTLR.RWP`: group-enable `1 -> 0` transitions, `ARE`/`DS` transitions, `GICD_ICENABLER<n>`.
  - `GICR_CTLR.RWP`: `GICR_ICENABLER0`, `GICR_CTLR` `DPG*`, `EnableLPIs` `1 -> 0`, and on GICv4.1 `GICR_VPROPBASER.Valid` `1 -> 0`.
  - Enabling interrupts (`ISENABLER*`), group `0 -> 1` transitions, and priority/routing writes are **not** tracked by `RWP`; do not flag missing `RWP` polls after them.
- **Set-active / set-pending completion requires MMIO read-back:** Because active/pending registers are not tracked by `RWP` and Device-nGnRnE early write acknowledgement can buffer writes in the interconnect, callers that require immediate completion (e.g., `gic_irq_set_irqchip_state()` before guest entry) must read the register back. Do not flag that read-back as dead/redundant.

## GICv4 vPE Residency, Affinity, and Doorbells (`drivers/irqchip/irq-gic-v4.c`)

- **`GICR_VPENDBASER` rules:**
  - Never modify any field of `GICR_VPENDBASER` other than `Valid` while `Valid == 1` (UNPREDICTABLE).
  - Wait for `GICR_VPENDBASER.Dirty == 0` before clearing `Valid` on deschedule (`its_clear_vpend_valid()`). If `Dirty` remains set on timeout, force `PendingLast = true` conservatively so a vCPU never sleeps with an unacknowledged interrupt.
- **Lock hierarchy (`include/linux/irqchip/arm-gic-v4.h`):** `vmapp_lock -> vpe_lock -> vmovp_lock`. Reading or updating `vpe->col_idx` requires `vpe_lock` (not just `irq_desc->lock`, since vPE affinity affects all routed vLPIs). Under `ITSList`, `its_send_vmovp()` serializes sequence numbers across all ITS instances under `vmovp_lock`.
- **Unmapped vs lazily-mapped vPE affinity (`its_vpe_set_affinity()`):** Guard `VMOVP` emission on `vpe->its_vm->vlpi_count` / `vmapp_count`: under lazy mapping, an affinity change before the first `MAPTI` simply updates software affinity and returns success without touching hardware.
- **WFI doorbell race:** Because `vcpu_put()`/`vcpu_load()` on preemption changes residency, KVM records WFI intent in an explicit vCPU flag under `preempt_disable()` and keys doorbell requests on that flag rather than residency alone.

## vGIC LPI Refcounting, XArray, and AP-List Migration (`arch/arm64/kvm/vgic/`)

- **Lookup under RCU/lock:** Lookups in `dist->lpi_xa` must call `vgic_try_get_irq_ref(irq)` *inside* `rcu_read_lock()` before returning the pointer.
- **Atomic decrement + eviction:** `vgic_put_irq()` uses `refcount_dec_and_lock_irqsave(&irq->refcount, &dist->lpi_xa.xa_lock, flags)` so a concurrent `INTID` re-registration cannot be erased by a racing release.
- **Raw-spinlock callers (`vgic_put_irq_norelease()`):** Callers holding a `raw_spinlock_t` (such as `vgic_cpu->ap_list_lock`) cannot acquire `lpi_xa.xa_lock`; they must accumulate the `__must_check` return of `vgic_put_irq_norelease()` and call `vgic_release_deleted_lpis()` after dropping the raw lock. Orphans are identified strictly by `refcount == 0`, never by an in-object flag.
- **Concurrent cache draining (`its->translation_cache`):** When draining a shared XArray, only call `vgic_put_irq()` on the non-NULL pointer returned by `xa_erase(&its->translation_cache, idx)`, never on the loop iterator variable.
- **AP-list lock-drop revalidation (`vgic_prune_ap_list()`):** Hold a `vgic_get_irq()` reference across any lock drop; when locks are reacquired, verify **both** that the target vCPU matches and that `irq->vcpu` ownership has not been cleared. Two-vCPU `ap_list_lock` acquisition is ordered by ascending `vcpu_id`.
- **EOIcount deactivation walk:** Must start after the marker for the last interrupt loaded into a List Register, not from the head of `ap_list_head`. In nested vGIC mode, physical deactivation is already performed by the LR `HW` bit; do not perform a duplicate physical deactivation.

## List Registers, Mapped Interrupts, and ITS Table Save/Restore

- **LR uniqueness (`ICH_LR<n>_EL2`):** Two non-Invalid LRs (`State != 0b00`) must never hold the same `vINTID` or the same `pINTID`, and a valid LR must not overlap a `vINTID` delivered via GICv4 direct vLPI/vSGI injection. An LR with `HW == 1` in active/pending state requires the physical interrupt to be active at the distributor.
- **Mapped interrupts & `ICH_VMCR_EL2` sync:** Deactivating a mapped level interrupt requires resampling the physical pending state. Because `vgic_v3_get_vmcr()` reads the shadow `cpu_if->vgic_vmcr` (only updated on `vgic_v3_put()`), evaluating whether a guest has pending interrupts before blocking in WFI requires syncing `ICH_VMCR_EL2` first.
- **ITS guest-memory table save/restore (`vgic-its.c`):**
  - Restore handlers must enforce every validation that live `MAPD`/`MAPTI`/`MAPI` handlers enforce (such as `EventID` width caps).
  - Unmapping a device or event must clear the valid bit in guest memory so a subsequent save/restore cycle does not resurrect a deleted entry.
  - Two-level table scans advancing by `byte_offset = next_offset * esz` must check `if (byte_offset >= len) break;` *before* subtracting from unsigned `len`.
- **vGIC readiness & `config_lock`:** Set `dist->ready` with `smp_store_release()` only *after* MMIO regions are registered; never hold `kvm->arch.config_lock` across `kvm_io_bus_unregister_dev()` / MMIO bus registration calls.
- **LPI free-range list sorting:** Free LPI ID ranges (`its_free_lpi_list`) are kept sorted ascending and merged with **both** adjacent neighbors on release to prevent silent fragmentation.

## Priority Scales, Pseudo-NMI, and GICv3-on-GICv5 Compatibility

- **Acknowledge barrier:** `isb()` after reading `ICC_IAR1_EL1` must be unconditional, not gated inside the `supports_deactivate_key` (`ICC_EOIR1_EL1`) branch.
- **IPI barriers:** Sending an IPI requires `dsb(ishst)` (never `dmb`, since `ICC_SGI1R_EL1` is a sysreg write, not a memory access) before the write and `isb()` after; extract the 4-bit SGI `INTID` with an unsigned `0xf` mask at bits `[27:24]`.
- **Distributor vs CPU-interface priority scale (`gic_prio_init()`):** When `GICD_CTLR.DS == 0` and Group 0 is routed to EL3 (`SCR_EL3.FIQ == 1`), distributor/redistributor `IPRIORITYR` writes use the non-secure shifted view (`dist_prio_irq` / `dist_prio_nmi`), whereas `ICC_PMR_EL1` and `ICC_RPR_EL1` comparisons use unshifted `GICV3_PRIO_*` constants.
- **GICv3 guest on GICv5 host (`FEAT_GCIE_LEGACY`):** Provides only a GICv3-compatible *virtual* CPU interface; the host has no `GICD`/`GICR`/GICv3-ITS hardware (all guest MMIO is emulated by KVM `vgic-v3` on top of native GICv5).
