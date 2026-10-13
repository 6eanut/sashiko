# MMIO Register and Stream Accessor APIs (`include/asm-generic/io.h`)

This guide defines the authoritative semantics for Memory-Mapped I/O (MMIO) and Port I/O accessors in Linux v7.3-rc5. These APIs provide the fundamental interface between the kernel and hardware registers/FIFOs, ensuring architectural portability, correct endianness conversion, and memory ordering.

## 1. Core API Overview & Ownership Lifecycles

### 1.1 The `__iomem` Cookie
All MMIO accessors operate on pointers qualified with `__iomem`. This is a `sparse`-checked attribute indicating that the pointer resides in an I/O address space and **must not** be dereferenced directly.
*   **Acquisition**: `__iomem` pointers are obtained via `ioremap()`, `devm_ioremap()`, or `pci_iomap()`.
*   **Ownership**: The driver that performs the mapping owns the lifecycle. `devm_` managed resources tie the mapping lifecycle to the `struct device`.
*   **Validation**: Accessors do not validate the `__iomem` pointer at runtime. Passing a `NULL` or unmapped address triggers an architecture-specific exception (kernel paging fault).

### 1.2 Reference Counting and Teardown
*   **No Implicit Refcounting**: MMIO accessors do not increment reference counts on the underlying device or memory region.
*   **Teardown Symmetry**: Manual mappings (`ioremap`) require explicit `iounmap()`. Managed mappings (`devm_ioremap`) are automatically released on driver unbind.
*   **Asynchronous Safety**: If a background worker or IRQ handler uses an `__iomem` pointer, the driver must ensure `iounmap()` is not called until all asynchronous consumers are synchronously joined or cancelled.

## 2. Calling Context, Locking & Synchronization Rules

### 2.1 Execution Context
*   **Atomic Safety**: All standard accessors (`readl`, `writel`, `readsl`, etc.) are **non-sleeping** and safe to call from any context, including HARDIRQ, SOFTIRQ, and RCU critical sections.
*   **Mapping Context**: While accessors are atomic, mapping functions (e.g., `ioremap`) may sleep and must be called from process context.

### 2.2 Memory Ordering and Barriers
Standard accessors (`readX`/`writeX`) provide implicit memory barriers:
*   **`__io_bw()` / `__io_aw()`**: Barriers before and after writes. `__io_bw` ensures preceding memory writes (e.g., DMA buffers) are visible to the device before the MMIO write. `__io_aw` uses `mmiowb_set_pending()` to serialize device access against `spin_unlock`.
*   **`__io_br()` / `__io_ar()`**: Barriers before and after reads. `__io_ar` prevents the CPU from speculatively reading coherent DMA data before the MMIO read has finished.
*   **`__raw_` variants**: These **do not** contain these barriers. They must only be used in performance-critical loops where barriers are manually managed.

## 3. Endianness Semantics: Register vs. Stream/FIFO

### 3.1 Register I/O (`readl`, `writel`, `readw`, `writew`, `readb`, `writeb`)
*   Perform little-endian conversion (`__cpu_to_leN` / `__leN_to_cpu`) around `__raw_readN`/`__raw_writeN` plus I/O memory barriers.
*   **Invariant**: Intended for hardware control and status registers with fixed little-endian bit layouts.

### 3.2 FIFO / Stream I/O (`readsl`, `writesl`, `readsw`, `writesw`, `readsb`, `writesb`)
*   Map to `__raw_readN`/`__raw_writeN` without byte-swapping, preserving raw byte order between memory buffers and hardware FIFOs on both little-endian and big-endian CPUs.

### 3.3 The "No Mixed FIFO Accessors" Rule
Never use `writesl()` / `readsl()` for aligned 4-byte bulk FIFO transfers and `writel()` / `readl()` for the 1–3 trailing remainder bytes on the same FIFO register. On big-endian architectures, `writel()` / `readl()` byte-swaps the tail word while `writesl()` / `readsl()` does not.
*   **Required Pattern**: Use `writesl(fifo, &tmp, 1)` / `readsl(fifo, &tmp, 1)` for tail handling.
*   **Reference**: See `i3c_writel_fifo()` and `i3c_readl_fifo()` in `drivers/i3c/internals.h` for the correct implementation of this pattern.

## 4. High-Frequency Bug Patterns & Review Invariants

### 4.1 Alignment and Granularity
*   **Bug Pattern**: Using `readl` (32-bit) to access a 1-byte register. This triggers alignment faults or corrupts adjacent registers.
*   **Review Invariant**: The accessor width (`b`, `w`, `l`, `q`) must exactly match the hardware register width defined in the device specification.

### 4.2 PCI Configuration Space
*   **API Selection**: Use `pci_remap_cfgspace()` for mapping PCI configuration registers. This ensures "non-posted" transaction semantics. Standard `ioremap()` may allow write-posting, leading to race conditions.

### 4.3 Tracing and Debugging
*   **Overhead Guard**: Tracing is guarded by `rwmmio_tracepoint_enabled()`.
*   **Opt-out**: Drivers can disable tracing for high-frequency registers by defining `__DISABLE_TRACE_MMIO__` before including `<linux/io.h>`.
*   **Static Keys**: Ensure that `log_read_mmio` and `log_write_mmio` are only called when tracing is enabled. Do not bypass this with custom logging.

### 4.4 Resource Management (devm)
*   **Mixed Lifecycle Risk**: Avoid mixing `devm_ioremap()` with manual `release_mem_region()`. If a probe fails, `devres` will attempt to unmap the region; manual freeing leads to double-free or use-after-free.
