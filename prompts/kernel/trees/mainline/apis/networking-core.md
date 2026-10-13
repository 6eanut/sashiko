# SKB, Socket, Netfilter, Routing, and SNMP API (`include/linux/skbuff.h`, `include/net/sock.h`, `include/net/route.h`, `include/net/snmp.h`)

## SKB Bounds, Linearization, and Unsharing (`include/linux/skbuff.h`)

- **`skb_put()`, `skb_push()`, and `skb_pull()`:**
  - `skb_put(skb, len)` extends `skb->tail` and calls `BUG()` (`skb_over_panic()`) if `skb->tail > skb->end`.
  - `skb_push(skb, len)` prepends to `skb->data` and calls `BUG()` (`skb_under_panic()`) if `skb->data < skb->head`.
  - `skb_pull(skb, len)` returns `NULL` if `len > skb->len`, and calls `BUG()` in `__skb_pull()` if `skb->len < skb->data_len` (linear region exhausted). Always validate untrusted lengths before calling these helpers.
- **Header linearization (`pskb_may_pull(skb, len)`):** Packet headers may span paged fragments. Call `pskb_may_pull(skb, len)` before dereferencing protocol headers in `skb->data` (and re-fetch header pointers like `ip_hdr(skb)` after `pskb_may_pull()`, which may reallocate `skb->head`).
- **Shared and cloned SKBs (`skb_shared()`, `skb_cloned()`, `skb_unshare()`):**
  - `skb_shared(skb)` checks `refcount_read(&skb->users) != 1`; `skb_cloned(skb)` checks whether the data buffer is shared with another `sk_buff`.
  - `skb_unshare(skb, gfp)` returns an exclusive SKB, freeing the input `skb` unconditionally when cloned (via `consume_skb()` on success or `kfree_skb()` on allocation failure). Always assign and NULL-check the returned pointer; never access the old pointer after `skb_unshare()`.
- **Buffer handoff ownership:** Once an SKB is queued, enqueued, or passed to a protocol handler or `NF_HOOK()` / `NF_HOOK_COND()`, the caller loses ownership and must not dereference the SKB afterward (verdicts `NF_ACCEPT`, `NF_DROP`, `NF_STOLEN`, and `NF_QUEUE` all consume or free the SKB).

## Socket Locking vs Destruction and Reference Counting (`include/net/sock.h`)

- **`release_sock(sk)` vs `sock_release(sock)`:**
  - `release_sock(sk)` releases the socket lock acquired by `lock_sock(sk)` (processing the backlog queue and waking waiters); `sk` remains alive.
  - `sock_release(sock)` closes and destroys the `struct socket` and its underlying inode.
- **`sock_hold(sk)` and `sock_put(sk)`:** Manage `sk->sk_refcnt`. Any pointer to `struct sock` stored beyond the lifetime of an active file descriptor or RCU section must hold a reference via `sock_hold()` and release it via `sock_put()`.

## Routing `dst_entry` RCU Protection (`include/net/route.h`, `include/net/dst.h`)

- `ip_route_input_noref()` and `skb_dst_set_noref()` attach an RCU-protected `dst_entry` to `skb` without taking a reference (`skb_dst_set_noref()` warns if neither `rcu_read_lock()` nor `rcu_read_lock_bh()` is held).
- Note that `ip_route_input_noref()` takes its own internal `rcu_read_lock()` / `rcu_read_unlock()`; if the noref `dst` must survive past the call, the caller must hold an outer `rcu_read_lock()` and upgrade the entry to a refcounted reference via `skb_dst_force(skb)` before `rcu_read_unlock()` (checking `!skb_dst(skb)` in case `skb_dst_force()` failed).

## Per-CPU Network Statistics (`include/net/snmp.h`)

- `SNMP_INC_STATS()` / `SNMP_ADD_STATS()`: Update single-word (`unsigned long`) per-CPU counters via `this_cpu_inc()` / `this_cpu_add()`.
- `SNMP_ADD_STATS64()` / `SNMP_UPD_PO_STATS64()`: Wrap 64-bit updates in `local_bh_disable()` / `local_bh_enable()` and use `u64_stats` seqcounts on 32-bit architectures to prevent torn reads.
- `__SNMP_ADD_STATS64()` / `__SNMP_UPD_PO_STATS64()`: Omit `local_bh_disable()` and may only be called when bottom halves are already disabled.
