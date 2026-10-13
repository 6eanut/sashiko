# Sysfs Group & Kobject Lifecycle API (`include/linux/sysfs.h`, `include/linux/kobject.h`)

## Kobject and Sysfs Group Creation Cleanup (`kobject_create_and_add`, `sysfs_create_group`)

- `kobject_create_and_add(name, parent)` returns a referenced `struct kobject *` (`NULL` on failure) and increments `parent`'s reference count via `kobject_add()`.
- When creating multiple kobjects or attaching sysfs groups (`sysfs_create_group()`) in a loop or hierarchy:
  - If `sysfs_create_group(children[i], &grp)` fails, `kobject_put(children[i])` must still be called on the current element in addition to `children[0 .. i-1]`.
  - Each created child must be released individually via `kobject_put()` on the unwind path (putting only `parent` while children still hold references on `parent` leaks the entire hierarchy).

## Attribute Group Visibility (`is_visible`, `is_bin_visible`, `SYSFS_GROUP_INVISIBLE`)

- In `struct attribute_group`, when `is_visible(kobj, attr, idx)` (or `is_bin_visible(kobj, battr, idx)` for `bin_attrs`) returns `0`, `sysfs_create_group()` (`create_files()`) does **not** create the kernfs node at all (unlike returning `-EOPNOTSUPP` from `.show()`).
- For named groups (`grp->name != NULL`), returning `SYSFS_GROUP_INVISIBLE` from `is_visible` or `is_bin_visible` suppresses creation of the entire group subdirectory.
