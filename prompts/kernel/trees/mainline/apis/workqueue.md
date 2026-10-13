# Workqueue Lifecycle & Teardown API (`include/linux/workqueue.h`, `kernel/workqueue.c`)

## Queued Work & Teardown Guarantees (`queue_work`, `flush_work`, `cancel_work_sync`, `disable_work_sync`)

- Once a `struct work_struct` is queued via `queue_work()` / `schedule_work()`, synchronous workqueue teardown helpers (`flush_work()`, `cancel_work_sync()`, `disable_work_sync()`, `flush_workqueue()`, `destroy_workqueue()`) wait for any in-flight or pending execution to finish before returning.
- Do not flag a queued `work_struct` as leaked or dangling across teardown when the teardown path drains or cancels the work item (`cancel_work_sync()`, `disable_work_sync()`, or `flush_work()` when re-queuing is already prevented) or destroys its workqueue before freeing the enclosing structure.
