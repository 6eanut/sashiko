# BPF Map Special Field Lifecycle API Guide (Linux v7.3-rc5)

This guide defines the lifecycle management, synchronization, and safety invariants for BPF map special fields (e.g., `BPF_TIMER`, `BPF_WORKQUEUE`, `BPF_KPTR`, `BPF_LIST_HEAD`, `BPF_RB_ROOT`, `BPF_UPTR`) as implemented in `include/linux/bpf.h`.

---

## 1. Core API Lifecycle Helpers (`include/linux/bpf.h`)

*   **`check_and_init_map_value(struct bpf_map *map, void *dst)`**: Zeroes/reinitializes the special BTF fields skipped by `copy_map_value()`. Must be called when initializing a newly allocated element or zeroing special fields in a buffer copied to userspace.
*   **`bpf_obj_cancel_fields(const struct btf_record *rec, void *obj)`**: Cancels active `BPF_TIMER`, `BPF_WORKQUEUE`, and `BPF_TASK_WORK` fields without freeing other special fields (`BPF_KPTR`, `BPF_LIST_HEAD`, `BPF_RB_ROOT`, `BPF_UPTR`).
*   **`bpf_obj_free_fields(const struct btf_record *rec, void *obj)`**: Performs full destruction of all special fields in `rec`. Callers must ensure all field destructors for the map's permitted `field_mask` are safe in the calling context (e.g., non-NMI vs NMI).

---

## 2. Calling Context & Synchronization
*   **Atomic vs. Sleepable Contexts:** `bpf_obj_free_fields` may trigger operations that require sleeping (e.g., `kfree_rcu`). **Rule:** Do not call `bpf_obj_free_fields` in NMI or hard-IRQ contexts.
*   **Locking:** When updating map values, use `copy_map_value_locked` to ensure the copy is atomic relative to concurrent readers.
*   **Memory Allocation:** Use `bpf_map_area_alloc` or `bpf_map_area_mmapable_alloc` for map backing stores to ensure proper handling of `__GFP_NORETRY` and `__GFP_RETRY_MAYFAIL`.
*   **Memory Ordering:** When publishing a newly initialized object to a shared map, ensure appropriate memory barriers (e.g., `smp_wmb()`) are used if the map implementation does not provide implicit ordering.

---

## 3. High-Frequency Bug Patterns & Review Invariants

### A. The "Stale Field" Pattern
**Bug:** Recycling map elements without clearing special fields.
**Invariant:** When a map element is reused, `check_and_init_map_value` must be invoked to reset the `btf_record` state. Failure to do so leads to use-after-free if the old element contained active timers or workqueues.

### B. The "NMI/Atomic" Pattern
**Bug:** Calling `bpf_obj_free_fields` inside an NMI handler.
**Invariant:** Reviewers must verify that the destruction path is not reachable from NMI. If the object must be freed from an atomic context, defer the destruction to a workqueue or use `bpf_mem_free`.

### C. The "Precision/NULL" Pattern
**Bug:** Missing `NULL` checks on `BPF_KPTR` lookups.
**Invariant:** Any code path retrieving a `BPF_KPTR` or `BPF_UPTR` must treat the result as potentially `NULL`. The verifier requires explicit `NULL` checks.

### D. The "Teardown Order" Pattern
**Bug:** Freeing an object before cancelling its asynchronous tasks.
**Invariant:** Always invoke `bpf_obj_cancel_fields` before `bpf_obj_free_fields` to ensure that pending timers or workqueues are quiesced before the underlying memory is reclaimed.

---

## 4. Summary Checklist for Reviewers
1.  **Is `check_and_init_map_value` called?** Required for all new map elements with `btf_record`.
2.  **Is the context safe?** Ensure `bpf_obj_free_fields` is not called in NMI/hard-IRQ.
3.  **Are pointers validated?** Ensure all `BPF_KPTR` accesses are guarded by `NULL` checks.
4.  **Is the teardown order correct?** Cancel tasks (`bpf_obj_cancel_fields`) *before* freeing the object (`bpf_obj_free_fields`).
5.  **Is the object unlinked?** Ensure the object is removed from all maps/lists before calling `bpf_obj_free_fields`.
6.  **Is the allocation context correct?** Verify `bpf_map_area_alloc` is used for large map backing stores to respect `__GFP_NORETRY` constraints.
7.  **Are recursive structures handled?** Ensure that object destruction logic prevents unbounded recursion when following pointers to other objects.
8.  **Is the state update atomic?** Ensure that map updates are performed only after all required resources have been successfully allocated to avoid dangling pointers.
