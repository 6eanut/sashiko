# FUSE Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for the FUSE subsystem (`fs/fuse/`). It is intended for automated patch review and architectural validation.

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **Core Objects:**
    *   `fuse_conn`: Primary connection object representing a FUSE mount.
    *   `fuse_chan`: Communication channel to userspace.
    *   `fuse_ring`: `io_uring` integration structure.
    *   `fuse_req`: Fundamental unit of work.
*   **Object Lifetimes:**
    *   `fuse_conn` uses reference counting (`fuse_conn_get`/`put`).
    *   `fuse_ring_ent` (ring entries) lifecycle is managed within `fuse_ring_queue`. Transitions between `fuse_ring_req_state` must be strictly serialized via `queue->lock`.
    *   **Teardown:** When a `fuse_chan` or `fuse_conn` is destroyed, all pending `fuse_req` objects must be aborted. Failure to wait for completion leads to use-after-free if asynchronous workers access the request.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Locking Hierarchy:**
    *   `queue->lock` (in `fuse_ring_queue`): Protects all state transitions of `fuse_ring_ent` and list movements within the ring queue.
*   **Lockless Reads & Publication:**
    *   `fc->ring` / `fch->ring` is published with `smp_store_release()` after its fields are initialized.
    *   Lockless readers that only dereference through the pointer rely on the address dependency and only need `READ_ONCE()`. Do **not** flag lockless `READ_ONCE(fch->ring)` readers as missing an acquire barrier.
    *   `smp_load_acquire()` is required only when the reader must ensure that subsequent memory accesses (e.g., dereferencing fields within the ring structure) are ordered after the pointer load.
*   **Concurrency Rules:**
    *   Operations that may sleep (e.g., `fuse_chan_send`) must not be called while holding spinlocks or in atomic contexts.
    *   State flags (e.g., `fuse_ring_req_state`) must be checked under `queue->lock` to prevent TOCTOU races.

## 3. Initialization, Registration, Asynchronous Execution & Teardown Ordering

*   **Initialization:**
    *   `fuse_dev_init()` must be called before device nodes are exposed.
*   **Asynchronous Teardown:**
    *   All pending `fuse_req` objects must be flushed or aborted before memory is freed.
*   **SQE Access Stability (`fs/fuse/dev_uring.c`):**
    *   `fuse_uring_cmd()` reads SQE fields synchronously at first issue via `io_uring_sqe128_cmd(cmd->sqe, ...)` and `READ_ONCE(cmd->sqe->...)` before returning `-EIOCBQUEUED`.
    *   `fuse_uring_cmd()` never calls `io_uring_cmd_issue_blocking()`.
    *   Because `io_uring` keeps the SQE slot valid during initial issue and copies the full SQE (`io_req_sqe_copy()`) before punting to `io-wq`, poll, or a link chain, reading `cmd->sqe` inside `fuse_uring_cmd()` without a separate prep hook is safe and must **not** be flagged as a bug.

## 4. Error Handling, Resource Recovery & Boundary/State Validation

*   **Error Pointers:** Functions returning `struct fuse_conn *` or `struct fuse_chan *` may return `ERR_PTR()`. Always use `IS_ERR()` before dereferencing.
*   **Integer Overflows:** Use `check_add_overflow()` or `check_mul_overflow()` when calculating buffer sizes for `fuse_args` or `fuse_ring_ent` payloads.
*   **State Invariants:**
    *   Verify `FUSE_IO_URING` feature flags before accessing `fuse_ring` structures.
    *   If a request is cancelled, it must be removed from all lists (`ent_avail_queue`, `ent_w_req_queue`, etc.) before being freed.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **False Positive (Lockless Reads):** `READ_ONCE(fch->ring)` is correct for lockless access to the ring pointer.
*   **False Positive (SQE Access):** `fuse_uring_cmd()` access to `cmd->sqe` is safe due to `io_uring`'s internal SQE copying mechanism.
*   **Bug Pattern (Unbalanced Refcount):** Overwriting a pointer to a `fuse_conn` or `fuse_chan` without calling `put()` on the previous object is a leak.
*   **Bug Pattern (Dangling List Entry):** When moving a `fuse_req` between queues, `list_del()` must be called on the old list before `list_add()` on the new one to prevent list corruption.
*   **Bug Pattern (Uninitialized Variables):** Ensure all `fuse_args` fields (especially `in_args` and `out_args`) are zeroed or fully initialized before submission to avoid leaking kernel stack data.
*   **Bug Pattern (Missing Barrier):** Ensure `smp_store_release` is paired with appropriate read-side barriers if the reader is not relying solely on address dependencies.
