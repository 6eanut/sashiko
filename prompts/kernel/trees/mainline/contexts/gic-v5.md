# ARM GICv5 Subsystem Context (`drivers/irqchip/irq-gic-v5*`, `arch/arm64/kvm/vgic/vgic-v5.c`)

## Core Architectural Differences from GICv3/v4 (Do NOT Apply GICv3 Rules)

- **No Distributor or Redistributor (`GICD_*`/`GICR_*`):** GICv5 comprises Interrupt Routing Services (IRS), Interrupt Translation Services (ITS), and Interrupt Wire Bridges (IWB). Each SPI is owned by one IRS (`gicv5_irs_lookup_by_spi_id()`, which can return `NULL` for unclaimed SPIs).
- **PPIs live in CPU interface system registers (`ICC_PPI_*_EL1`, `ICV_PPI_*_EL1`, `ICH_PPI_*_EL2`):** World switch saves/restores PPI state via sysregs (`__vgic_v5_save_ppi_state()` / `__vgic_v5_restore_ppi_state()`), not MMIO.
- **No List Registers on native GICv5:** Native virtualization uses memory-backed VM/VPE tables walked by the IRS, Direct Virtual Injection (DVI) for PPIs, and doorbells (`ICH_LR<n>_EL2` exists only under `FEAT_GCIE_LEGACY`).
- **No ITS command queue:** ITS configuration is a table memory write (`__le64`), cache clean/barrier, register-triggered invalidation (`ITS_INV_EVENTR` / `ITS_INV_DEVICER`), and `ITS_STATUSR.IDLE` poll. Unmapping is `VALID = 0` + invalidate (no `DISCARD`).
- **No IRS cache-invalidate register:** The IRS is the sole writer of Interrupt State Table (IST) contents; it uses `gicv5_irs_syncr()` (`IRS_SYNCR` -> `IRS_SYNC_STATUSR.IDLE`) to drain accepted events, never an invalidate.
- **5-bit priorities, no binary point (`ICC_BPR*`), 32-bit `ICC_APR_EL1` / `ICH_APR_EL2`:** Usable priority width is `min_not_zero(cpuif_pri_bits, irs_pri_bits)`.

## Typed INTIDs & Namespace Bounds (`include/linux/irqchip/arm-gic-v5.h`, `include/kvm/arm_vgic.h`)

- A GICv5 INTID encodes `GICV5_HWIRQ_TYPE` in bits `[31:29]` (`_PPI = 0x1`, `_LPI = 0x2`, `_SPI = 0x3`) and `GICV5_HWIRQ_ID` in bits `[23:0]` (three independent 24-bit namespaces).
- **Never index arrays or `*_SELR` registers with a full typed INTID:** Extract the bare ID via `FIELD_GET(GICV5_HWIRQ_ID, ...)` / `vgic_v5_get_hwirq_id()`.
- **`__irq_is_ppi()` on GICv5 checks only the 3-bit type field (`is_v5_type(GICV5_HWIRQ_TYPE_PPI, i)`), NOT the ID range:** Callers indexing `VGIC_V5_NR_PRIVATE_IRQS` (`64`) arrays must independently bounds-check the hardware ID (`< VGIC_V5_NR_PRIVATE_IRQS`). KVM deliberately supports only the 64 architected PPIs and zeroes `ICH_PPI_*1_EL2` / `ICH_PPI_PRIORITYR8..15_EL2` on restore.
- **No SGIs on GICv5:** `__irq_is_sgi()` is unconditionally `false`; IPIs are allocated from the LPI domain (`gicv5_irq_ipi_domain_alloc()`).

## System Instructions, `gsb_sys()` / `gsb_ack()`, and `isb()`

- **GSB vs `DSB` vs `isb()`:**
  - `gsb_sys()` completes Interrupt Effects of prior `GIC` instructions; `gsb_ack()` completes prior `GICR` acknowledge instructions (`arch/arm64/include/asm/barrier.h`). A `DSB` alone does **not** complete GIC instruction effects.
  - Neither `gsb_sys()` nor `gsb_ack()` is a Context Synchronization Event: reading a sysreg written by a GIC instruction (such as `ICC_ICSR_EL1` after `GIC CDRCFG`, checking `ICC_ICSR_EL1.F` first) or relying on a direct sysreg write requires an `isb()`.
