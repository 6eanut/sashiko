# FUSE Subsystem Context (`fs/fuse/`)

## `fch->ring` Publication & Lockless Readers (`fs/fuse/dev_uring.c`)

- `fc->ring` / `fch->ring` is published with `smp_store_release()` after its fields are initialized.
- Lockless readers that only dereference through the pointer rely on the address dependency and only need `READ_ONCE()` (not `smp_load_acquire()`). Do **not** flag lockless `READ_ONCE(fch->ring)` readers as missing an acquire barrier.

## `fuse_uring_cmd()` SQE Access Stability (`fs/fuse/dev_uring.c`)

- `fuse_uring_cmd()` reads SQE fields synchronously at first issue via `io_uring_sqe128_cmd(cmd->sqe, ...)` and `READ_ONCE(cmd->sqe->...)` before returning `-EIOCBQUEUED`, and never calls `io_uring_cmd_issue_blocking()`.
- Because `io_uring` keeps the SQE slot valid during initial issue and copies the full SQE (`io_req_sqe_copy()`) before punting to `io-wq`, poll, or a link chain, reading `cmd->sqe` inside `fuse_uring_cmd()` without a separate prep hook is safe and must **not** be flagged as a bug.
