# Sysfs Attribute Group Internals (`fs/sysfs/`)

## `grp->attrs` / `grp->bin_attrs` Iteration and `is_visible` (`fs/sysfs/group.c`)

When an `attribute_group` defines `.is_visible` or `.is_bin_visible`, attributes for which the callback returns `0` (or named groups returning `SYSFS_GROUP_INVISIBLE`) have no kernfs node:
- Any `fs/sysfs/group.c` helper iterating `grp->attrs[]` or `grp->bin_attrs[]` and calling `kernfs_find_and_get()` (such as `sysfs_group_attrs_change_owner()` called from `sysfs_groups_change_owner()` during device/netns ownership changes) must evaluate `grp->is_visible` / `grp->is_bin_visible` first:
  - `if (mode & SYSFS_GROUP_INVISIBLE) break;`
  - `if (!mode) continue;`
  Otherwise `kernfs_find_and_get()` returns `NULL` for hidden attributes and aborts the group operation with `-ENOENT`.
