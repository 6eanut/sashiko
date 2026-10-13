# io_uring Subsystem Code Context (`io_uring/`)

## Request Cleanup, Async Data, and Zero-Copy Lifetimes

* **Zero-Copy Buffer Attachment (`IORING_OP_SEND_ZC`, `IORING_OP_SENDMSG_ZC`)**:
  * `io_alloc_notif()` must precede buffer import, and `io_import_reg_buf(sr->notif, ...)` / `io_import_reg_vec(ITER_SOURCE, &msg_iter, sr->notif, ...)` must attach `buf_node` to `sr->notif` (completed via `io_tx_ubuf_complete()` when TX finishes), never to `req`.
  * `io_notif_flush()` drops the notification reference; immediately set `zc->notif = NULL` afterward so cleanup (`io_send_zc_cleanup()`) does not flush twice.
* **`REQ_F_NEED_CLEANUP` Placement**: Set `req->flags |= REQ_F_NEED_CLEANUP` immediately after any allocation whose teardown relies on `io_cold_defs[req->opcode].cleanup`, with no early-return error path in between. Clear `REQ_F_NEED_CLEANUP` / `REQ_F_ASYNC_DATA` only after confirming the resource was recycled or freed (e.g., inside the `io_alloc_cache_put()` success branch in `io_netmsg_recycle()`).
* **Async Data Helpers (`io_uring/io_uring.h`)**:
  * Allocate in `prep` (not `issue`) via `io_uring_alloc_async_data(cache, req)` (sets `REQ_F_ASYNC_DATA`), free via `io_req_async_data_free(req)`, and clear via `io_req_async_data_clear(req, extra_flags)`. Never manually assign `req->async_data` without `REQ_F_ASYNC_DATA` or call `kfree(req->async_data)` directly.
* **`uring_cmd` SQE Stability (`io_uring/uring_cmd.c`)**:
  * Unlike standard opcodes (which read SQE fields via `READ_ONCE()` in `prep`), `f_op->uring_cmd()` may read `ioucmd->sqe` at issue time because `io_uring_cmd_sqe_copy()` copies the SQE into `ac->sqes`, updates `ioucmd->sqe`, and sets `REQ_F_SQE_COPIED` on async punt. Do not flag `->uring_cmd()` reading `ioucmd->sqe` at issue time; only flag reading `ioucmd->sqe` from a deferred completion callback after returning `-EIOCBQUEUED`.

## CQE Sizing, Multishot, and Buffer Rings

* **`cqe32` Parameter (`io_get_cqe` / `io_get_cqe_overflow`)**: Means "mixed-mode (`IORING_SETUP_CQE_MIXED`) per-CQE 32-byte entry needing extra slot advancement," so derive `cqe32` from `cqe.flags & IORING_CQE_F_32`, **never** from `ctx->flags & IORING_SETUP_CQE32` (where ring slot doubling is handled internally).
* **Multishot & `io_req_post_cqe()`**:
  * `io_req_post_cqe()` requires `task_work` context with `uring_lock` held (never `io-wq`) and requires `REQ_F_MULTISHOT` or `REQ_F_APOLL_MULTISHOT` on `req`.
  * Check `issue_flags & IO_URING_F_MULTISHOT` (not just `REQ_F_APOLL_MULTISHOT`) before returning multishot-specific status codes (`IOU_STOP_MULTISHOT`), and never return `-EAGAIN` for `io-wq` punt from a multishot handler.
* **Provided Buffer Rings (`struct io_uring_buf`, `io_uring/kbuf.c`)**:
  * Ring entries reside in userspace-shared memory: read `buf->len`, `buf->addr`, and `buf->bid` with `READ_ONCE()` and write with `WRITE_ONCE()` (capture `buf->addr` before `io_kbuf_commit()`).
  * Opcodes that commit provided buffers explicitly must be exempted in `io_should_commit()`. On partial completion retry, commit via `io_kbuf_commit()` before setting `REQ_F_BL_NO_RECYCLE`. `io_put_kbufs()` takes the current transfer count (`this_ret`), not cumulative total.
* **Registered Buffers (`io_uring/rsrc.c`)**:
  * Never assume `imu->ubuf` is page/folio-aligned; add `imu->bvec[0].bv_offset` (`offset = (buf_addr - imu->ubuf) + imu->bvec[0].bv_offset`) rather than address-masking.
  * Unpin registered buffers with `unpin_user_folio()`, never `unpin_user_page()`.

