# Sysfs Group & Kobject Lifecycle API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for managing `kobject` lifecycles and `sysfs` attribute groups. Adherence to these patterns is mandatory to prevent common kernel vulnerabilities, including use-after-free (UAF), reference count imbalances, and dangling sysfs entries.

## 1. Kobject and Sysfs Group Creation Cleanup
*   **`kobject_create_and_add(name, parent)`**: Returns a referenced `struct kobject *` (`NULL` on failure) and increments `parent`'s reference count via `kobject_add()`.
*   **Hierarchy Teardown**: When creating multiple kobjects or attaching sysfs groups (`sysfs_create_group()`) in a loop or hierarchy:
    *   If `sysfs_create_group(children[i], &grp)` fails, `kobject_put(children[i])` must still be called on the current element in addition to `children[0 .. i-1]`.
    *   Each created child must be released individually via `kobject_put()` on the unwind path. Putting only the `parent` while children still hold references on the `parent` leaks the entire hierarchy.
*   **Teardown Sequence**: To destroy a `kobject`, you **must** call `kobject_del()` to remove it from sysfs and unhash it, followed by `kobject_put()` to drop the final reference. Calling `kobject_put()` without `kobject_del()` is a common source of UAF and directory name collision bugs.

## 2. Attribute Group Visibility
*   **`is_visible` / `is_bin_visible`**: In `struct attribute_group`, when `is_visible(kobj, attr, idx)` (or `is_bin_visible(kobj, battr, idx)` for `bin_attrs`) returns `0`, `sysfs_create_group()` (`create_files()`) does **not** create the kernfs node at all (unlike returning `-EOPNOTSUPP` from `.show()`).
*   **Named Groups**: For named groups (`grp->name != NULL`), returning `SYSFS_GROUP_INVISIBLE` from `is_visible` or `is_bin_visible` suppresses creation of the entire group subdirectory.
*   **Constraint**: Only `read`/`write` permissions and `SYSFS_PREALLOC` are valid return values for attribute permissions.

## 3. Calling Context & Synchronization
*   **Sleeping Constraints**: `sysfs_create_group()` and related functions perform memory allocations (`GFP_KERNEL`) and internal locking. They **must not** be called from atomic context (interrupts, spinlocks, or `preempt_disable()` regions).
*   **Registration Ordering**: Never expose a `kobject` to userspace (via `kobject_add`) until all internal state, including sysfs attributes and private data, is fully initialized.
*   **Concurrency**: `sysfs_update_group()` is designed to handle visibility changes. It is the caller's responsibility to ensure that the underlying data structures accessed by `is_visible` or `show`/`store` callbacks remain valid for the duration of the call.

## 4. High-Frequency Bug Patterns & Review Invariants

| Bug Pattern | Prevention Rule |
| :--- | :--- |
| **Dangling Sysfs Entries** | Always call `sysfs_remove_group()` or `kobject_del()` before freeing the parent structure. |
| **UAF on Removal** | Ensure `kobject_del()` is called before `kobject_put()`. |
| **Double Free** | Never call `kfree()` on a `kobject` directly; use the `ktype->release` callback. |
| **Race Condition** | Do not expose objects to userspace until `kobject_init_and_add` is complete. |
| **Integer Truncation** | Use `kstrto*` helpers in `store` callbacks; never use `simple_strtoul`. |
| **NULL Dereference** | Always check if optional callbacks (e.g., `show`/`store`) are non-NULL before invocation. |

## 5. Advanced Lifecycle & Ownership Rules
*   **Reference Ownership**: When a subsystem lookup yields a shared object, the caller must assume ownership of a new reference (via `kobject_get`) that must be explicitly dropped via `kobject_put`.
*   **Asynchronous Handoff**: When passing an object to an asynchronous worker, the caller must duplicate the reference (`kobject_get`) before scheduling the work to ensure the object remains valid during execution.
*   **Initialization Failure**: If an initialization sequence fails, the rollback path must mirror the registration sequence in reverse order. If `kobject_init_and_add` succeeds, `kobject_del` must be called before the error path proceeds to release other resources.
*   **Memory Ordering**: When publishing a newly initialized object to a shared data structure, ensure that all initialization is visible to other processors (e.g., via `smp_wmb` or implicit barriers in list/kobject primitives).
*   **Device-Managed Resources**: Prefer `devm_` variants for resource allocation to automate cleanup. However, note that `devm_` cleanup occurs *after* the driver's `remove` callback; ensure that any sysfs attributes that depend on driver-specific state are removed manually in `remove` to prevent access to stale data.
*   **Input Validation**: Always validate user-provided lengths and offsets in `store` callbacks against the maximum capacity of the target buffer to prevent out-of-bounds memory accesses. Use `sysfs_emit` or `sysfs_emit_at` for safe output formatting.
*   **State Idempotency**: Ensure that `is_visible` callbacks are idempotent and do not rely on transient state that might change between the initial check and the actual file creation. If state changes, use `sysfs_update_group()` to trigger a re-evaluation.
*   **Error Handling**: Always check the return value of `sysfs_create_group()`. If it returns a non-zero value, the caller must unwind all previously successful operations in the current initialization sequence to prevent resource leaks.
*   **Locking Hierarchy**: Never hold a mutex while calling `sysfs_create_group` if the `is_visible` callback or any other sysfs-related operation might attempt to acquire the same mutex, as this will result in a deadlock.
*   **Lifecycle Stability**: Ensure that `kobject_del` is called before the final `kobject_put` to guarantee that the sysfs directory is removed and no further userspace access can occur while the object is being destroyed.
*   **Callback Safety**: Always verify that the `kobject` passed to `show` or `store` is still valid by checking the object's state flags or using a reference-counted container structure.
*   **Namespace Awareness**: When working with `kobj_ns_type`, ensure that the namespace is correctly handled during `kobject_add` to prevent cross-namespace visibility leaks.

