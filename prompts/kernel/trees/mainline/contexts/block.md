# Block Layer Subsystem Context

## Queue Freezing Synchronization (`block/blk-mq.c`)

- Every bio submission path acquires `q->q_usage_counter` (`percpu_ref`) via `blk_try_enter_queue()` (`blk_queue_enter()`) and releases it via `blk_queue_exit()`.
- While a queue is frozen (`blk_mq_freeze_queue()`), `blk_try_enter_queue()` fails and all in-flight references have drained, allowing safe teardown or reconfiguration of queue-wide state (such as `q->nr_requests`, hardware tag maps, or elevator data).

## Elevator `depth_updated` Callback (`block/elevator.h`, `block/blk-mq.c`)

- `void (*depth_updated)(struct request_queue *)` in `struct elevator_mq_ops` recomputes per-queue scheduler depth limits derived from `q->nr_requests` (e.g., `async_depth` in `mq-deadline` and `kyber`, `async_depths` in `bfq`).
- **Ordering invariant in `blk_mq_update_nr_requests()`:** `q->nr_requests` must be assigned *before* invoking `q->elevator->type->ops.depth_updated(q)`. Reordering the assignment after the callback causes elevators to compute limits from the stale `nr_requests` value.
- **Initialization invariant:** Every elevator that derives state from `q->nr_requests` must invoke its `depth_updated` implementation at the end of `init_sched()` (`bfq_init_queue()` -> `bfq_depth_updated()`, `dd_init_sched()` -> `dd_depth_updated()`, `kyber_init_sched()` -> `kyber_depth_updated()`), or its depth limits remain zero until a runtime sysfs write to `nr_requests`.
