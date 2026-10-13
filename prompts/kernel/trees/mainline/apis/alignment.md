# Alignment Helpers API

## Generic Alignment (`include/linux/align.h`)

- `ALIGN(x, a)` rounds `x` **up** to the first `a` boundary at or after `x`. If `x` is already aligned, the result remains `x`.
- `ALIGN_DOWN(x, a)` rounds `x` **down** to the containing `a` boundary.
- `IS_ALIGNED(x, a)` tests whether `x` is a multiple of `a`.
- **Constraint:** The alignment argument `a` must be a power of two.

## Page Alignment (`include/linux/mm.h`)

- `PAGE_ALIGN(addr)` expands to `ALIGN(addr, PAGE_SIZE)`.
- `PAGE_ALIGN_DOWN(addr)` expands to `ALIGN_DOWN(addr, PAGE_SIZE)`.
- `PAGE_ALIGNED(addr)` checks `IS_ALIGNED((unsigned long)(addr), PAGE_SIZE)`.
- **Unit rule:** `PAGE_ALIGN*` helpers operate on byte addresses, never PFNs.

## Pageblock Alignment (`include/linux/pageblock-flags.h`)

- `pageblock_nr_pages` is `1UL << pageblock_order`.
- **Unit rule:** `pageblock_*` helpers operate on PFNs, never byte addresses:

```c
#define pageblock_align(pfn)       ALIGN((pfn), pageblock_nr_pages)
#define pageblock_aligned(pfn)     IS_ALIGNED((pfn), pageblock_nr_pages)
#define pageblock_start_pfn(pfn)   ALIGN_DOWN((pfn), pageblock_nr_pages)
#define pageblock_end_pfn(pfn)     ALIGN((pfn) + 1, pageblock_nr_pages)
```

- `pageblock_start_pfn(pfn)` returns the **inclusive** start PFN of the pageblock containing `pfn`.
- `pageblock_end_pfn(pfn)` returns the **exclusive** end PFN of the pageblock containing `pfn` (the start of the next pageblock).
- `pageblock_align(pfn)` rounds upward to the first pageblock boundary at or after `pfn`. For an unaligned `pfn`, `pageblock_align(pfn)` equals `pageblock_end_pfn(pfn)`, NOT `pageblock_start_pfn(pfn)`.

| PFN (`pageblock_nr_pages == 512`) | `pageblock_start_pfn()` | `pageblock_align()` | `pageblock_end_pfn()` |
|---|---|---|---|
| 512 | 512 | 512 | 1024 |
| 513 | 512 | 1024 | 1024 |

## Review Invariants & Bug Patterns

### Critical Review Invariants
1. **Unit Mismatch:** Passing a PFN to `PAGE_ALIGN*` or a byte address to `pageblock_*` is a logic error.
2. **Integer Overflow:** When calculating `ALIGN(x, a)`, ensure `x + (a - 1)` does not overflow the type width, especially for user-controlled inputs.
3. **Exclusive Boundaries:** Always treat `pageblock_end_pfn(pfn)` as an exclusive boundary.
4. **Redundant Arithmetic:** Avoid manual pointer arithmetic if the logic already accounts for alignment; this prevents "double-alignment" bugs.
5. **Architecture-Specific Alignment:** Ensure DMA buffers and network headers use `ALIGN()` to satisfy hardware-specific alignment requirements to prevent alignment faults.

### Bug Prevention Checklist
- [ ] **Is the alignment `a` a power of two?**
- [ ] **Are you using the correct unit?** (Bytes for `PAGE_ALIGN`, PFNs for `pageblock_align`).
- [ ] **Is the calculation overflow-safe?**
- [ ] **Are you treating `pageblock_end_pfn` as exclusive?**
- [ ] **Are you using `pageblock_start_pfn` when you need the block's base?**

## Allocator Alignment

Do not confuse `PAGE_ALIGNED(addr)` with page-allocator origin: `kmalloc()` of a power-of-two size is aligned to at least that size, so `kmalloc(PAGE_SIZE, ...)` satisfies `PAGE_ALIGNED()`.