## Cross-Ring `msg_ring`, SQPOLL, Task Work, IOPOLL, and Timeouts

* **`msg_ring` Remote Requests (`io_uring/msg_ring.c`)**: Requests allocated via `io_msg_data_remote()` must set `req->tctx = NULL`, must never be placed in `io_alloc_cache`, and must be freed via `kfree_rcu(req, rcu_head)` (`io_msg_tw_complete()`).
* **SQPOLL Thread (`sqd->thread`, `io_uring/sqpoll.h`)**: `__rcu`-annotated; read via `sqpoll_task_locked(sqd)` (under `sqd->lock`) or `rcu_dereference(sqd->thread)` (under RCU), assign via `rcu_assign_pointer()`, signal via `req->tctx->task`, and never call `put_task_struct()` in the creator after `wake_up_new_task()`.
* **`IORING_SETUP_DEFER_TASKRUN` Cancel Loops**: Any non-submitter cancellation loop calling `io_uring_try_cancel_requests()` (such as `io_ring_exit_work()`) must call `io_move_task_work_from_local()` on **every** loop iteration, not just once before the loop.
* **IOPOLL Completion (`io_complete_rw_iopoll`, `io_uring/rw.c`)**: Must always reach `smp_store_release(&req->iopoll_completed, 1)` without early return; on `-EAGAIN`, set `REQ_F_REISSUE | REQ_F_BL_NO_RECYCLE` and fall through.
* **Timeout Lock Ordering (`io_uring/timeout.c`)**: `ctx->timeout_lock` is a `raw_spinlock_t`. Never queue `task_work` or invoke completions (`io_eventfd_signal()` takes a regular spinlock, invalid under `raw_spinlock_t` on `PREEMPT_RT`) while holding `ctx->timeout_lock`; move killed timeouts to a local list under `timeout_lock` (`io_kill_timeout()`) and complete them after unlocking (`io_flush_killed_timeouts()`).
* **Additional Invariants**:
  * Set `ctx->cqe_sentinel = ctx->cqe_cached` before dropping `completion_lock` in `__io_cqring_overflow_flush()`.
  * Clear registration tags via `io_clear_table_tags()` before unregistering a failed all-or-nothing resource registration.
  * Call `io_req_track_inflight()` in `prep` for requests that depend on the submitter's `mm` (e.g., `io_futex_prep()`).
  * Use `io_req_queue_tw_complete()` (never fabricate `io_tw_token_t` on the stack) outside `task_work` context, and use `io_eventfd_put()` (never `io_eventfd_free()` directly).
  * Ring-geometry mutations (`io_register_resize_rings()`) require `IORING_SETUP_DEFER_TASKRUN`.
  * Generic poll (`io_uring/poll.c`) must not treat `POLLERR` as a generic error; leave event bit interpretation to opcode issue handlers.

## Synchronization and Concurrency Contracts
* **Lock Hierarchy**: `ctx->uring_lock` is the primary mutex for submission/completion. `ctx->completion_lock` protects the CQ ring and overflow list. `ctx->timeout_lock` (raw spinlock) must never be held while calling functions that might sleep or acquire mutexes.
* **Task Context**: `io-wq` workers are distinct from the submitter task. Accessing `current->mm` or `current->files` from `io-wq` is only safe if the request was prepared with `REQ_F_NEED_CLEANUP` or explicitly tracked via `io_req_track_inflight()`.
* **Memory Barriers**: Use `smp_load_acquire` and `smp_store_release` for `req->iopoll_completed` and ring head/tail pointers to ensure visibility across CPU cores.

## Memory Safety and Allocation
* **Cache Management**: Use `io_cache_alloc()` and `io_cache_free()` for high-frequency objects (e.g., `io_kiocb`). Ensure `io_alloc_cache_put()` returns `true` before assuming the object is cached; otherwise, use `kvfree()`.
* **Buffer Validation**: Always validate `sqe->len` and `sqe->addr` against `rsrc_node` limits. Use `iov_iter_count()` to verify remaining bytes before performing `copy_from_user` or `copy_to_user`.
* **Integer Overflows**: All size calculations involving `sqe` parameters must use `check_add_overflow()` or `check_mul_overflow()` to prevent wrapping, especially when calculating buffer offsets or array indices.
