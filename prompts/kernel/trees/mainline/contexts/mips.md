# MIPS Architecture Code Context (`arch/mips/`)

## TLB Duplicate Entry Shutdown Hazards (`arch/mips/mm/tlb-r4k.c`)

* **Fatal TLB Shutdown (`ST0_TS` in `CP0_Status`)**: On R4x00, microAptiv/M5150, Cavium OCTEON3, and SB1 cores, executing `TLBP` (`tlb_probe()`), `TLBWI` (`tlb_write_indexed()`), or `TLBWR` (`tlb_write_random()`) when duplicate TLB entries exist (or when a write creates a duplicate) triggers an unrecoverable machine check (`do_mcheck()` in `arch/mips/kernel/traps.c`).
* **Early Boot Uniquification (`r4k_tlb_uniquify`)**:
  * Firmware/PROMs (such as the SGI IP22 PROM) may leave duplicate TLB entries at kernel entry.
  * Never call `tlb_probe()` (`TLBP`) during early TLB initialization before the TLB has been uniquified.
  * Read all entries by index first (`write_c0_index(i)` -> `mtc0_tlbr_hazard()` -> `tlb_read()` -> `tlb_read_hazard()` -> `read_c0_entryhi()`), detect collisions in software (`r4k_tlb_uniquify_read()`), and overwrite entries using `tlb_write_indexed()` (`r4k_tlb_uniquify_write()`) with guaranteed-unique VPNs and required CP0 hazard barriers (`mtc0_tlbw_hazard()`, `tlbw_use_hazard()`).
