# Btrfs Subsystem Context (`fs/btrfs/`)

## `struct extent_map` Fields & Invariants (`fs/btrfs/extent_map.h`, `extent_map.c`)

For uncompressed, non-reflinked extents, `len`, `ram_bytes`, and `disk_num_bytes` are equal, hiding field-selection bugs that corrupt or over-read compressed or reflinked/partial extents:

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
| File range covered by `em` | `em->len` | `em->ram_bytes` (over-reads/over-allocates on partial refs) |
| On-disk I/O length for compressed extent | `em->disk_num_bytes` | `em->len` (reads past on-disk extent) |
| Full decompressed extent size | `em->ram_bytes` | `em->disk_num_bytes` (truncates compressed data) |
| Physical disk start for I/O | `btrfs_extent_map_block_start(em)` | raw `em->disk_bytenr` (misses `em->offset` on uncompressed partial refs) |

## Zoned Storage Active vs Open Zone Limits (`fs/btrfs/zoned.c`)

- `bdev_max_active_zones()` bounds zones that are implicitly open, explicitly open, or closed.
- `bdev_max_open_zones()` bounds zones concurrently open for writing (a stricter subset of active zones). A zoned block device may report `bdev_max_active_zones(bdev) == 0` (no active limit) while reporting a non-zero `bdev_max_open_zones(bdev)`.
- In `btrfs_get_dev_zone_info()`, when `max_active_zones` is synthesized via `min_not_zero(bdev_max_active_zones(bdev), bdev_max_open_zones(bdev))`, mount-time validation (`nactive > max_active_zones`) must not fail with `-EIO` if `bdev_max_active_zones(bdev) == 0` (existing filesystems created on devices without a hardware active-zone limit can have more active zones than `max_open_zones`).
