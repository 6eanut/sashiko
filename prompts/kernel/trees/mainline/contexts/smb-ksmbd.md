# SMBDirect (RDMA) & ksmbd Internals (`fs/smb/smbdirect/`, `fs/smb/server/`)

## SMB Direct (RDMA) Credit Grant Ordering (`fs/smb/smbdirect/`)

In the MS-SMBD protocol, no data transfer message carrying a non-zero `credits_granted` may be sent before the SMB Direct negotiate response is sent (on accept) or received (on connect):
- Two work items on `struct smbdirect_socket` (`fs/smb/smbdirect/socket.h`, `fs/smb/smbdirect/connection.c`) drive credit-granting sends:
  - `sc->recv_io.posted.refill_work` (`smbdirect_connection_recv_io_refill_work()`): Posts receive buffers and queues `sc->idle.immediate_work` on `sc->workqueues.immediate` if receive credits were posted.
  - `sc->idle.immediate_work` (`smbdirect_connection_send_immediate_work()`): Sends an empty data transfer PDU granting credits to the peer.
- **Disabled-work initialization pattern:** `smbdirect_socket_init()` (`fs/smb/smbdirect/socket.h`) initializes `sc->recv_io.posted.refill_work` and `sc->idle.immediate_work` with `__smbdirect_socket_disabled_work` (which fires `WARN_ON_ONCE`) and immediately calls `disable_work_sync()` on both so `queue_work()` calls during early receive buffer setup (`accept.c` / `connect.c`) are no-ops until the real handlers are bound via `INIT_WORK()` (`connection.c`) at the appropriate protocol state transition.
- Any change that removes `disable_work_sync()` in `smbdirect_socket_init()`, reorders `INIT_WORK()` / `enable_work()` relative to negotiation completion, or converts between `work_struct` and `delayed_work` without preserving the disabled state breaks credit grant ordering.
