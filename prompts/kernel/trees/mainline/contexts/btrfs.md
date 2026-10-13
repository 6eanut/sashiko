# Btrfs Subsystem Context (`fs/btrfs/`)

## 1. `struct extent_map` Fields & Invariants (`fs/btrfs/extent_map.h`, `extent_map.c`)

For uncompressed, non-reflinked extents, `len`, `ram_bytes`, and `disk_num_bytes` are equal.

- `start`: file offset (`BTRFS_EXTENT_DATA_KEY` offset).
- `len`: file byte range covered by this `extent_map` (`num_bytes` on disk; sector size for inline extents).
- `disk_bytenr`: physical start byte on disk (or sentinel `EXTENT_MAP_HOLE` / `EXTENT_MAP_INLINE`).
- `disk_num_bytes`: full size of the on-disk extent (compressed size when compressed).
- `offset`: byte offset within the decompressed extent where this file range starts (non-zero for reflinked/split partial references).
- `ram_bytes`: decompressed size of the entire on-disk extent (`ram_bytes == disk_num_bytes` when uncompressed; `ram_bytes > disk_num_bytes` when compressed).

### Replacement Helpers & `validate_extent_map()` Invariants

- `btrfs_extent_map_block_start(em)` (`extent_map.h`): returns `em->disk_bytenr` when `btrfs_extent_map_is_compressed(em)` is true, and `em->disk_bytenr + em->offset` when uncompressed.
- `extent_map_block_len(em)` (static in `extent_map.c`): returns `em->disk_num_bytes` when compressed, and `em->len` when uncompressed.
- `validate_extent_map()` invariants for data extents (`disk_bytenr < EXTENT_MAP_LAST_BYTE`):
  - `disk_num_bytes != 0` and `offset + len <= ram_bytes`.
  - When uncompressed: `offset + len <= disk_num_bytes` and `ram_bytes == disk_num_bytes`.
  - For holes/inline extents (`disk_bytenr >= EXTENT_MAP_LAST_BYTE`): `offset == 0`.

| Intended Quantity | Correct Expression | Common Bug |
|---|---|---|
| File range covered by `em` | `em->len` | `em->ram_bytes` |
| On-disk I/O length for compressed extent | `em->disk_num_bytes` | `em->len` |
| Full decompressed extent size | `em->ram_bytes` | `em->disk_num_bytes` |
| Physical disk start for I/O | `btrfs_extent_map_block_start(em)` | raw `em->disk_bytenr` |

## 2. Zoned Storage Active vs Open Zone Limits (`fs/btrfs/zoned.c`)

- `bdev_max_active_zones()` bounds zones that are implicitly open, explicitly open, or closed.
- `bdev_max_open_zones()` bounds zones concurrently open for writing (a stricter subset of active zones). A zoned block device may report `bdev_max_active_zones(bdev) == 0` (no active limit) while reporting a non-zero `max_open_zones`.
- In `btrfs_get_dev_zone_info()`, when `max_active_zones` is synthesized via `min_not_zero(bdev_max_active_zones(bdev), bdev_max_open_zones(bdev))`, mount-time validation (`nactive > max_active_zones`) must not fail with `-EIO` if `bdev_max_active_zones(bdev) == 0` (existing filesystems created on devices without a hardware active-zone limit can have more active zones than `max_open_zones`).

## 3. Synchronization, Lifecycle, and Integrity Invariants

- **Transaction Handles:** `struct btrfs_trans_handle` must be ended via `btrfs_end_transaction()`. Failure leads to transaction leaks and read-only transitions.
- **Lock Ordering:** `fs_info->commit_root_sem` must be held when iterating over commit roots to prevent concurrent transaction commits from freeing the tree root.
- **Atomic Contexts:** `GFP_KERNEL` allocations are forbidden inside `btrfs_func_t` callbacks executed in atomic/interrupt contexts. Use `GFP_ATOMIC` or pre-allocated workspaces.
- **Wait Queues:** Tasks must be added to wait queues *before* checking the condition to avoid lost wakeups (e.g., zstd workspace allocation).
- **Metadata Modification:** Modifying an `extent_buffer` requires exclusive access; ensure the buffer is locked before modification.
- **Arithmetic Overflow:** Calculate `start` and `len` independently. Do not calculate `end = start + len` before performing arithmetic to avoid integer overflows.
- **Error Propagation:** Helper functions must return standard negative errno values. Do not return positive values for errors, as this triggers incorrect transaction commit logic.
- **Transaction Aborts:** Metadata update failures require an immediate `btrfs_abort_transaction()` to prevent inconsistent on-disk states.

## 4. Architectural Review & Safety Protocols

- **Input Validation:** All offsets derived from untrusted disk metadata must be validated as non-negative. Use `btrfs_set_` and `btrfs_get_` accessors to ensure proper endianness conversion.
- **Object Lifecycles:** When an object is removed from a cache (e.g., `btrfs_device`), it must be unhashed or marked invalid before the final reference is dropped to prevent use-after-free.
- **Error Unwinding:** Every successful allocation or reference increment must be reversed in the exact reverse order of acquisition on error paths.
- **Reentrancy:** Ensure that callbacks do not invoke higher-layer functions that attempt to re-enter the lower layer, as this causes circular locking dependencies.
- **Alignment:** Use `btrfs_stripe_offset_nr` and related helpers for chunk/stripe calculations to ensure alignment with underlying block boundaries.
- **State Consistency:** Always check the filesystem's read-only status before proceeding with write-related operations.
- **Asynchronous Work:** All pending `btrfs_work` items must be flushed via `btrfs_flush_workqueue()` before the associated `fs_info` or device memory is released.
