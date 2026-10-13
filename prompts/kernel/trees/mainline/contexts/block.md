# Block Layer Subsystem Context (Linux v7.3-rc5)

## 1. Queue Freezing Synchronization (`block/blk-mq.c`)

- Every bio submission path acquires `q->q_usage_counter` (`percpu_ref`) via `blk_try_enter_queue()` (`blk_queue_enter()`) and releases it via `blk_queue_exit()`.
- While a queue is frozen (`blk_mq_freeze_queue()`), `blk_try_enter_queue()` fails and all in-flight references have drained, allowing safe teardown or reconfiguration of queue-wide state (such as `q->nr_requests`, hardware tag maps, or elevator data).

## 2. Elevator `depth_updated` Callback (`block/elevator.h`, `block/blk-mq.c`)

- `void (*depth_updated)(struct request_queue *)` in `struct elevator_mq_ops` recomputes per-queue scheduler depth limits derived from `q->nr_requests` (e.g., `async_depth` in `mq-deadline` and `kyber`, `async_depths` in `bfq`).
- **Ordering invariant in `blk_mq_update_nr_requests()`:** `q->nr_requests` must be assigned *before* invoking `q->elevator->type->ops.depth_updated(q)`. Reordering the assignment after the callback causes elevators to compute limits from the stale `nr_requests` value.
- **Initialization invariant:** Every elevator that derives state from `q->nr_requests` must invoke its `depth_updated` implementation at the end of `init_sched()` (`bfq_init_queue()` -> `bfq_depth_updated()`, `dd_init_sched()` -> `dd_depth_updated()`, `kyber_init_sched()` -> `kyber_depth_updated()`), or its depth limits remain zero until a runtime sysfs write to `nr_requests`.

## 3. Synchronization Model & Concurrency Rules

- **Lock Hierarchy:**
    1. `q->q_usage_counter`: Must be held (via `blk_queue_enter`) for any I/O submission.
    2. `q->queue_lock`: Protects legacy queue state and specific scheduler data.
    3. `hctx->lock`: Protects the hardware dispatch list.
    4. `elevator_data` lock: Specific to the I/O scheduler (e.g., `bfq_data->lock`).
- **Atomic Context:** `blk_mq_run_hw_queue()` and `blk_mq_request_issue_directly()` may be called from atomic contexts. Memory allocations within these paths must use `GFP_ATOMIC`.
- **RCU:** The `blkcg` associations are protected by RCU. Readers must use `rcu_read_lock()` when traversing lists.
- **Interrupt Masking:** When modifying shared hardware registers or queue state from both process and interrupt context, `spin_lock_irqsave()` is mandatory.

## 4. Initialization & Error-Path Unwind

- **Allocation:** `blk_mq_alloc_tag_set()` followed by `blk_mq_alloc_queue()`.
- **Publication:** The queue is live only after `device_add_disk()` is called.
- **Symmetry:** Every `blk_mq_alloc_tag_set()` must have a corresponding `blk_mq_free_tag_set()`.
- **Error-Path:** If `blk_mq_alloc_tag_set()` fails, all previously allocated resources (e.g., `blk_mq_hw_ctx` private data) must be freed.
- **Return Values:** Block layer functions (e.g., `blk_mq_alloc_request`) return `ERR_PTR` on failure. Always use `IS_ERR()` before dereferencing.

## 5. High-Frequency Bug Patterns & False-Positive Guards

- **Stable Pages:** If the driver performs DMA, it must ensure pages are stable during I/O. Use `bio_flagged(bio, BIO_NO_PAGE_REF)` if the driver supports it.
- **Shutdown Quiesce:** Drivers must implement a `shutdown` callback that calls `blk_mq_freeze_queue()` and `blk_mq_quiesce_queue()` to prevent DMA into freed memory.
- **`blk_mq_ctx` access:** Accessing `rq->mq_ctx` is safe only if the request is currently owned by the block layer or the driver.
- **`bio_for_each_segment_all`:** Strictly for internal block layer use (e.g., bio splitting). Drivers must use `bio_for_each_segment` to respect the `bi_iter` state.

## 6. Storage Controllers & DMA

- **DMA Mapping:** All `dma_map_sg()` calls must be paired with `dma_unmap_sg()` using the exact same `nents` and `direction` parameters.
- **Hardware State:** Before accessing registers, verify the device is not in a suspended state (`pm_runtime_get_sync()`).
- **Alignment:** Scatter-gather lists must respect `queue_limits` (e.g., `logical_block_size`, `physical_block_size`, `dma_alignment`).
- **Arithmetic Integrity:** All user-provided offsets and lengths (e.g., via `ioctl` or `sysfs`) must be validated against `get_capacity(disk)`.
- **Endianness:** Bitwise operations on hardware registers must use `cpu_to_le32()` or `le32_to_cpu()` to ensure cross-architecture compatibility.

## 7. Block Cgroup & Statistics Invariants

- **`blkg_rwstat`:** Counters are per-cpu. Use `blkg_rwstat_read()` for snapshots. `blkg_rwstat_total()` provides atomic u64 access without explicit locking.
- **`blkcg_gq` (blkg):** The association between `blkcg` and `request_queue`. All non-root `blkg`s are guaranteed access to their parent.
- **`iostat`:** `blkg_iostat_set` uses `u64_stats_sync` to protect against partial reads on 32-bit architectures.
- **Lifecycle:** `blkg` destruction is deferred via `free_work` (a `work_struct`) to ensure RCU safety. Never access a `blkg` after the final `percpu_ref` has dropped without holding an RCU read lock.

## 8. BFQ Scheduler Specifics

- **Weights:** `BFQ_MIN_WEIGHT` (1) to `BFQ_MAX_WEIGHT` (1000).
- **Service Trees:** `bfq_service_tree` manages active/idle entities via `rb_root`. All operations are protected by the `bfqd->queue_lock`.
- **Actuators:** `BFQ_MAX_ACTUATORS` (8) defines the static array size for multi-actuator drive support.
- **Context:** BFQ components must be invoked within the context of the request queue lock to maintain consistency of the B-WF2Q+ scheduler state.
