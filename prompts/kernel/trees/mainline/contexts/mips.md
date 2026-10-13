# MIPS Architecture Code Context (`arch/mips/`)

## 1. TLB Duplicate Entry Shutdown Hazards (`arch/mips/mm/tlb-r4k.c`)

* **Fatal TLB Shutdown (`ST0_TS` in `CP0_Status`)**: On R4x00, microAptiv/M5150, Cavium OCTEON3, and SB1 cores, executing `TLBP` (`tlb_probe()`), `TLBWI` (`tlb_write_indexed()`), or `TLBWR` (`tlb_write_random()`) when duplicate TLB entries exist (or when a write creates a duplicate) triggers an unrecoverable machine check (`do_mcheck()` in `arch/mips/kernel/traps.c`).
* **Early Boot Uniquification (`r4k_tlb_uniquify`)**:
  * Firmware/PROMs (such as the SGI IP22 PROM) may leave duplicate TLB entries at kernel entry.
  * Never call `tlb_probe()` (`TLBP`) during early TLB initialization before the TLB has been uniquified.
  * Read all entries by index first (`write_c0_index(i)` -> `mtc0_tlbr_hazard()` -> `tlb_read()` -> `tlb_read_hazard()` -> `read_c0_entryhi()`), detect collisions in software (`r4k_tlb_uniquify_read()`), and overwrite entries using `tlb_write_indexed()` (`r4k_tlb_uniquify_write()`) with guaranteed-unique VPNs and required CP0 hazard barriers (`mtc0_tlbw_hazard()`, `tlbw_use_hazard()`).

## 2. Synchronization, Memory, and Concurrency

* **Atomic Context**: Memory allocations in atomic contexts (interrupt handlers, spinlock-protected regions) must use `GFP_ATOMIC`.
* **Memory Barriers**: MIPS weak memory models require explicit barriers. Use `smp_mb__before_atomic()` or equivalent to prevent CPU reordering in lockless data structures.
* **RCU Usage**: `rcu_dereference()` must be enclosed within `rcu_read_lock()` / `rcu_read_unlock()` blocks.
* **CPU Teardown**: During `stop_this_cpu()`, the dying CPU must notify the RCU subsystem to prevent grace period hangs.
* **Lock Hierarchy**: When multiple locks are required, establish a strict acquisition order (e.g., `device_lock` -> `mutex_lock`) to prevent circular wait deadlocks.
* **Preemption**: Use `preempt_disable()`/`preempt_enable()` when accessing per-CPU variables to prevent thread migration during read-modify-write sequences.
* **Interrupt Masking**: When accessing shared hardware registers from both process and interrupt context, use `spin_lock_irqsave()` to prevent reentrancy deadlocks.

## 3. Initialization and Resource Management

* **Premature Publication**: Do not register devices or expose objects to the driver core until all internal state (clocks, IRQs, memory mappings) is fully initialized.
* **Symmetric Unwind**: Every resource acquisition (`request_irq`, `ioremap`, `kzalloc`) must have a corresponding release. Use `devm_*` managed resources where possible; do not mix with manual `kfree()`.
* **Firmware Constraints**: Ensure buffers passed to firmware are within the addressable range (32-bit vs 64-bit segments).
* **Device Tree Fixups**: Use `IS_ENABLED()` to guard platform-specific fixups to ensure they execute correctly regardless of whether the driver is built-in or modular.
* **Platform Device Model**: Always use `platform_driver_register()` and `platform_device_register()` to ensure proper probe/remove lifecycle management.
* **Error Propagation**: Always check return values of `ioremap()` and `request_irq()` immediately; propagate negative error codes (e.g., `-ENOMEM`, `-EBUSY`) to the caller.

## 4. Architecture-Specific Bug Patterns

* **Sign/Zero Extension (MIPS64)**: Explicitly perform zero extension for sub-register width operations (32-bit values in 64-bit registers) to prevent sign-extension corruption.
* **Array Bounds**: Use `ARRAY_SIZE()` for all bounds checks. Always validate indices before writing to arrays (e.g., `maar_res_walk()`).
* **Instruction Encoding**: Verify opcodes match operand types (immediate vs. register) in JIT compilers and assembly stubs.
* **Syscall Interception**: Syscall skipping logic must distinguish between valid syscalls and those intended to be skipped; ensure return values are preserved.
* **Endianness**: Stack argument fetching (e.g., `o32` wrappers) must account for platform endianness to prevent incorrect 32-bit loads from 64-bit slots.
* **Linker Scripts**: Ensure `.got` and other non-dynamic sections are excluded from the dynamic segment to prevent invalid ELF headers.

## 5. Hardware Invariants and Power Management

* **TLB Hazard Barriers**: After any `mtc0` instruction modifying CP0 registers (e.g., `EntryHi`, `Index`), insert the appropriate hazard barrier (`mtc0_tlbw_hazard()`, `tlbw_use_hazard()`, or `tlb_read_hazard()`) before subsequent TLB operations.
* **Resource Alignment**: Hardware resource ranges must be correctly aligned and registered to prevent conflicts with legacy hardware.
* **MMIO Annotations**: All pointers to memory-mapped I/O regions must be annotated with `__iomem` to satisfy sparse and ensure type safety.
* **Power State**: Ensure all hardware clocks and power domains are enabled before register access. Use runtime PM to balance reference counts across all success/error paths.
* **Hardware Side Effects**: Be aware that modifying certain TLB or CP0 registers may have hidden side effects (e.g., clearing status bits) that must be preserved via read-modify-write cycles.
* **DMA Consistency**: Ensure `dma_alloc_coherent()` is used for shared buffers to guarantee visibility across the MIPS cache hierarchy.
* **Cache Coherency**: Use `dma_sync_single_for_device()` and `dma_sync_single_for_cpu()` to manage cache lines for streaming DMA mappings.
* **Hardware Reset**: When resetting hardware, ensure all in-flight DMA transactions are quiesced and pending interrupts are masked to prevent spurious triggers during re-initialization.
* **Register Access**: Always perform read-modify-write cycles when updating shared hardware registers to avoid clobbering unrelated bitfields.
* **Memory Safety**: Validate all lengths and offsets derived from hardware descriptors against the actual allocated buffer size to prevent out-of-bounds access.
* **Interrupt Storms**: Implement rate-limiting or interrupt masking in the ISR if the hardware is prone to spurious events or high-frequency triggers.
* **Reference Counting**: Ensure that every `get_device()` call is paired with a `put_device()` on all exit paths, including error handling blocks.
* **Compiler Workarounds**: Only apply compiler-specific workarounds if the kernel version and compiler version match the affected range; verify the bug is not already patched in the toolchain.
* **Static vs Dynamic**: Do not call `kfree()` on statically allocated objects; ensure memory management strategy matches the object's lifecycle.
* **Subsystem Contracts**: When implementing callbacks for external subsystems, ensure the execution context (atomic vs. process) matches the contract requirements.
