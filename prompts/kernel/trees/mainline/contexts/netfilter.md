# Netfilter Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides authoritative technical invariants for the Netfilter subsystem. Developers and automated reviewers must adhere to these rules to prevent common vulnerabilities.

---

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **Hook Management (`net/netfilter/core.c`):** Netfilter hooks are stored in `struct nf_hook_entries` arrays, protected by RCU.
    *   **Invariant:** Hook unregistration must never fail. When a hook is removed, the subsystem replaces the old array with a new one containing the remaining hooks. The old array is freed after an RCU grace period.
*   **Conntrack Objects (`struct nf_conn`):**
    *   **Ownership:** The reference count (`ct_general.use`) is managed via `nf_conntrack_get()` and `nf_conntrack_put()`.
    *   **Lifecycle:** An object is typically referenced by the hash table (1), the `skb` (1), and potentially by `master` connections.
    *   **Extensions:** `struct nf_ct_ext` allows attaching data to conntrack entries. Reallocating extensions on a confirmed entry is a high-risk operation; it must be protected against concurrent lockless readers.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Conntrack Locking (`nf_conntrack_core.c`):**
    *   **Hierarchy:** `nf_conntrack_locks` (array of spinlocks) protects hash chains. `nf_conntrack_locks_all` is a global flag used for resizing or bulk cleanup.
    *   **Rule:** When acquiring multiple locks, follow the established order. If `nf_conntrack_locks_all` is set, readers must wait.
    *   **RCU:** All lookups in the conntrack hash table must be wrapped in `rcu_read_lock()`. Accessing `nf_conn` members after `rcu_dereference` without holding the lock or a reference is a use-after-free vulnerability.
*   **Atomic Context:** Netfilter hooks often run in softirq or atomic contexts. Never call `kmalloc(..., GFP_KERNEL)` or any function that can sleep (e.g., `mutex_lock`) within a hook function. Use `GFP_ATOMIC` for allocations.

## 3. Initialization, Registration & Teardown Ordering

*   **Premature Publication:** Never expose a structure to the global list or hash table until it is fully initialized.
    *   **Bug Pattern:** Initializing a structure, adding it to a list, and *then* setting its internal fields leads to race conditions where concurrent readers see uninitialized memory.
*   **Error Unwind:** Every `nf_register_net_hook` must have a corresponding `nf_unregister_net_hook` on all error paths. Use the "goto cleanup" pattern to ensure symmetry.

## 4. Error Handling, Resource Recovery & Boundary Validation

*   **Pointer Sentinels:** Always distinguish between `NULL` (not found) and `ERR_PTR` (error).
    *   **Rule:** Use `IS_ERR()` to check return values from lookup functions.
*   **Input Validation:**
    *   **SKB Headers:** Before accessing protocol headers (TCP/UDP/IP), always call `pskb_may_pull()` to ensure the data is in the linear buffer. Failure to do so results in out-of-bounds reads.
    *   **Netlink Attributes:** Validate all user-provided lengths and types in netlink handlers. Use `nla_parse_nested()` and check return codes strictly.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **Use-After-Free (UAF):**
    *   *Pattern:* Freeing an object while an RCU reader is still accessing it.
    *   *Guard:* Ensure `call_rcu()` or `synchronize_rcu()` is used before freeing objects that were reachable via RCU.
*   **Unbalanced Refcounts:**
    *   *Pattern:* Failing to decrement the refcount on an error path after a successful `nf_conntrack_get()`.
    *   *Guard:* Audit every `return` statement in functions that increment refcounts.
*   **Integer Overflows:**
    *   *Pattern:* Calculating buffer sizes for netlink dumps without checking for overflow.
    *   *Guard:* Use `size_add()` or `check_add_overflow()` when calculating memory requirements for netlink attributes.
*   **Deadlocks:**
    *   *Pattern:* Attempting to acquire a mutex while holding a spinlock.
    *   *Guard:* Use `lockdep` to verify that no sleepable locks are acquired in the hook path.

## 6. Domain-Specific Invariants (Networking)

*   **Packet Buffer Management:**
    *   When stripping headers, validate the remaining length against the minimum header size of the next protocol layer.
    *   **Checksums:** When modifying packets (NAT), ensure the checksum is updated correctly. Use `nf_checksum_partial()` or equivalent helpers.
*   **State Machine Invariants:**
    *   The `nf_conn` status bitmask must be updated atomically. Use `test_and_set_bit()` or `set_bit()` to avoid races during status transitions (e.g., `IPS_CONFIRMED`).

---

### Critical Checklist for Patch Review
1.  **Is `rcu_read_lock()` held** for all hash table lookups?
2.  **Is `pskb_may_pull()` called** before accessing packet headers?
3.  **Are all error paths** cleaning up allocated resources (hooks, memory, refcounts)?
4.  **Is the object fully initialized** before being linked into a global list?
5.  **Are `IS_ERR()` checks present** for all lookup functions?
6.  **Are there any sleeping allocations** (`GFP_KERNEL`) in the hook path?