- **Self-synchronizing registers on GICv5:** Only `ICC_PCR_EL3`, `ICC_PCR_EL1`, and `ICV_PCR_EL1` are self-synchronizing (`ICH_VMCR_EL2` and `ICH_APR_EL2` are **not**, even though `ICH_VMCR_EL2.VPMR` aliases `ICV_PCR_EL1.Priority`).
- **IRI disable vs enable (`gicv5_iri_irq_mask()` vs `unmask()`):** `GIC CDDIS` in `irq_mask()` requires `gsb_sys()` before returning (to satisfy Linux's lazy-disable contract), whereas `GIC CDEN` in `irq_unmask()` needs no barrier because GIC instructions complete in finite time.
- **PPI sysreg disable AND enable both require `isb()`:** Direct writes to `ICC_PPI_*` registers do **not** have the GIC-instruction finite-time completion guarantee, so both `gicv5_ppi_irq_mask()` and `gicv5_ppi_irq_unmask()` require `isb()`.

## Split Acknowledge, Priority Drop, and Deactivation (`gicv5_handle_irq()`)

- Native GICv5 is unconditionally split-EOI:
  1. `GICR CDIA` acknowledges -> `gsb_ack()` -> `isb()`.
  2. `gic_insn(0, CDEOI)` drops the highest active priority **immediately before dispatching the handler** (takes `XZR`/`0`, no INTID operand). Performing `CDEOI` inside `irq_eoi()` blocks all equal-priority interrupts for the entire handler duration.
  3. `gicv5_hwirq_eoi()` issues `GIC CDDI` with the INTID to deactivate (skipped in `gicv5_ppi_irq_eoi()` when `irqd_is_forwarded_to_vcpu(d)`).
- GICv5 LPIs have an Active state, so `IRQD_RESEND_WHEN_IN_PROGRESS` is not used.

## IRS Select-Program-Poll & Table Sizing (`drivers/irqchip/irq-gic-v5-irs.c`)

- **Serialize entire select-then-program sequences:** Hold `irs_data->spi_config_lock` (`raw_spinlock_t`) across `IRS_SPI_SELR` + `IRS_SPI_STATUSR` wait + `IRS_SPI_CFGR` + second `IRS_SPI_STATUSR` wait.
- **`IDLE` vs `V` in `*_STATUSR`:** `IDLE` indicates write completion; `V` indicates the selected object is valid. `gicv5_irs_wait_for_irs_pe()` checks `V` only after a selector write (`selr == true`), not after `IRS_PE_CR0` writes (do not flag the `CR0` path for skipping `V`). Always check and propagate `-ETIMEDOUT` from `gicv5_wait_for_op*()` and unwind allocations on failure.
- **IST sizing (`n` is an alignment exponent):** `IRS_IST_BASER.ADDR` requires bits `[n:0]` to be zero (`2^(n+1)`-byte alignment, with `Max(5, ...)` as a 64-byte alignment floor). Power-of-two `kmalloc()`/`kzalloc()` allocations are naturally aligned to their size; use `size_t` (never `u32` for `BIT(n + 1)`) and cap against `KMALLOC_MAX_SIZE`. When validating and walking a table, bound both the validation check and the loop with the **same** variable.

## Valid-Bit Ownership (IRS Hardware-Owned vs ITS Software-Owned)

- **IRS live structures (hardware sets `VALID` — Do NOT flag missing `VALID` bit in software):** When adding an L2 IST (`IRS_MAP_L2_ISTR`), L2 VM table (`IRS_VMAP_L2_VMTR`), or virtual IST (`IRS_VMAP_VISTR`) to an already-valid structure, software must write the entry with `VALID == 0` and write the map register; the IRS hardware sets `VALID = 1` and reports `IDLE`. Software setting `VALID = 1` on a live IRS structure is UNPREDICTABLE. (Software only writes `L2_VMTE.*_IST_VALID` when pre-populating an entry *before* the containing VM becomes valid, and migration must scrub `Pending`/`HWU`/metadata from the IST image before making it valid and replay pending interrupts via `GIC VDPEND` after VPEs are valid.)
- **ITS tables (software sets `VALID`):** The ITS has no map register; software writes `__le64` entries with `GICV5_DTL*E_VALID` / `GICV5_ITTL*E_VALID` set (`cpu_to_le64()` / `le64_to_cpu()`), cleans dcache (`gicv5_its_dcache_clean()`), invalidates (`gicv5_its_itt_cache_inv()` / `gicv5_its_device_cache_inv()`), and polls `ITS_STATUSR.IDLE` (`gicv5_its_cache_sync()`). Freeing/reusing an LPI or ITT also requires draining accepted events via `gicv5_its_syncr()` followed by `gicv5_irs_syncr()`.
- **Non-coherent IRS/ITS:** Branch on the coherency flag between `dcache_clean_inval_poc()` and `dsb(ishst)`. When reading a hardware-written L1 entry on a non-coherent IRS (`gicv5_irs_iste_alloc()`), call `dcache_inval_poc()` *after* `gicv5_irs_ist_synchronise()`. Program `IRS_CR1` / `ITS_CR1` before enabling the component.

## ID Register Widths, KVM PPI Reconciliation, and `FEAT_GCIE_LEGACY`

- **ID register field width != value range:** Check the architectural maximum (e.g., `IRS_IDR3.VM_ID_BITS` max is `16`, `VMD_SZ`/`VPED_SZ` max is `12`) before claiming integer truncation, and always provide a `default:` fallback when decoding ID registers.
- **KVM GICv5 PPI state (`arch/arm64/kvm/vgic/vgic-v5.c`):**
  - Iterate `for_each_visible_v5_ppi()` (`vgic_ppi_mask`), not all 64 PPIs.
  - Edge vs level asymmetry: `vgic_v5_flush_ppi_state()` clears `pending_latch` for edge PPIs after transferring to hardware; `vgic_v5_fold_ppi_state()` ORs hardware pending back into `pending_latch` for edge PPIs only. Exclude DVI'd PPIs (`bitmap_andnot(pendr, ..., dvir)`) when restoring pending state, and use atomic bitops (`assign_bit()`) on the shared DVI bitmap.
  - `vgic_v5_has_pending_ppi()` uses strict inequality `irq->priority < priority_mask`.
  - Userspace register write overrides (`uaccess_write`) must either enforce guest-side invariants or validate post-conditions at the end of restore, and must never silently discard writes (`case X: break;`).
- **Native VPE doorbells & `FEAT_GCIE_LEGACY`:**
  - VPE descriptor memory must be zero-initialized before making a VPE valid, and `IRS_VPE_DBR` doorbell settings must be reprogrammed after every VPE validation/reset.
  - Under `FEAT_GCIE_LEGACY` (`cpus_have_cap(ARM64_HAS_GICV5_LEGACY)`), `__vgic_v5_restore_vmcr_apr()` must clear `ICH_VCTLR_EL2.V3` and execute `isb()` before writing native `ICH_VMCR_EL2`.
