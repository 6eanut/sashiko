# 64-Bit Network Driver Statistics (`include/linux/u64_stats_sync.h`)

## `u64_stats_sync` Writer and Reader Rules

`struct u64_stats_sync` prevents torn 64-bit counter reads on 32-bit architectures using a seqcount (and compiles to a no-op on 64-bit architectures):
- **Initialization:** Initialize `struct u64_stats_sync` with `u64_stats_init(&stats->syncp)` before use.
- **Writer exclusivity and preemption:**
  - Writers updating counters protected by a given `syncp` (`u64_stats_update_begin(&stats->syncp)` / `u64_stats_update_end(&stats->syncp)`) must be mutually exclusive (e.g., per-CPU or single-NAPI-poll instance) and must run with preemption disabled (such as NAPI/softirq/BH-disabled context). If a writer can be preempted by a reader on the same CPU, the reader will spin forever on 32-bit kernels.
  - When a hardirq handler can also update or read counters sharing the same `syncp`, writers must use `u64_stats_update_begin_irqsave(&stats->syncp)` and `u64_stats_update_end_irqrestore(&stats->syncp, flags)`.
- **Idempotent reader retry loop:**
  ```c
  do {
      start = u64_stats_fetch_begin(&stats->syncp);
      bytes = u64_stats_read(&stats->bytes);
      pkts  = u64_stats_read(&stats->packets);
  } while (u64_stats_fetch_retry(&stats->syncp, start));
  ```
  The body of the `do { ... } while (u64_stats_fetch_retry(...))` loop must assign into local temporary variables and only accumulate (`+=`) into output totals *after* the retry loop succeeds. Using `+=` directly inside the retry loop double-counts values whenever a retry occurs.
