# BPF Map Special Field Lifecycle Helpers (`include/linux/bpf.h`)

## `check_and_init_map_value()`, `bpf_obj_cancel_fields()`, `bpf_obj_free_fields()`

- `check_and_init_map_value(struct bpf_map *map, void *dst)` zeroes/reinitializes the special BTF fields skipped by `copy_map_value()`. Must be called when initializing a newly allocated element or zeroing special fields in a buffer copied to userspace.
- `bpf_obj_cancel_fields(const struct btf_record *rec, void *obj)` cancels active `BPF_TIMER`, `BPF_WORKQUEUE`, and `BPF_TASK_WORK` fields without freeing other special fields (`BPF_KPTR`, `BPF_LIST_HEAD`, `BPF_RB_ROOT`, `BPF_UPTR`).
- `bpf_obj_free_fields(const struct btf_record *rec, void *obj)` performs full destruction of all special fields in `rec`. Callers must ensure all field destructors for the map's permitted `field_mask` are safe in the calling context (e.g., non-NMI vs NMI).
