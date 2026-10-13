# Networking Core API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for the Linux networking core APIs. Adherence to these invariants is mandatory for all kernel modules interacting with `sk_buff`, `sock`, `dst_entry`, and `SNMP` statistics.

## 1. SKB Bounds, Linearization, and Unsharing (`include/linux/skbuff.h`)

- **`skb_put()`, `skb_push()`, and `skb_pull()`:**
  - `skb_put(skb, len)` extends `skb->tail` and calls `BUG()` (`skb_over_panic()`) if `skb->tail > skb->end`.
  - `skb_push(skb, len)` prepends to `skb->data` and calls `BUG()` (`skb_under_panic()`) if `skb->data < skb->head`.
  - `skb_pull(skb, len)` returns `NULL` if `len > skb->len`, and calls `BUG()` in `__skb_pull()` if `skb->len < skb->data_len` (linear region exhausted). Always validate untrusted lengths before calling these helpers.
- **Header linearization (`pskb_may_pull(skb, len)`):** Packet headers may span paged fragments. Call `pskb_may_pull(skb, len)` before dereferencing protocol headers in `skb->data` (and re-fetch header pointers like `ip_hdr(skb)` after `pskb_may_pull()`, which may reallocate `skb->head`).
- **Shared and cloned SKBs (`skb_shared()`, `skb_cloned()`, `skb_unshare()`):**
  - `skb_shared(skb)` checks `refcount_read(&skb->users) != 1`; `skb_cloned(skb)` checks whether the data buffer is shared with another `sk_buff`.
  - `skb_unshare(skb, gfp)` returns an exclusive SKB, freeing the input `skb` unconditionally when cloned (via `consume_skb()` on success or `kfree_skb()` on allocation failure). Always assign and NULL-check the returned pointer; never access the old pointer after `skb_unshare()`.
- **Buffer handoff ownership:** Once an SKB is queued, enqueued, or passed to a protocol handler or `NF_HOOK()` / `NF_HOOK_COND()`, the caller loses ownership and must not dereference the SKB afterward (verdicts `NF_ACCEPT`, `NF_DROP`, `NF_STOLEN`, and `NF_QUEUE` all consume or free the SKB).

## 2. Socket Locking vs Destruction and Reference Counting (`include/net/sock.h`)

- **`release_sock(sk)` vs `sock_release(sock)`:**
  - `release_sock(sk)` releases the socket lock acquired by `lock_sock(sk)` (processing the backlog queue and waking waiters); `sk` remains alive.
  - `sock_release(sock)` closes and destroys the `struct socket` and its underlying inode.
- **`sock_hold(sk)` and `sock_put(sk)`:** Manage `sk->sk_refcnt`. Any pointer to `struct sock` stored beyond the lifetime of an active file descriptor or RCU section must hold a reference via `sock_hold()` and release it via `sock_put()`.

## 3. Routing `dst_entry` RCU Protection (`include/net/route.h`, `include/net/dst.h`)

- `ip_route_input_noref()` and `skb_dst_set_noref()` attach an RCU-protected `dst_entry` to `skb` without taking a reference (`skb_dst_set_noref()` warns if neither `rcu_read_lock()` nor `rcu_read_lock_bh()` is held).
- Note that `ip_route_input_noref()` takes its own internal `rcu_read_lock()` / `rcu_read_unlock()`; if the noref `dst` must survive past the call, the caller must hold an outer `rcu_read_lock()` and upgrade the entry to a refcounted reference via `skb_dst_force(skb)` before `rcu_read_unlock()` (checking `!skb_dst(skb)` in case `skb_dst_force()` failed).

## 4. Per-CPU Network Statistics (`include/net/snmp.h`)

- `SNMP_INC_STATS()` / `SNMP_ADD_STATS()`: Update single-word (`unsigned long`) per-CPU counters via `this_cpu_inc()` / `this_cpu_add()`.
- `SNMP_ADD_STATS64()` / `SNMP_UPD_PO_STATS64()`: Wrap 64-bit updates in `local_bh_disable()` / `local_bh_enable()` and use `u64_stats` seqcounts on 32-bit architectures to prevent torn reads.
- `__SNMP_ADD_STATS64()` / `__SNMP_UPD_PO_STATS64()`: Omit `local_bh_disable()` and may only be called when bottom halves are already disabled.

## 5. High-Frequency Bug Patterns & Review Invariants

- **Memory Allocation:** Never use `GFP_KERNEL` in atomic context (e.g., inside `spin_lock`, `rcu_read_lock`, or hard IRQ). Use `GFP_ATOMIC` for allocations within these contexts.
- **Integer Underflow:** When calculating lengths for `skb_pull` or `skb_put`, ensure the arithmetic is performed on `size_t` or `unsigned int` and checked against `skb->len` to prevent underflow.
- **TOCTOU Races:** Do not sample `skb->len` or `skb->data` and then perform multiple operations without re-validating if the buffer could have been modified or reallocated (e.g., after `pskb_may_pull`).
- **Use-After-Free on Consumed SKBs:** Never dereference or free an `skb` after passing it to a consuming helper (`kfree_skb()`, `consume_skb()`, `dev_queue_xmit()`, `netif_rx()`, `napi_gro_receive()`), and clear any persistent struct field holding an `skb` pointer when releasing it so later teardown paths do not double-free it.
- **Premature Exposure:** Never register a device or object with the global subsystem until all internal state, locks, and buffers are fully initialized.
- **Idempotency:** Ensure that cleanup routines (e.g., `kfree_skb`) are idempotent or guarded by checks to prevent double-frees on error paths.
- **NAPI/Polling:** Always ensure `napi_complete_done()` is called with the correct budget to prevent infinite polling loops or premature re-scheduling.
- **Lock Hierarchy:** Always acquire `rtnl_lock` before modifying device state (e.g., `dev_change_flags`, `register_netdevice`). Never hold `rtnl_lock` while performing blocking I/O or waiting for hardware.
- **Reference Ownership:** When receiving an object from a lookup, verify if the API returns a reference-counted object (requiring `_put`) or a transient pointer (valid only under RCU).
- **Error Propagation:** Always check return values of `skb_copy_expand` or `pskb_expand_head` as they may return `NULL` on failure, requiring the original SKB to be freed.
- **Context Awareness:** When transitioning from non-preemptible to preemptible contexts, ensure all local state (e.g., `local_bh_disable`) is balanced to prevent kernel panics.
- **Hardware Synchronization:** Ensure all hardware-visible state updates (e.g., doorbell rings) are flushed before releasing NAPI ownership or returning from a poll cycle.
- **Resource Cleanup:** On probe failure, ensure all successfully registered resources are explicitly unregistered in reverse order of initialization to prevent dangling references.
- **State Consistency:** When applying dynamic configuration updates, enforce the same uniqueness and validity constraints as the initial creation sequence to prevent state machine corruption.
