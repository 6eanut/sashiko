# Block Layer & Bio API Rules (Linux v7.3-rc5)

## Queue Freeze / Unfreeze (`include/linux/blk-mq.h`)

- `unsigned int __must_check blk_mq_freeze_queue(struct request_queue *q)` waits for `q->q_usage_counter` to reach zero and calls `memalloc_noio_save()`, returning the saved `memflags`.
- Callers must capture the returned `memflags` and pass it to `blk_mq_unfreeze_queue(q, memflags)` (or use `scoped_guard(freeze_queue, q)`).
- `blk_mq_freeze_queue()` is a blocking operation; it must not be called in atomic context (e.g., hard IRQ, soft IRQ, or with spinlocks held).
- Quiescing the tagset via `blk_mq_quiesce_queue()` blocks internal commands; ensure the error recovery path does not introduce circular dependencies by requiring commands that are blocked by the quiesced state.

## Bio Data Buffer Access & `bio_has_data()` (`include/linux/bio.h`)

Non-data operations (`REQ_OP_DISCARD`, `REQ_OP_FLUSH`, `REQ_OP_WRITE_ZEROES`, `REQ_OP_SECURE_ERASE`) have `bio->bi_io_vec == NULL`.

- Guard accesses to `bio->bi_io_vec`, `bio->bi_vcnt`, `bio->bi_iter.bi_bvec_done`, `bio_first_bvec_all()`, `bio_for_each_bvec()`, or `bio_for_each_segment()` with `bio_has_data(bio)`.
- **Do NOT use `op_is_write(bio_op(bio))` as a guard for data presence:** `op_is_write()` tests bit 0 of the op code, which is set for `REQ_OP_DISCARD` (3), `REQ_OP_SECURE_ERASE` (5), and `REQ_OP_WRITE_ZEROES` (9) even though they carry no data vectors (`REQ_OP_ZONE_APPEND` (7) also has bit 0 set and *does* carry data). `bio_has_data()` explicitly excludes `REQ_OP_DISCARD`, `REQ_OP_SECURE_ERASE`, and `REQ_OP_WRITE_ZEROES` and checks `bio->bi_iter.bi_size > 0`.
- Always validate I/O request parameters (e.g., `max_sectors`) against hardware alignment constraints; small bios with single bvecs may bypass standard alignment checks if not explicitly validated.

## Bio & Integrity Mempool Allocation Guarantees (`block/bio.c`, `block/bio-integrity*.c`)

- `bio_alloc()` / `bio_alloc_bioset()`: mempool-backed when `bs` is valid; cannot return `NULL` when `gfp` includes `__GFP_DIRECT_RECLAIM` (`GFP_KERNEL`, `GFP_NOFS`, `GFP_NOIO`), **unless** `nr_vecs > 0` and the `bio_set` was initialized without a bvec pool (which triggers `WARN_ON_ONCE` and returns `NULL`). Failure paths are otherwise only reachable with `GFP_NOWAIT` / `GFP_ATOMIC`.
- `bvec_alloc()`: tries slab allocation first; if that fails and `__GFP_DIRECT_RECLAIM` is set, falls back to the `bio_set`'s bvec mempool (cannot fail).
- `void bio_integrity_prep(struct bio *bio, unsigned int action)` (`block/bio-integrity-auto.c`): returns `void`; allocates `struct bio_integrity_data` from `bid_pool` with `GFP_NOIO` and calls `bio_integrity_alloc_buf(bio, GFP_NOIO, ...)`, which cannot fail.
- `void bio_integrity_alloc_buf(struct bio *bio, gfp_t gfp, bool zero_buffer)` (`block/bio-integrity.c`): returns `void`; tries `kmalloc(len, gfp | __GFP_NOWARN | ...)` and falls back to `mempool_alloc(&integrity_buf_pool, gfp)` on failure (infallible when `gfp` includes `__GFP_DIRECT_RECLAIM`).
- `bio_kmalloc()`: uses plain `kmalloc()` with **no** mempool backing and can return `NULL` under any GFP flags; callers must always check for `NULL`.
- Use a dedicated memory pool for operations that might trigger recursive allocations (e.g., splitting GC writes) to avoid deadlocks under memory pressure.

## Lifecycle & Concurrency Invariants

- **Ownership:** Once `submit_bio()` is called, the block layer owns the bio until `bi_end_io` is invoked.
- **Initialization:** Use `bio_init()` for stack-allocated bios; `bio_uninit()` must be called before freeing memory to release `bio_set` resources.
- **Partial Failure:** If a bio is partially submitted or fails before `submit_bio()`, the caller must call `bio_put()` or `bio_uninit()`.
- **Async Handoff:** If a bio is passed to an asynchronous worker, use `bio_get()` before scheduling and `bio_put()` after completion.
- **Synchronous Cancellation:** During device removal, ensure all pending work items and timers are canceled synchronously using `cancel_work_sync()`.
- **Double-Free Guard:** Do not call `bio_put()` if the bio has already been submitted to the block layer, as the completion handler will free it.
- **Atomic Allocation Guard:** If in atomic context, use a pre-allocated `bio_set` or mempool; never use `GFP_KERNEL`.
- **Split Guard:** If a bio is split, the original and split bios have distinct lifecycles; ensure `bi_end_io` logic handles completion of both to prevent leaks.
- **Request Completion:** Ensure that split block requests are finalized exactly once; use shadow request tracking to synchronize completion logic.
- **Error Unwinding:** Every error path in a function that performs dynamic allocation must ensure that all previously allocated resources (including integrity buffers and DMA mappings) are properly released.
- **Polling Limits:** Polling loops must enforce a strict time or iteration bound to prevent infinite spinning in preemptible contexts or RCU CPU stalls.
- **State Transitions:** Avoid setting state flags (e.g., partition scan flags) until the object is fully initialized to prevent race conditions where concurrent openers trigger premature operations.
- **Memory Ordering:** When publishing a newly initialized object to a shared data structure, use `smp_wmb()` or equivalent to ensure all processors see the correct initial state.
- **Lock Hierarchy:** Always acquire sleeping mutexes (`disk->open_mutex`, `q->limits_lock`, `q->sysfs_lock`) before the `q->queue_lock` spinlock (`disk->open_mutex` -> `q->queue_lock`). Never attempt to acquire `disk->open_mutex` or any mutex while holding `q->queue_lock`.
- **I/O Accounting:** Every `part_stat_inc()` or `part_stat_add()` call must be balanced by a corresponding completion accounting call in all exit paths, including error paths.
- **Alignment:** When calculating buffer boundaries, account for the current offset within the data structure to prevent integer underflow or out-of-bounds access.
- **Device-Managed Cleanup:** Use `devm_kzalloc()` and `devm_add_action()` for resources tied to the device lifecycle; manual cleanup is only required for resources allocated outside the `devm` framework.
- **Idempotency:** Ensure that state-transition functions are idempotent; repeated calls should not result in inconsistent hardware state or resource leaks.
