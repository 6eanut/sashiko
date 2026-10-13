# SMB/CIFS Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides architectural and safety invariants for `fs/smb/`, covering `smbdirect/` (RDMA) and `server/` (ksmbd) as of Linux v7.3-rc5.

## 1. SMB Direct (RDMA) Credit Grant Ordering (`fs/smb/smbdirect/`)
In the MS-SMBD protocol, no data transfer message carrying a non-zero `credits_granted` may be sent before the SMB Direct negotiate response is sent (on accept) or received (on connect):
- Two work items on `struct smbdirect_socket` (`fs/smb/smbdirect/socket.h`, `fs/smb/smbdirect/connection.c`) drive credit-granting sends:
  - `sc->recv_io.posted.refill_work` (`smbdirect_connection_recv_io_refill_work()`): Posts receive buffers and queues `sc->idle.immediate_work` on `sc->workqueues.immediate` if receive credits were posted.
  - `sc->idle.immediate_work` (`smbdirect_connection_send_immediate_work()`): Sends an empty data transfer PDU granting credits to the peer.
- **Disabled-work initialization pattern:** `smbdirect_socket_init()` (`fs/smb/smbdirect/socket.h`) initializes `sc->recv_io.posted.refill_work` and `sc->idle.immediate_work` with `__smbdirect_socket_disabled_work` (which fires `WARN_ON_ONCE`) and immediately calls `disable_work_sync()` on both so `queue_work()` calls during early receive buffer setup (`accept.c` / `connect.c`) are no-ops until the real handlers are bound via `INIT_WORK()` (`connection.c`) at the appropriate protocol state transition.
- **Invariant:** Any change that removes `disable_work_sync()` in `smbdirect_socket_init()`, reorders `INIT_WORK()` / `enable_work()` relative to negotiation completion, or converts between `work_struct` and `delayed_work` without preserving the disabled state breaks credit grant ordering.

## 2. Synchronization and Lock Hierarchy
- **Lock Ordering:** 
  1. `smbdirect_globals.mutex`
  2. `sc->mutex`
  3. `cfids->de_mutex`
- **Atomic Contexts:** `WQ_MEM_RECLAIM` workqueues (e.g., `smbdirect_globals.workqueues.cleanup`) are permitted to perform blocking allocations. Standard `WQ_PERCPU` workqueues must not perform blocking operations.
- **Serialization:** Access to `cfids->entries` and `cfids->dying` requires holding `cfids->de_mutex`. Use `list_for_each_entry_safe` for iteration. If the mutex is dropped during iteration, re-validate the object (check `on_list` flag) before dereferencing.

## 3. Object Lifecycle and Teardown
- **Reference Counting:** `struct cached_fid` uses `struct kref`. `close_cached_dir()` must be called to decrement the reference count; the `put_work` workqueue item handles final memory release.
- **Quiescing:** All pending `delayed_work` and `work_struct` items must be explicitly cancelled and flushed before the underlying object memory is freed.
- **Publication:** Objects (e.g., `ksmbd_conn`, `smbdirect_socket`) must be fully initialized and linked into global lists only after all internal pointers and workqueues are valid.

## 4. Initialization and Error-Path Unwind
- **Symmetry:** Initialization must follow a strict stack-based unwind. If `smbdirect_devices_init()` fails, destroy workqueues in reverse order of creation (e.g., `cleanup` -> `immediate` -> `refill`).
- **VFS Deadlocks:** Do not call `d_instantiate()` on an inode marked `I_NEW` until the inode is fully initialized and unlocked.
- **Resource Cleanup:** Failed inode reads must be removed from the inode hash to prevent incomplete inodes from lingering in the cache.

## 5. High-Frequency Bug Patterns & Guards
- **Use-After-Free (UAF):** Use `kref_get_unless_zero()` before accessing shared objects in asynchronous workers.
- **Integer Overflows:** Use `check_add_overflow()` or `check_mul_overflow()` when calculating allocation sizes based on user-provided lengths.
- **Out-of-Bounds (OOB) Reads:** Every structure cast must be preceded by `if (remaining_len < sizeof(struct_type)) return -EINVAL;`.
- **Unaligned Access:** Use `get_unaligned_le32()` or `get_unaligned_le64()` when accessing network buffers to prevent alignment faults.
- **Credential Handling:** Asynchronous workers must capture session-specific credentials (`fsuid`, `fsgid`) at task creation; do not rely on `current->fsuid`.
- **Compound Requests:** Update `total_read` length per sub-PDU in compound requests to prevent OOB reads.

## 6. Protocol-Specific Invariants
- **Buffer Validation:** `data_offset` and `data_length` in `struct smbdirect_data_transfer` must be validated against the total received buffer size.
- **VFS Delegation:** Verify `S_ISREG(inode->i_mode)` before applying file-specific operations.
- **Session Management:** Distinguish between idle timeouts and actual credential expiration when destroying sessions.
- **Error Propagation:** Always propagate error codes from nested parsing (e.g., `ksmbd_decode_negTokenInit`) to prevent malformed requests from being treated as successful.
- **Alignment:** Ensure `struct` padding and flexible-array members are handled via `__packed` attributes and explicit size checks to maintain cross-architecture consistency.
- **RDMA Memory Pools:** `smbdirect_connection_create_mem_pools()` must be balanced by `smbdirect_connection_destroy_mem_pools()` on all failure paths to prevent memory leaks in the RDMA subsystem.
