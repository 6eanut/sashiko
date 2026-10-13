# Linux v7.3-rc5 Network Subsystem Code Context Guide

This guide provides authoritative technical invariants for the Linux networking subsystem (v7.3-rc5), focusing on `drivers/net/ethernet/`, `drivers/net/phy/`, and `include/linux/netdevice.h`.

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

*   **`net_device` Lifecycle:** The `net_device` structure must be registered via `register_netdev()` only after all internal state (queues, NAPI, private data) is fully initialized.
*   **Reference Counting:** Use `dev_hold()` and `dev_put()` to manage references. Never access a `net_device` after `unregister_netdev()` has been called unless a valid reference is held.
*   **NAPI Lifecycle:** NAPI structures (`struct napi_struct`) must be initialized via `netif_napi_add()` (or variants) before `napi_enable()`. `napi_disable()` must be called before `netif_napi_del()` to ensure no pending poll cycles occur during teardown.
*   **PHY Objects:** `struct phy_device` is managed by the PHY library. Drivers must not manually free `phy_device` structures; use `phy_disconnect()` or `phy_detach()` during teardown.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Atomic vs. Sleepable Contexts:**
    *   `ndo_start_xmit()` is called in atomic context (BH disabled). Never perform memory allocations with `GFP_KERNEL` or blocking operations (e.g., `mutex_lock`, `msleep`). Use `GFP_ATOMIC` for allocations.
    *   `ndo_open()` and `ndo_stop()` are called in process context and may sleep.
*   **Locking Hierarchy:**
    *   The `rtnl_lock` is the global serialization primitive for device configuration.
    *   For per-queue synchronization, use `netif_tx_lock()` or `spin_lock_bh()` on queue-specific locks.
    *   **Deadlock Prevention:** Never hold a spinlock while calling functions that may sleep.
*   **RCU Usage:** Use `rcu_read_lock()` when accessing `net_device` pointers or configuration lists that may be modified concurrently. Use `rcu_dereference()` to access protected pointers.

## 3. Initialization, Registration & Teardown Ordering

*   **Premature Publication:** A driver must not call `register_netdev()` until the hardware is fully ready to process interrupts and transmit packets.
*   **Symmetric Unwind:** Every resource allocated in `ndo_open` (e.g., `request_irq`, `dma_alloc_coherent`) must have a corresponding cleanup in `ndo_stop`.
*   **Error Path Invariants:**
    *   Always check for `IS_ERR()` on pointers returned by kernel APIs.
    *   Ensure `of_node_put()` is called on error paths if `of_find_node_by_name()` or similar was used.
    *   Guard cleanup operations with the same conditions used during initialization to prevent double-frees or NULL pointer dereferences.

## 4. Packet Buffer Management & Header Validation

*   **Linearity:** Before accessing transport headers, verify the buffer is linear using `pskb_may_pull()`.
*   **Checksums:** For virtual devices, ensure `skb->csum` does not include headers that the stack expects to be excluded.
*   **Alignment:** DMA buffers must be aligned according to hardware requirements. Failure to account for alignment in buffer sizing leads to memory corruption.

## 5. High-Frequency Bug Patterns & False-Positive Guards

*   **Unbalanced Refcounts:** Ensure every `dev_hold` has a corresponding `dev_put` on all exit paths.
*   **Uncancelled Work:** Use `cancel_work_sync()` or `timer_delete_sync()` before freeing the structure containing the work/timer.
*   **Register Access:**
    *   Always mask reserved/status bits when reading hardware registers.
    *   When using indirect register access, ensure the page/bank selection is atomic or protected by a lock to prevent race conditions between concurrent configuration threads.
*   **Integer Overflows:** Validate all length fields from hardware or user-space against the actual buffer size before using them as array indices or allocation sizes.
*   **False-Positive Guard:** If a `netif_napi_add` is called, ensure the corresponding `napi_disable` is present in the `ndo_stop` path, even if the device failed to open.

## 6. Hardware State Machines & DMA

*   **DMA Mapping:** Always unmap DMA buffers in the error path if the probe fails after mapping.
*   **Hardware Errata:** Implement known hardware workarounds within the driver's `probe` or `open` sequence, guarded by device-specific flags.
*   **Polling Loops:** Never loop infinitely on hardware status registers. Always implement a timeout mechanism (e.g., `readl_poll_timeout`) to prevent soft lockups.
