# BPF Subsystem Code Context Guide (Linux v7.3-rc5)

## 1. BTF Special Fields in BPF Maps (`kernel/bpf/syscall.c`)

### Allowed Special Fields by Map Type (`map_check_btf()`)

BPF map values may contain special BTF-described kernel objects (`map->record` parsed by `btf_parse_fields()`). `map_check_btf()` restricts which `BPF_MAP_TYPE_*` support each field flag:

| Field Mask | Permitted `BPF_MAP_TYPE_*` |
|---|---|
| `BPF_SPIN_LOCK`, `BPF_RES_SPIN_LOCK` | `HASH`, `RHASH`, `ARRAY`, `CGROUP_STORAGE`, `SK_STORAGE`, `INODE_STORAGE`, `TASK_STORAGE`, `CGRP_STORAGE` |
| `BPF_TIMER`, `BPF_WORKQUEUE`, `BPF_TASK_WORK` | `HASH`, `RHASH`, `LRU_HASH`, `ARRAY` |
| `BPF_KPTR_UNREF`, `BPF_KPTR_REF`, `BPF_KPTR_PERCPU`, `BPF_REFCOUNT` | `HASH`, `RHASH`, `PERCPU_HASH`, `LRU_HASH`, `LRU_PERCPU_HASH`, `ARRAY`, `PERCPU_ARRAY`, `SK_STORAGE`, `INODE_STORAGE`, `TASK_STORAGE`, `CGRP_STORAGE` |
| `BPF_UPTR` | `TASK_STORAGE` |
| `BPF_LIST_HEAD`, `BPF_RB_ROOT` | `HASH`, `LRU_HASH`, `ARRAY` |

*Note: Per-CPU variants exclude `BPF_SPIN_LOCK`, `BPF_TIMER`, `BPF_LIST_HEAD`, and `BPF_RB_ROOT`.*

### Map Lookup, Update, and Release Invariants
- **Lookup (kernel-to-userspace copy):** Special fields copied into a userspace-visible buffer must be zeroed/reinitialized via `check_and_init_map_value()` before returning to userspace to prevent leakage of kernel pointers or lock states.
- **Update/Deletion:** Trace ownership of every permitted special field from allocation through map update, deletion, and element recycling (e.g., `bpf_mem_alloc` / per-CPU freelists).
- **Destruction:** Ensure correct invocation of `bpf_obj_cancel_fields()` vs `bpf_obj_free_fields()`. Verify if the destructor can execute in NMI context.

## 2. Synchronization, Memory, and Lifecycle

*   **Lock Hierarchy:**
    *   `bpf_lru_list->lock` must be held for list manipulations.
    *   `bpf_lru_locallist->lock` protects per-CPU free lists.
    *   **Invariant:** Never acquire `bpf_lru_list->lock` while holding `bpf_lru_locallist->lock`.
*   **Atomic Context:** BPF programs in `BPF_PROG_TYPE_TRACING` or `BPF_PROG_TYPE_KPROBE` execute in atomic contexts. Use `GFP_ATOMIC` or `bpf_global_ma` for allocations.
*   **Object Lifecycle:** Objects transition through `bpf_prog_alloc()` -> `bpf_prog_load()` -> `bpf_prog_free()`. Destruction is deferred via `bpf_prog_put()` triggering `bpf_prog_free_deferred()` (RCU-based).
*   **LRU Management:** `bpf_lru_destroy()` must quiesce all per-CPU lists before freeing the `bpf_lru` structure.

## 3. Error Handling and Validation

*   **Pointer Validation:** Use `IS_ERR()` for pointer returns. Always check `IS_ERR_OR_NULL()` for syscall-layer returns.
*   **Arithmetic Safety:** Use `check_mul_overflow()` or `size_mul()` for map size calculations (e.g., `max_entries * elem_size`) to prevent out-of-bounds access.
*   **BTF Validation:** `btf_parse_fields()` must validate that special field offsets do not overlap and remain within the map value size.
*   **Sign-Extension:** Ensure 32-bit keys are zero-extended to 64-bit before comparison to prevent sign-extension bypasses.

## 4. Critical False-Positive Guards

*   **Guard 1 (LRU):** If modifying `bpf_lru_node`, verify `pending_free` is checked in `bpf_lru_push_free()` to prevent double-deletion.
*   **Guard 2 (Recursion):** `bpf_obj_free_fields()` triggers deep recursion for nested `BPF_LIST_HEAD` or `BPF_RB_ROOT`. The verifier enforces a maximum depth; ensure patches do not bypass this limit.
*   **Guard 3 (Instruction Access):** Any arithmetic on `insn->off` or `insn->imm` must be validated against `BPF_MAXINSNS` to prevent out-of-bounds instruction access.
*   **Guard 4 (Verifier Precision):** When comparing states, ensure stack frame numbers are included in the equivalence check to prevent incorrect loop termination.

## 5. Architectural Invariants (v7.3-rc5)

*   **Range Tracking:** Use `cnum_t` helpers (`FN(from_urange)`, `FN(intersect)`) for range-based verifier logic. Always check `urange_overflow` or `srange_overflow` before assuming a range is valid.
*   **Memory Allocation:** `bpf_global_ma` is the preferred allocator for BPF objects. Ensure `bpf_global_ma_set` is checked if the allocator state is dynamic.
*   **JIT Mapping:** `insn_to_jit_off` must be populated for every xlated instruction. The `jited_off` must point to the end of the instruction to ensure correct `linfo` mapping.
*   **Callback Context:** When verifying subprograms, the verifier must account for the `sleepable` attribute of the caller. A non-sleepable program cannot invoke a sleepable helper.
*   **Resource Unwinding:** Every `bpf_prog_alloc` must have a corresponding `bpf_prog_free` path. Use `bpf_prog_put` to trigger deferred cleanup if the object is shared.
*   **Interrupt Safety:** `rqspinlock_t` is used in `bpf_lru_list`. Ensure that any code path acquiring this lock is aware of the potential for interrupt-context reentrancy if the lock is shared with IRQ handlers.
