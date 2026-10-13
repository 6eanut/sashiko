# 64-Bit Network Driver Statistics API Guide (Linux v7.3-rc5)

This guide defines the usage, synchronization, and safety invariants for the `u64_stats_sync` API, located in `include/linux/u64_stats_sync.h`. This API prevents torn reads of 64-bit counters on 32-bit architectures.

## 1. Initialization and Lifecycle
*   **Initialization:** Every `struct u64_stats_sync` instance must be initialized via `u64_stats_init(&stats->syncp)` before any update or read operation.
*   **Lifecycle:** The sync object lifetime must match the statistics container. No dynamic allocation or reference counting is required.

## 2. Writer Rules (Producer)
Writers must ensure mutual exclusion and prevent preemption to avoid deadlocks on 32-bit architectures.

*   **Exclusivity:** Writers must be mutually exclusive (e.g., per-CPU or single-NAPI-poll instance).
*   **Preemption:** Writers must run with preemption disabled (e.g., `local_bh_disable()` or `spin_lock_bh()`). If a writer is preempted by a reader on the same CPU, the reader will spin indefinitely on 32-bit kernels.
*   **Interrupt Context:** If a hardirq handler can update or read counters sharing the same `syncp`, writers must use `u64_stats_update_begin_irqsave(&stats->syncp)` and `u64_stats_update_end_irqrestore(&stats->syncp, flags)`.
*   **Ordering:** Statistics must be updated between `u64_stats_update_begin()` and `u64_stats_update_end()`.

## 3. Reader Rules (Consumer)
Readers perform non-blocking, lockless reads using a retry loop.

*   **Retry Loop Invariant:**
    ```c
    do {
        start = u64_stats_fetch_begin(&stats->syncp);
        bytes = u64_stats_read(&stats->bytes);
        pkts  = u64_stats_read(&stats->packets);
    } while (u64_stats_fetch_retry(&stats->syncp, start));
    ```
*   **Accumulation Rule:** The body of the loop must assign values to local temporary variables. Accumulation (`+=`) into output totals must occur **only after** the retry loop succeeds. Performing accumulation inside the loop results in double-counting during retries.

## 4. Review Invariants and Bug Patterns
Automated review tools and maintainers must enforce the following:

1.  **The "Double-Count" Trap:** Flag any `+=` or `-=` operations performed inside the `u64_stats_fetch_begin` / `u64_stats_fetch_retry` loop.
2.  **Preemption/Locking Mismatch:** Any `u64_stats_update_begin` call not preceded by a `spin_lock_bh` or running in a guaranteed non-preemptible context (e.g., `local_bh_disable`) is a high-priority bug.
3.  **IRQ Safety:** If a driver's `ndo_get_stats64` path can be interrupted by a hardirq that also updates stats, the writer must use the `_irqsave` variants.
4.  **Torn Read Awareness:** On 32-bit systems, `u64_stats_read` is not atomic. Reading a 64-bit counter without the `u64_stats_sync` wrapper is a critical data-integrity bug.
5.  **Initialization Order:** `u64_stats_init` must be called during device probe or queue allocation, before the interface is brought `UP` and statistics are exposed to the stack.

## 5. Concurrency and Memory Ordering
*   **Lock Hierarchy:** When updating stats, the `u64_stats_sync` primitive does not provide memory barriers for the data itself; it only provides the sequence counter. Ensure that the actual counter update is visible to other CPUs via appropriate compiler barriers or atomic operations if the architecture requires it.
*   **Reader Serialization:** Readers are not serialized against each other. They are only protected against torn reads from writers.
*   **False-Positive Guards:**
    *   **64-bit Architectures:** The API compiles to no-ops on 64-bit architectures. Missing locks in code paths strictly restricted to 64-bit hardware are not bugs, though the API should be used for cross-architecture consistency.
    *   **Per-CPU Data:** If statistics are strictly per-CPU and the writer is in a `preempt_disable()` or `local_bh_disable()` block, the writer is considered "exclusive" even without a spinlock.

## 6. API Reference Summary
*   `u64_stats_init(struct u64_stats_sync *syncp)`: Initializes the sync object.
*   `u64_stats_update_begin(struct u64_stats_sync *syncp)`: Marks the start of a write section.
*   `u64_stats_update_end(struct u64_stats_sync *syncp)`: Marks the end of a write section.
*   `u64_stats_read(const u64_stats_t *p)`: Safely reads a 64-bit value.
*   `u64_stats_fetch_begin(const struct u64_stats_sync *syncp)`: Returns the sequence count for the retry loop.
*   `u64_stats_fetch_retry(const struct u64_stats_sync *syncp, unsigned int start)`: Checks if a write occurred during the read.
