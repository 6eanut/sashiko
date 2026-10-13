# ARM64 Architecture & Subsystem Context

## Grounding Rule: Do Not Confabulate ARM ARM Encodings

Do not rely on unverified recall for exact ARM architectural bit positions, `ESR_ELx` exception classes or `ISS` sub-fields, `RES0`/`RES1` polarity, barrier rules, or trap routing. Only report an architectural encoding or ordering issue when it can be grounded directly in the code under review or the pre-verified rules below.

## `ESR_ELx` Syndrome: Exception Class, `IL` Bit, and `ISS` Sub-fields

`ESR_ELx` (`arch/arm64/include/asm/esr.h`) encodes:
- Exception Class `EC` in bits `[31:26]` (`ESR_ELx_EC_SHIFT`)
- Instruction Length `IL` in bit `[25]` (`ESR_ELx_IL`)
- Instruction Specific Syndrome `ISS` in bits `[24:0]` (`ESR_ELx_ISS_MASK`)

### The `IL` Bit (bit `[25]`)

For single-instruction traps, `IL` is `0` for 16-bit and `1` for 32-bit instructions. However, the architecture requires `IL == 1` unconditionally for:
- SError, Instruction Abort, PC alignment fault, SP alignment fault
- Data Abort with `ISV == 0`
- Illegal Execution State exception
- Any debug exception other than Breakpoint (`BRK` / `BKPT`)
- Any exception reported with `EC == 0b000000` (Unknown)

When KVM synthesizes `ESR_ELx` to inject one of these exceptions into a guest or nested guest, it must set `ESR_ELx_IL`.

### Per-`EC` `ISS` Sub-fields (`EC == 0x1A` vs `EC == 0x1C`)

- **ERET trap (`EC == 0x1A`, `ESR_ELx_EC_ERET`, trapped by `HCR_EL2.NV`):**
  - bit `[1]` `ESR_ELx_ERET_ISS_ERET` (`0x2`): `0` = `ERET`, `1` = `ERETAA`/`ERETAB` (`esr_iss_is_eretax()`).
  - bit `[0]` `ESR_ELx_ERET_ISS_ERETA` (`0x1`): `0` = A key (`ERETAA`), `1` = B key (`ERETAB`) (`esr_iss_is_eretab()`). `RES0` when bit `[1]` is `0`.
  - bits `[24:2]`: `RES0`.
- **PAC Fail (`EC == 0x1C`, `ESR_ELx_EC_FPAC`):**
  - bit `[1]` `DnI`: `0` = Instruction key, `1` = Data key.
  - bit `[0]` `BnA`: `0` = A key, `1` = B key.
  - bits `[24:2]`: `RES0`.

In `kvm_emulate_nested_eret()` (`arch/arm64/kvm/emulate-nested.c`), constructing an FPAC syndrome from a failed `ERETAx` trap syndrome via:
```c
esr &= (ESR_ELx_ERET_ISS_ERETA | ESR_ELx_IL);
esr |= FIELD_PREP(ESR_ELx_EC_MASK, ESR_ELx_EC_FPAC);
```
is **correct** and must not be flagged: bit `[0]` (`ERETA` -> `BnA`) and `IL` (`1`) carry over identically, and clearing bit `[1]` sets `DnI == 0` (Instruction key).

Also do not flag lower-EL trap helpers in `arch/arm64/include/asm/esr.h` (such as `esr_trap_is_iabt()` or `esr_abt_is_exec_fault()`) for checking `ESR_ELx_EC_IABT_LOW` without `ESR_ELx_EC_IABT_CUR` when callers only handle lower-EL traps.

## Memory Tagging Extension (MTE) and Tagged Addresses

- **Pointer Arithmetic:** Top Byte Ignore (TBI) ignores bits `[63:56]` during hardware MMU translation, but software address arithmetic (e.g., bit-shifts to compute page indices or page-table offsets) must explicitly untag virtual addresses via `untagged_addr()`.
- **Tag Initialization Barriers:** A `DSB` is required between explicit tag stores (`STG`/`STZG`) and the PTE update so tag stores complete before page-table walkers or userspace observe the mapping.
- **Shared Page Tag Initialization:** Shared or special pages (such as the huge zero folio) must have tag clearing completed and synchronized before the page is marked MTE-tagged or exposed to observers.

## System Register and Context Synchronization

Whether a Context Synchronization Event (CSE: `isb()`, or exception entry/return when `SCTLR_ELx.{EIS,EOS}` is enabled) is required after a system register write depends on how the register is subsequently consumed (Arm ARM DDI 0487 Table D24-1):

