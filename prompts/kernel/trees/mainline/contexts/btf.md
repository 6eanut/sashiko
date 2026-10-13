# BTF Special Fields in BPF Maps (`kernel/bpf/syscall.c`)

## Allowed Special Fields by Map Type (`map_check_btf()`)

BPF map values may contain special BTF-described kernel objects (`map->record` parsed by `btf_parse_fields()`). `map_check_btf()` (`kernel/bpf/syscall.c`) restricts which `BPF_MAP_TYPE_*` support each field flag:

| Field Mask | Permitted `BPF_MAP_TYPE_*` |
|---|---|
| `BPF_SPIN_LOCK`, `BPF_RES_SPIN_LOCK` | `HASH`, `RHASH`, `ARRAY`, `CGROUP_STORAGE`, `SK_STORAGE`, `INODE_STORAGE`, `TASK_STORAGE`, `CGRP_STORAGE` |
| `BPF_TIMER`, `BPF_WORKQUEUE`, `BPF_TASK_WORK` | `HASH`, `RHASH`, `LRU_HASH`, `ARRAY` |
| `BPF_KPTR_UNREF`, `BPF_KPTR_REF`, `BPF_KPTR_PERCPU`, `BPF_REFCOUNT` | `HASH`, `RHASH`, `PERCPU_HASH`, `LRU_HASH`, `LRU_PERCPU_HASH`, `ARRAY`, `PERCPU_ARRAY`, `SK_STORAGE`, `INODE_STORAGE`, `TASK_STORAGE`, `CGRP_STORAGE` |
| `BPF_UPTR` | `TASK_STORAGE` |
| `BPF_LIST_HEAD`, `BPF_RB_ROOT` | `HASH`, `LRU_HASH`, `ARRAY` |

Always check `map_check_btf()` for the exact map type under review (especially per-CPU variants, which exclude `BPF_SPIN_LOCK`, `BPF_TIMER`, `BPF_LIST_HEAD`, and `BPF_RB_ROOT`).

## Map Lookup, Update, and Release Invariants

- **Lookup (kernel-to-userspace copy):** Special fields copied into a userspace-visible buffer must be zeroed/reinitialized via `check_and_init_map_value()` before returning to userspace so kernel pointers or lock states are not leaked.
- **Update, deletion, and deferred freeing:** Trace ownership of every permitted special field from allocation through map update, deletion, element recycling (e.g., `bpf_mem_alloc` / per-CPU freelists), and final destructor execution (`bpf_obj_cancel_fields()` vs `bpf_obj_free_fields()`), including whether the destructor can run in NMI context.
