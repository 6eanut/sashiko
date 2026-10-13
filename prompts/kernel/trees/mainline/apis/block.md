# Block Layer & Bio API Rules

## Queue Freeze / Unfreeze (`include/linux/blk-mq.h`)

- `unsigned int __must_check blk_mq_freeze_queue(struct request_queue *q)` waits for `q->q_usage_counter` to reach zero and calls `memalloc_noio_save()`, returning the saved `memflags`.
- Callers must capture the returned `memflags` and pass it to `blk_mq_unfreeze_queue(q, memflags)` (or use `scoped_guard(freeze_queue, q)`).

## Bio Data Buffer Access & `bio_has_data()` (`include/linux/bio.h`)

Non-data operations (`REQ_OP_DISCARD`, `REQ_OP_FLUSH`, `REQ_OP_WRITE_ZEROES`, `REQ_OP_SECURE_ERASE`) have `bio->bi_io_vec == NULL`.

- Guard accesses to `bio->bi_io_vec`, `bio->bi_vcnt`, `bio->bi_iter.bi_bvec_done`, `bio_first_bvec_all()`, `bio_for_each_bvec()`, or `bio_for_each_segment()` with `bio_has_data(bio)`.
- **Do NOT use `op_is_write(bio_op(bio))` as a guard for data presence:** `op_is_write()` tests bit 0 of the op code, which is set for `REQ_OP_DISCARD` (3), `REQ_OP_SECURE_ERASE` (5), and `REQ_OP_WRITE_ZEROES` (9) even though they carry no data vectors (`REQ_OP_ZONE_APPEND` (7) also has bit 0 set and *does* carry data). `bio_has_data()` explicitly excludes `REQ_OP_DISCARD`, `REQ_OP_SECURE_ERASE`, and `REQ_OP_WRITE_ZEROES` and checks `bio->bi_iter.bi_size > 0`.

## Bio & Integrity Mempool Allocation Guarantees (`block/bio.c`, `block/bio-integrity*.c`)

- `bio_alloc()` / `bio_alloc_bioset()`: mempool-backed when `bs` is valid; cannot return `NULL` when `gfp` includes `__GFP_DIRECT_RECLAIM` (`GFP_KERNEL`, `GFP_NOFS`, `GFP_NOIO`), **unless** `nr_vecs > 0` and the `bio_set` was initialized without a bvec pool (which triggers `WARN_ON_ONCE` and returns `NULL`). Failure paths are otherwise only reachable with `GFP_NOWAIT` / `GFP_ATOMIC`.
- `bvec_alloc()`: tries slab allocation first; if that fails and `__GFP_DIRECT_RECLAIM` is set, falls back to the `bio_set`'s bvec mempool (cannot fail).
- `void bio_integrity_prep(struct bio *bio, unsigned int action)` (`block/bio-integrity-auto.c`): returns `void`; allocates `struct bio_integrity_data` from `bid_pool` with `GFP_NOIO` and calls `bio_integrity_alloc_buf(bio, GFP_NOIO, ...)`, which cannot fail.
- `void bio_integrity_alloc_buf(struct bio *bio, gfp_t gfp, bool zero_buffer)` (`block/bio-integrity.c`): returns `void`; tries `kmalloc(len, gfp | __GFP_NOWARN | ...)` and falls back to `mempool_alloc(&integrity_buf_pool, gfp)` on failure (infallible when `gfp` includes `__GFP_DIRECT_RECLAIM`).
- `bio_kmalloc()`: uses plain `kmalloc()` with **no** mempool backing and can return `NULL` under any GFP flags; callers must always check for `NULL`.
