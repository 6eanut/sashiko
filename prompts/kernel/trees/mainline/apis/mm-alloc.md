# Memory Allocation, GFP, Slab, and Mempool APIs

## GFP Flags and Context Predicates (`include/linux/gfp.h`)

* **`__GFP_KSWAPD_RECLAIM` in `GFP_ATOMIC` / `GFP_NOWAIT`**: Both include `__GFP_KSWAPD_RECLAIM`, which wakes `kswapd` via `try_to_wake_up()` and acquires scheduler/timer locks. Allocations under scheduler-internal locks (runqueue lock, hrtimer base lock) must strip `__GFP_KSWAPD_RECLAIM` (e.g., via `gfp_nested_mask()` or bare `__GFP_NOWARN`).
* **Testing Blocking / Spinning Capability (`gfpflags_allow_blocking` / `gfpflags_allow_spinning`)**:
  * `current_gfp_context()` strips `__GFP_IO` / `__GFP_FS` under `memalloc_noio_save()` / `memalloc_nofs_save()`.
  * Never test `(gfp & GFP_KERNEL) == GFP_KERNEL` to check if an allocation can sleep (that misclassifies `GFP_NOFS`/`GFP_NOIO` as atomic); use `gfpflags_allow_blocking(gfp)` (`__GFP_DIRECT_RECLAIM`) or `gfpflags_allow_spinning(gfp)` (`__GFP_RECLAIM`).
* **Placement Flags (`GFP_ZONEMASK` and `__GFP_THISNODE`)**: Pool-based allocators intercepting allocations (e.g., KFENCE, swiotlb) must check both `GFP_ZONEMASK` and `__GFP_THISNODE` (which is not in `GFP_ZONEMASK`). Pages allocated with `__GFP_MOVABLE` must have unconditional migration/reclaim support.
* **`__GFP_ACCOUNT` and `SLAB_ACCOUNT`**: Slabs created with `SLAB_ACCOUNT` are charged automatically; kthreads or helpers charging a non-current memcg must wrap allocations with `old = set_active_memcg(memcg); ...; set_active_memcg(old);`.

## `kmalloc` Alignment and Backing-Page Restrictions (`include/linux/slab.h`)

* **Natural Alignment Guarantee**: `kmalloc()` / `kzalloc()` guarantees alignment to at least `ARCH_KMALLOC_MINALIGN`, and **for power-of-two sizes, alignment is at least the size itself** (e.g., `kmalloc(PAGE_SIZE, ...)` is always `PAGE_SIZE`-aligned), even with `SLUB_DEBUG`, redzones, `KASAN`, or `kmemleak` enabled. Do **not** flag power-of-two `kmalloc` buffers as potentially unaligned.
* **Shared Backing-Page Restrictions**: `kmalloc` objects share slab pages (`struct slab` overlays `struct page` with refcount 0 since v6.14).
  * Never call `get_page()`/`put_page()`, `SetPage*()`, modify `page->mapping`/`private`/`lru`, or map a `kmalloc` page to userspace (`vm_insert_page()`, `remap_pfn_range()`).
  * Converting a `kmalloc` buffer via `virt_to_page()` / `virt_to_phys()` for DMA or scatterlists (`sg_set_buf()`, `dma_map_single()`) is completely valid.
* **`__GFP_DMA32` Ignored by `kmalloc`**: `kmalloc_type()` ignores `__GFP_DMA32` (`KMALLOC_NOT_NORMAL_BITS` only includes `__GFP_DMA`, `__GFP_RECLAIMABLE`, `__GFP_ACCOUNT`), so `kmalloc(..., GFP_DMA32)` can silently return memory above 4 GiB.
* **`kmemleak` Pairing**: Pair `kmalloc()` with `kfree()`/`kfree_rcu()` and `kmalloc_nolock()` with `kfree_nolock()`. Because SLUB skips `kmemleak` registration when `!gfpflags_allow_spinning(flags)`, guard `kmemleak_not_leak()` / `kmemleak_ignore()` / `kmemleak_no_scan()` with the same condition.

## `mempool`, `memblock`, and NUMA Node APIs

* **`mempool_alloc()` (`include/linux/mempool.h`)**: Retries forever and never returns `NULL` when `gfpflags_allow_blocking(gfp)` is true (`GFP_KERNEL`, `GFP_NOFS`, `GFP_NOIO`), so `NULL` checks are dead code; when `!gfpflags_allow_blocking(gfp)` (`GFP_ATOMIC`, `GFP_NOWAIT`), it can return `NULL` and must be checked.
* **`memblock` `(base, size)` vs `(start, end)` (`include/linux/memblock.h`)**: `memblock_add()` / `memblock_remove()` / `memblock_reserve()` take `(base, size)`, whereas `reserve_bootmem_region()` and `__memblock_find_range_*()` take `(start, end)`.
* **NUMA Node Validation and Iteration**:
  * Validate untrusted node IDs before `NODE_DATA(nid)` with `nid >= 0 && nid < MAX_NUMNODES && node_state(nid, N_MEMORY)`.
  * Iterate memory-bearing nodes with `for_each_node_state(nid, N_MEMORY)` (not `for_each_online_node()`, which includes memoryless nodes), and bound raw node ID loops by `nr_node_ids` (not `num_node_state()`, which is a count of sparse IDs).
  * `alloc_pages_node()` / `__alloc_pages_node()` bypass task NUMA mempolicy; preserve `alloc_pages()` / `folio_alloc()` when `nid == NUMA_NO_NODE`.
