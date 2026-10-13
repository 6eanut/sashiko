# Networking Core Subsystem Code Context Guide (Linux v7.3-rc5)

## 1. SKB Control Block and Metadata Lifecycle
- **`skb->cb` (48 bytes):** A layer-local scratchpad defined in `include/linux/skbuff.h`. It is **not** persistent across the entire stack. It is overwritten by netfilter, qdisc, and device drivers.
- **Persistence:** Never store data in `skb->cb` during packet construction to be read later from an `skb->destructor` or asynchronous completion callback. Use `skb_shinfo(skb)->destructor_arg` (and helpers such as `skb_uarg()`) for metadata that must survive until `kfree_skb()`.
- **Bug Pattern:** Never store pointers in `skb->cb` if the packet traverses layers that perform `skb_pull()` or `skb_push()`, as these operations may shift the data or trigger re-allocation.

## 2. Packet Type Constants (`include/uapi/linux/if_packet.h`)
| Constant | Value | Meaning |
| :--- | :--- | :--- |
| `PACKET_HOST` | `0` | Destined for this host |
| `PACKET_BROADCAST` | `1` | Link-layer broadcast |
| `PACKET_MULTICAST` | `2` | Link-layer multicast |
| `PACKET_OTHERHOST` | `3` | Destined for another host (promiscuous mode) |
| `PACKET_OUTGOING` | `4` | Outgoing packet of any type |
| `PACKET_LOOPBACK` | `5` | Multicast/broadcast frame looped back |

## 3. UAPI Embedded Header Alignment and Types
- **Type Definitions:** Use `__le16`, `__le32`, `__u32`, `__u8`, and `__virtio16` for UAPI structures to ensure cross-architecture consistency.
- **Alignment Inheritance:** When a UAPI header struct embeds an existing UAPI struct (e.g., `virtio_net_hdr_v1` -> `virtio_net_hdr_v1_hash` -> `virtio_net_hdr_v1_hash_tunnel`), the outer struct inherits the alignment of its members.
- **Extension Rules:** Placing a 4-byte aligned `__le32` / `__u32` field after a 12-byte header (which has 2-byte alignment) introduces padding holes. When extending headers, ensure new fields do not require wider alignment than the embedded struct provides, or split wider values into matching fields (e.g., two `__le16` members).
- **Validation:** Guard casts with `BUILD_BUG_ON(__alignof__(outer) != __alignof__(inner))`.

## 4. XFRM / IPsec Packet Family Determination (`include/net/xfrm.h`)
- `struct xfrm_state` carries multiple address-family fields (`x->props.family` for outer tunnel family, `x->inner_mode.family` for primary inner family, `x->inner_mode_iaf.family` for dual-stack alternative inner family, and `x->outer_mode.family`). In cross-family tunnels (e.g., IPv6-over-IPv4) and dual-stack SA states, `x->props.family` or `x->inner_mode.family` may not match the actual packet being processed.
- Determine the actual packet family from `skb_dst(skb)->ops->family` (or `skb->protocol`) before invoking family-specific header accessors such as `ip_hdr(skb)` or `ipv6_hdr(skb)`.

## 5. Synchronization, Locking, and Concurrency
- **`rtnl_lock`:** Required for all device configuration changes (e.g., `ndo_stop`, `ndo_open`, `ndo_set_rx_mode`).
- **RCU:** All lookups in global lists (e.g., `fib_lookup`) must be wrapped in `rcu_read_lock()`. Missing `rcu_read_lock()` around `rcu_dereference()` is a critical regression.
- **Atomic Context:** Never perform `GFP_KERNEL` allocations or blocking operations (e.g., `mutex_lock`) inside `ndo_start_xmit` or NAPI poll loops. Use `GFP_ATOMIC`.
- **Atomic Access:** Use `READ_ONCE()` and `WRITE_ONCE()` for shared sysctl variables to prevent compiler reordering and data races.
- **Lock Hierarchy:** When multiple locks are required, follow the established hierarchy (e.g., `rtnl_lock` > `dev_addr_sem` > `mutex` locks).

## 6. Initialization and Teardown
- **Registration:** A `net_device` must be fully initialized (queues, NAPI, stats) before calling `register_netdev()`.
- **Teardown:** Before freeing a device, you must:
  1. Stop the NAPI poll loop (`napi_disable`).
  2. Synchronously cancel all pending timers and work items (`cancel_work_sync`, `del_timer_sync`).
  3. Drain per-CPU packet queues.
- **Error Unwinding:** Ensure all resources (e.g., `dma_buf` bindings, `page_pool` allocations) are released in reverse order of allocation on error paths.

## 7. Error Handling and Boundary Validation
- **`pskb_may_pull()`:** Before accessing any protocol header, you **must** call `pskb_may_pull(skb, len)`. Failure to do so is a primary cause of OOB reads.
- **Headroom:** Always verify `skb_headroom(skb)` against the maximum possible size of headers added by tunneling.
- **Integer Overflows:** Use `check_add_overflow()` or `check_mul_overflow()` when calculating buffer sizes from user-provided lengths.
- **Netlink Validation:** Use `nla_policy` for all netlink attributes. For `NLA_BINARY`, enforce an exact length check.
- **String Termination:** All user-provided strings (e.g., interface names) must be explicitly NUL-terminated before use in `dev_get_by_name()`.
- **Refcounts:** Every `dev_hold()` or `netdev_put()` must have a symmetric path. After `kfree_skb()` or `dev_put()`, set the pointer to `NULL` to prevent UAF.
- **Return Values:** Always check return values of resource allocators; distinguish between `NULL` and `ERR_PTR()` sentinels.
- **Memory Safety:** When copying structures to user-space, ensure the entire buffer is zeroed to prevent kernel stack information leaks.
- **Device Memory:** For `net_devmem_dmabuf_binding`, ensure `percpu_ref` is correctly managed; the binding remains alive until all `net_iovs` are released back to the `page_pool`.