- **Direct read-back (`MRS` of the same ordinary RW register just written) needs NO CSE.** Do not flag a missing `isb()` between `write_sysreg_s(val, REG)` and `read_sysreg_s(REG)` (except for `*_SET`/`*_CLR` registers such as `PMOVSSET_EL0` whose write effect is indirect).
- **Indirect read (a subsequent instruction relying on the register's active effect) requires a CSE** before that instruction, unless the register/field is self-synchronizing:
  - If the consumer runs at a lower/other Exception level after an `ERET`/exception boundary (and `SCTLR_ELx.EOS`/`EIS != 0`), that boundary supplies the CSE; no extra `isb()` at the writing EL is needed (e.g., activating guest traps in `CPTR_EL2` or `HFGRTR_EL2` before `ERET`, or writing `FPEXC32_EL2`).
  - If the writing EL itself relies on the new effect before any exception boundary (e.g., deactivating `CPTR_EL2` traps so EL2 can execute FP/SVE, or updating `TCR_EL1`/`TTBR_ELx` before an in-context memory access), an explicit `isb()` is required.
  - **Exception for `HCR_EL2` fields consumed by `ERET`:** Certain `HCR_EL2` fields are read by `ERET` itself while constructing the target context, before `ERET`'s synchronization point, and therefore require an explicit `isb()` before `ERET`.
- **Self-Synchronizing Registers/Fields (never flag missing `isb()`):**
  - `ZCR_ELx.LEN` and `SMCR_ELx.LEN` (SVE/SME vector length fields).
  - `FPMR` (`SYS_FPMR`, FP8 mode register).
  - `ICC_PMR_EL1` on the interrupt-masking direction (`local_irq_disable()`). On the unmasking direction (`local_irq_enable()`), `pmr_sync()` (`dsb` when `ICC_CTLR_EL1.PMHE` is set under `CONFIG_ARM64_PSEUDO_NMI`) is required, never `isb()`.
- **GIC System and MMIO Registers:**
  - Writes to `ICC_*_EL1` registers other than `ICC_PMR_EL1` require an `isb()`.
  - Writes to tracked MMIO GIC registers require polling `GICD_CTLR.RWP` (group-enable 1->0, `ARE`/`DS` writes, `GICD_ICENABLER<n>`) or `GICR_CTLR.RWP` (`GICR_ICENABLER0`, `EnableLPIs` 1->0, `DPG` writes). Priority, routing, and enable-set writes are not tracked by `RWP`.

## Auto-Generated `.sysreg` Definitions and Aggregate Masks

In `arch/arm64/tools/gen-sysreg.awk`, `<REG>_RES1` and `<REG>_RES0` are generated purely from literal `Res1` and `Res0` lines in `.sysreg` files (with no feature conditionality; conditional RES1 lives in `arch/arm64/kvm/config.c` as `AS_RES1`). When a patch modifies `.sysreg` bit classifications (`Res1`/`Res0`/`Field`), check all C call sites consuming `<REG>_RES1` or `<REG>_RES0` (such as `INIT_SCTLR_EL2_MMU_ON` / `INIT_SCTLR_EL2_MMU_OFF`) to verify that changing the generated mask value does not silently alter hardware initialization.

## TLB Invalidation and Break-Before-Make (BBM)

- **TLBI Completion:** A `TLBI` instruction is only guaranteed complete after the **same** PE executes a `DSB` (`DSB ISH` for broadcast invalidations). A remote CPU's `DSB` does not complete another CPU's `TLBI`. Every observing PE must execute an `isb()` (or CSE) after the issuing PE's `DSB` completes when executable mappings or local context synchronization are affected (including mode-entry paths like `enter_vhe()`, `__tlb_switch_to_guest()`, and `__primary_switch()`).
- **Break-Before-Make (BBM):** Updating a live shared translation table entry to change block/table size (unless `FEAT_BBML2` is supported across all CPUs), create an overlapping global entry, change Output Address to non-identical memory contents, or change memory attributes requires:
  1. Write invalid descriptor -> 2. `DSB` -> 3. Broadcast `TLBI` -> 4. `DSB` -> 5. Write new descriptor -> 6. `DSB`.

## Instruction and Data Coherency (PoU vs PoC)

- **Self-Modifying Code (PoU):** Writing instructions requires `DC CVAU` -> `DSB ISH` -> `IC IVAU` -> `DSB ISH` -> `isb()` (or CSE) on all observing CPUs.
- **Concurrent Modification and Execution (CMODX):** Without an intervening `isb()`/CSE on all observing CPUs, concurrent patching is only architecturally safe for `B`, `B.cond`, `BL`, `BRK`, `CB<cc>`, `CBB<cc>`, `CBH<cc>`, `CBNZ`, `CBZ`, `HVC`, `ISB`, `NOP`, `SMC`, `SVC`, `TBNZ`, `TBZ`, `TRCIT`, and `UDF`.

## Lockless Page Table Walks, Stack Pivots, and SVE/SME State

- **Lockless Walks:** Must load shared PTE/PMD/PUD/PGD descriptors with `READ_ONCE()` and account for statically folded page-table levels (`PGTABLE_LEVELS <= 2`).
- **Stack Pointer (`SP`) Pivots:** Asynchronous exceptions (`DAIF`) must be masked during stack transitions/pivots (e.g., switching to/from Shadow Call Stack or IRQ/overflow stacks) so an exception cannot clobber `ELR_ELx`/`SPSR_ELx` mid-pivot.
- **SVE/SME/FPSIMD:** Vector-length changes must invalidate or rebuild any state/buffers sized for the old VL; entering streaming SVE via ptrace/signal requires an explicit streaming-mode payload; signal return must merge FPSIMD and SVE state coherently.
- **Fixed-Register System Instructions:** Inline assembly for instructions with fixed register operands (e.g., `GIC CDEOI` requiring `XZR`/register 31) must hardcode the register in the asm string rather than using a general `"r"` constraint.
