# MMIO Register and Stream Accessor APIs (`include/asm-generic/io.h`)

## Endianness Semantics: Register I/O vs Stream/FIFO I/O

* **Register I/O (`readl`, `writel`, `readw`, `writew`, `readb`, `writeb`)**:
  * Perform little-endian conversion (`__cpu_to_leN` / `__leN_to_cpu`) around `__raw_readN`/`__raw_writeN` plus I/O memory barriers (`__io_br`/`__io_ar`, `__io_bw`/`__io_aw`).
  * Intended for hardware control and status registers with fixed little-endian bit layouts.
* **FIFO / Stream I/O (`readsl`, `writesl`, `readsw`, `writesw`, `readsb`, `writesb`)**:
  * Map to `__raw_readN`/`__raw_writeN` without byte-swapping, preserving raw byte order between memory buffers and hardware FIFOs on both little-endian and big-endian CPUs.
* **No Mixed FIFO Accessors**: Never use `writesl()` / `readsl()` for aligned 4-byte bulk FIFO transfers and `writel()` / `readl()` for the 1–3 trailing remainder bytes on the same FIFO register — on big-endian architectures `writel()` / `readl()` byte-swaps the tail word while `writesl()` / `readsl()` does not. Use `writesl(fifo, &tmp, 1)` / `readsl(fifo, &tmp, 1)` (as in `i3c_writel_fifo()` / `i3c_readl_fifo()` in `drivers/i3c/internals.h`).
