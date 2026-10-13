# MM Page Allocator, SLUB, and vmalloc Code Context (`mm/`)

## Page Allocator Internals (`mm/page_alloc.c`)

* **Frozen vs Refcounted Pages**: `get_page_from_freelist()`, `__alloc_frozen_pages_noprof()`, and `alloc_frozen_pages_nolock_noprof()` return pages with refcount 0 ("frozen"); `__alloc_pages_noprof()` calls `set_page_refcounted()` to set refcount 1. Never pass a frozen page to callers expecting refcount 1 without `set_page_refcounted()`.
* **Zone Watermarks & `lowmem_reserve`**:
  * `zone[i].lowmem_reserve[j]` protects lower zone `i` from allocations targeting classzone `j` (`__zone_watermark_ok()`). Never index `lowmem_reserve` with `zone`'s own index.
  * When `zone->percpu_drift_mark` is set and cached `zone_page_state()` is below it, use `zone_page_state_snapshot()` (`should_reclaim_retry()`, `pgdat_balanced()`).
  * Zone watermarks are 0 until `init_per_zone_wmark_min()` (`postcore_initcall`), and `high_memory` is 0 until `free_area_init()` (use `memblock_end_of_DRAM()` in early `__init` code).
* **`ALLOC_TRYLOCK` (`alloc_pages_nolock`)**: Clears spinning GFP flags (`gfpflags_allow_spinning()` is false). All helpers reachable from `get_page_from_freelist()` must respect `ALLOC_TRYLOCK` / `gfpflags_allow_spinning()` and never call unconditional `spin_lock()`.
* **Slowpath Retry, Pageblocks, and PCP Locks**:
  * Every `goto retry` in `__alloc_pages_slowpath()` must mutate state to guarantee termination and call `check_retry_cpuset()` / `check_retry_zonelist()`.
  * High-order pageblock migratetype updates (`order >= pageblock_order`) must use `change_pageblock_range()`, not single-block `set_pageblock_migratetype()`.
  * `pcp->lock` must use `pcp_spin_trylock()` / `pcp_spin_lock_maybe_irqsave()` (which save IRQs on `CONFIG_SMP=n`), never bare `spin_lock()`.
  * Never call `static_branch_enable()` / `static_branch_disable()` from allocation paths (they take `cpus_read_lock()` and deadlock during CPU hotplug bringup); use `_cpuslocked` variants or defer via workqueue.

## SLUB, vmalloc, and Accounting Internals (`mm/slub.c`, `mm/vmalloc.c`)

* **Layered vmstat Accounting**: `lruvec_stat_mod_folio()` / `mod_lruvec_page_state()` only update memcg counters when `folio_memcg(folio) != NULL`. When a slab/folio is charged after initial allocation (`kmem_cache_charge()`), subtract pre-charge node-only stats (`mod_node_page_state`) and re-add via the lruvec helper so free does not underflow the memcg counter.
* **`struct slab` Overlay & Freelist**:
  * `allocate_slab()` must initialize every `struct slab` field (including `#ifdef`-guarded fields), and `unaccount_slab()` must call `free_slab_obj_exts()` unconditionally (clears `OBJEXTS_ALLOC_FAIL` from `slab->obj_exts` / `folio->memcg_data`).
  * Access slab freelist pointers via `get_freepointer()` / `set_freepointer()` (handles `CONFIG_SLAB_FREELIST_HARDENED` XOR encoding and `kasan_reset_tag()`).
  * In `___slab_alloc()`, any trylock retry path must check `!allow_spin` and bail out to avoid infinite loops when interrupting the lock holder on the same CPU.
  * `get_node(s, numa_mem_id())` can return `NULL` on memoryless NUMA nodes.
* **Realloc Shrink Zeroing & KASAN**: In-place realloc shrink (`__do_krealloc()`, `vrealloc_node_align_noprof()`) must zero `[new_size, old_size)` when `want_init_on_free() || want_init_on_alloc(flags)` is true, and vmalloc realloc must use `kasan_vrealloc()` for unaligned granule boundaries.
* **Direct Map Restoration & User Zeroing**: Pages removed from the direct map (`set_direct_map_invalid_noflush()`, e.g., `secretmem`) must be restored via `set_direct_map_default_noflush()` before being freed to the page allocator. On cache-aliasing architectures, user-mapped pages require `clear_user_highpage()` / `folio_zero_user()` (`user_alloc_needs_zeroing()`) rather than bare `__GFP_ZERO`.
