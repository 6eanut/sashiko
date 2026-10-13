# Memory Allocation, GFP, Slab, and Mempool APIs (Linux v7.3-rc5)

## 1. GFP Flags and Context Predicates (`include/linux/gfp.h`)

*   **`GFP_KERNEL` vs `GFP_NOFS` / `GFP_NOIO`**: Use `GFP_KERNEL` for standard allocations that can sleep. Use `GFP_NOFS` or `GFP_NOIO` when holding filesystem or block layer locks to prevent deadlocks via recursive reclaim.
*   **`__GFP_KSWAPD_RECLAIM` in `GFP_ATOMIC` / `GFP_NOWAIT`**: Both include `__GFP_KSWAPD_RECLAIM`, which wakes `kswapd` via `try_to_wake_up()` and acquires scheduler/timer locks. Allocations under scheduler-internal locks (runqueue lock, hrtimer base lock) must strip `__GFP_KSWAPD_RECLAIM` (e.g., via `gfp_nested_mask()` or bare `__GFP_NOWARN`).
*   **Testing Blocking / Spinning Capability (`gfpflags_allow_blocking` / `gfpflags_allow_spinning`)**:
    *   `current_gfp_context()` strips `__GFP_IO` / `__GFP_FS` under `memalloc_noio_save()` / `memalloc_nofs_save()`.
    *   Never test `(gfp & GFP_KERNEL) == GFP_KERNEL` to check if an allocation can sleep; this misclassifies `GFP_NOFS`/`GFP_NOIO` as atomic.
    *   Use `gfpflags_allow_blocking(gfp)` (`__GFP_DIRECT_RECLAIM`) to determine if the caller can sleep.
    *   Use `gfpflags_allow_spinning(gfp)` (`__GFP_RECLAIM`) to determine if the caller can spin on locks.
*   **Placement Flags (`GFP_ZONEMASK` and `__GFP_THISNODE`)**: Pool-based allocators intercepting allocations (e.g., KFENCE, swiotlb) must check both `GFP_ZONEMASK` and `__GFP_THISNODE` (which is not in `GFP_ZONEMASK`). Pages allocated with `__GFP_MOVABLE` must have unconditional migration/reclaim support.
*   **`__GFP_ACCOUNT` and `SLAB_ACCOUNT`**: Slabs created with `SLAB_ACCOUNT` are charged automatically; kthreads or helpers charging a non-current memcg must wrap allocations with `old = set_active_memcg(memcg); ...; set_active_memcg(old);`. All user-creatable objects must use `__GFP_ACCOUNT`.

## 2. `kmalloc` Alignment and Backing-Page Restrictions (`include/linux/slab.h`)

*   **Natural Alignment Guarantee**: `kmalloc()` / `kzalloc()` guarantees alignment to at least `ARCH_KMALLOC_MINALIGN`. For power-of-two sizes, alignment is at least the size itself (e.g., `kmalloc(PAGE_SIZE, ...)` is always `PAGE_SIZE`-aligned), even with `SLUB_DEBUG`, redzones, `KASAN`, or `kmemleak` enabled. Do not flag power-of-two `kmalloc` buffers as potentially unaligned.
*   **Shared Backing-Page Restrictions**: `kmalloc` objects share slab pages (`struct slab` overlays `struct page` with refcount 0).
    *   Never call `get_page()`/`put_page()`, `SetPage*()`, modify `page->mapping`/`private`/`lru`, or map a `kmalloc` page to userspace (`vm_insert_page()`, `remap_pfn_range()`).
    *   Converting a `kmalloc` buffer via `virt_to_page()` / `virt_to_phys()` for DMA or scatterlists (`sg_set_buf()`, `dma_map_single()`) is valid.
*   **`__GFP_DMA32` Ignored by `kmalloc`**: `kmalloc_type()` ignores `__GFP_DMA32` (`KMALLOC_NOT_NORMAL_BITS` only includes `__GFP_DMA`, `__GFP_RECLAIMABLE`, `__GFP_ACCOUNT`), so `kmalloc(..., GFP_DMA32)` can silently return memory above 4 GiB.
*   **`kmemleak` Pairing**: Pair `kmalloc()` with `kfree()`/`kfree_rcu()` and `kmalloc_nolock()` with `kfree_nolock()`. Because SLUB skips `kmemleak` registration when `!gfpflags_allow_spinning(flags)`, guard `kmemleak_not_leak()` / `kmemleak_ignore()` / `kmemleak_no_scan()` with the same condition.
*   **`krealloc` Pattern**: Always assign `krealloc()` to a temporary pointer. If it returns `NULL`, the original pointer remains valid and must be freed.

## 3. `mempool`, `memblock`, and NUMA Node APIs

*   **`mempool_alloc()` (`include/linux/mempool.h`)**: Retries forever and never returns `NULL` when `gfpflags_allow_blocking(gfp)` is true; `NULL` checks are dead code in these cases. When `!gfpflags_allow_blocking(gfp)`, it can return `NULL` and must be checked.
*   **`memblock` (`include/linux/memblock.h`)**: `memblock_add()` / `memblock_remove()` / `memblock_reserve()` take `(base, size)`. `reserve_bootmem_region()` and `__memblock_find_range_*()` take `(start, end)`.
*   **NUMA Node Validation and Iteration**:
    *   Validate untrusted node IDs before `NODE_DATA(nid)` with `nid >= 0 && nid < MAX_NUMNODES && node_state(nid, N_MEMORY)`.
    *   Iterate memory-bearing nodes with `for_each_node_state(nid, N_MEMORY)` (not `for_each_online_node()`, which includes memoryless nodes).
    *   Bound raw node ID loops by `nr_node_ids` (not `num_node_state()`, which is a count of sparse IDs).
    *   `alloc_pages_node()` / `__alloc_pages_node()` bypasses task NUMA mempolicy; use `alloc_pages()` / `folio_alloc()` when `nid == NUMA_NO_NODE`.

## 4. Critical Invariants and Bug Patterns

*   **Initialization Failure**: Use the "goto-cleanup" pattern. Unwind in exact reverse order of allocation.
*   **Integer Overflows**: Use `size_mul()`, `size_add()`, or `array_size()` for all allocation size calculations.
*   **Devres (`devm_`)**: Do not mix manual `kfree()` with `devm_` allocated memory. If a `devm_` resource must be freed early, use `devm_kfree()`.
*   **Publication**: Objects must be fully initialized before exposure to global lists or device registration. Use `rcu_assign_pointer()` for lockless publication.
*   **Synchronous Cancellation**: Cancel asynchronous work (timers, workqueues) via `cancel_work_sync()` before freeing associated memory.
*   **RCU Identity**: With `SLAB_TYPESAFE_BY_RCU`, perform an identity check (e.g., `obj->key == expected_key`) after acquiring a reference to ensure the object was not recycled.
*   **Context Safety**: If an allocation occurs within a critical section or transaction, it must be guaranteed to succeed (e.g., `__GFP_NOFAIL`) or handled to prevent failure.
*   **Alignment**: Buffers used for hardware-mediated memory operations must be allocated with sufficient alignment and size to prevent concurrent access or DMA coherency violations.
