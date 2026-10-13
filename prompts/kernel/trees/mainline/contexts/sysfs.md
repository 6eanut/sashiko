# Sysfs Attribute Group Internals (`fs/sysfs/`)

## 1. `grp->attrs` / `grp->bin_attrs` Iteration and `is_visible` (`fs/sysfs/group.c`)

When an `attribute_group` defines `.is_visible` or `.is_bin_visible`, attributes for which the callback returns `0` (or named groups returning `SYSFS_GROUP_INVISIBLE`) have no `kernfs` node:
- Any `fs/sysfs/group.c` helper iterating `grp->attrs[]` or `grp->bin_attrs[]` and calling `kernfs_find_and_get()` (such as `sysfs_group_attrs_change_owner()` called from `sysfs_groups_change_owner()` during device/netns ownership changes) must evaluate `grp->is_visible` / `grp->is_bin_visible` first:
  - `if (mode & SYSFS_GROUP_INVISIBLE) break;`
  - `if (!mode) continue;`
  Otherwise `kernfs_find_and_get()` returns `NULL` for hidden attributes and aborts the group operation with `-ENOENT`.

## 2. Synchronization, Lifetimes, and Concurrency

*   **Locking Context:** `kernfs` operations (e.g., `kernfs_find_and_get`) can sleep. They must never be invoked from atomic contexts (interrupts or spinlock-held regions).
*   **Reference Counting:** `kernfs_find_and_get()` increments the `kernfs_node` reference count. Every successful lookup must be paired with a `kernfs_put()` on all exit paths to prevent reference leaks.
*   **State Serialization:** Modifications to attribute ownership require holding the `kobject` lock to prevent concurrent destruction or re-initialization. Iteration over `grp->attrs[]` assumes the array is immutable during the operation.
*   **Lockdep Compliance:** Ensure that `sysfs_symlink_target_lock` (defined in `fs/sysfs/sysfs.h`) is not held while performing operations that may trigger a recursive `kernfs` lookup or sleep.

## 3. Initialization and Error Handling

*   **Symmetry:** Every `sysfs_add_file_mode_ns` or `sysfs_add_bin_file_mode_ns` must have a corresponding removal path.
*   **Partial Failure:** `sysfs_update_group()` must not attempt to remove nodes that were not successfully created. Cleanup logic must verify node existence before removal to avoid `-ENOENT` or null pointer dereferences.
*   **Array Safety:** Code must explicitly check for `NULL` or empty `attrs` arrays before dereferencing.
*   **Error Unwinding:** When `sysfs_create_link_sd` fails, the caller must ensure that any previously created `kernfs_node` references are released via `kernfs_put` to prevent memory leaks.

## 4. Buffer Safety and API Usage

*   **`sysfs_emit()`:** The `show()` callback must use `sysfs_emit()` to ensure output does not exceed `PAGE_SIZE`. Manual `sprintf` usage is prohibited to prevent buffer overflows.
*   **Return Values:** Distinguish between `NULL` (node not found) and `ERR_PTR` (internal failure) when calling `kernfs` helpers.
*   **Input Validation:** Always validate that the length of data returned by a `show()` callback does not exceed the buffer provided by the VFS layer.

## 5. Critical Symbols (v7.3-rc5)

*   **`sysfs_group_attrs_change_owner`**: Primary entry for ownership updates; requires strict visibility filtering.
*   **`kernfs_find_and_get`**: Standard lookup mechanism; returns `NULL` if the node is hidden or missing.
*   **`SYSFS_GROUP_INVISIBLE`**: Bitmask flag signaling that an attribute group should be skipped during iteration.
*   **`sysfs_root_kn`**: Root of the sysfs `kernfs` tree.
*   **`sysfs_add_file_mode_ns`**: Internal helper for adding files with specific namespace and mode constraints.
*   **`sysfs_add_bin_file_mode_ns`**: Internal helper for adding binary attributes with specific size and mode constraints.
*   **`sysfs_symlink_target_lock`**: Spinlock protecting symlink target resolution.

## 6. Architectural Invariants and Review Checklist

1.  **Visibility Consistency:** Are all `is_visible` checks applied before `kernfs_find_and_get`? (Failure leads to `-ENOENT`).
2.  **Reference Balance:** Does every `kernfs_find_and_get` have a corresponding `kernfs_put` on all error paths?
3.  **Atomic Context:** Are there any `kernfs` calls inside `spin_lock` or `irq` handlers? (Must be removed).
4.  **Empty Array Handling:** Does the code handle `NULL` or empty `attribute_group` arrays without dereferencing?
5.  **Buffer Bounds:** Does the `show()` callback use `sysfs_emit()`?
6.  **Ownership Updates:** When changing ownership, is the `kobject` lock held to prevent concurrent teardown?
7.  **Symmetry:** Is the cleanup path the exact inverse of the initialization path?
8.  **Error Unwinding:** Does the error path only release resources that were explicitly allocated by the current operation?
9.  **Visibility Check Mismatch:** Does the subsystem verify the visibility of an object before attempting to modify its attributes?
10. **Lockdep Compliance:** Are all functions that perform state checks or modifications properly annotated or protected by the required locks?
