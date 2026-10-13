# Open Firmware (Device Tree) Node Reference & Iterator API (`include/linux/of.h`)

## 1. OF Node Iterator Macros and Reference Counting

The OF node iterator macros automatically release the current node's reference (`of_node_put()`) when advancing to the next iteration:
- `for_each_child_of_node(parent, child)`
- `for_each_available_child_of_node(parent, child)`
- `for_each_reserved_child_of_node(parent, child)`
- `for_each_node_by_type(dn, type)`
- `for_each_compatible_node(dn, type, compatible)`
- `for_each_matching_node(dn, matches)`
- `for_each_matching_node_and_match(dn, matches, match)`
- `for_each_node_with_property(dn, prop_name)`

**Reference Rules:**
- **Normal Advancement:** Loop advancement (including `continue`) automatically drops the current node reference. Calling `of_node_put(child)` inside the loop body causes a double-free.
- **Early Exit:** Early loop exits (`break`, `return`, `goto` out of the loop) leave the current iteration's node reference held; callers must call `of_node_put(child)` before exiting early (unless transferring ownership of `child`).
- **Scoped Variants:** `for_each_child_of_node_scoped()` and `for_each_available_child_of_node_scoped()` declare the loop variable with `__free(device_node)`, releasing the reference automatically on both normal completion and early exit. Never call manual `of_node_put()` on the scoped loop variable unless ownership is transferred with `no_free_ptr()`.
- **Property Iteration:** `for_each_property_of_node(dn, pp)` iterates the property list directly without modifying reference counts.

## 2. OF Node Reference Acquisition and `from`-Argument Consumption

The following functions return a `struct device_node *` with an incremented reference count that must be released with `of_node_put()` (or `struct device_node *np __free(device_node)`):
- `of_find_node_by_path()`, `of_find_node_by_phandle()`, `of_parse_phandle()`, `of_get_parent()`, `of_get_child_by_name()`, `of_node_get()`

**Functions that consume (`of_node_put`) their input/`from` argument:**
- `of_get_next_parent(node)` acquires a reference on the returned parent and drops the reference on the input `node`.
- `of_find_node_by_name(from, ...)`, `of_find_node_by_type(from, ...)`, `of_find_compatible_node(from, ...)`, `of_find_matching_node_and_match(from, ...)`, and `of_find_node_with_property(from, ...)` drop the reference on `from`; callers must not call `of_node_put(from)` after passing a non-NULL `from`.

## 3. Code Review Invariants & Lifecycle Management

*   **Unwind Symmetry:** Every successful acquisition of a `device_node` must have a corresponding `of_node_put()` on all error and success paths. Use `goto` labels for reverse-order cleanup in complex functions.
*   **Sentinel Checks:** All `of_find_*` and `of_get_*` functions return `NULL` on failure. Do not check for `ERR_PTR`.
*   **Atomic Context:** Avoid `of_find_*` functions in hard interrupt handlers. These functions often take `devtree_lock` (a spinlock) or perform operations that may sleep.
*   **Structural Integrity:** Do not modify the device tree (e.g., `of_detach_node`) while iterating over it. Modifications require holding the `devtree_lock` (or `of_mutex` in specific contexts).
*   **Unit-Address Consistency:** Ensure node names with unit-addresses (e.g., `@1000`) match the first value in the `reg` property.
*   **Hardware Dependencies:** Verify that all `clocks`, `resets`, and `power-domains` are explicitly defined in the DT. Relying on default/dummy values is prohibited.
*   **Initialization:** Nodes created via `kzalloc` must be initialized with `of_node_init()`. This sets the refcount to 1 and prepares the `kobj` structure.
*   **Status Property:** Always check `of_device_is_available(node)` before probing a device to respect the `status` property.
*   **Provider Unregistration:** Global providers (clocks, regulators) must be explicitly unregistered during device removal to prevent dangling references.
*   **Phandle Mapping:** The number of cells in a phandle map entry (e.g., `iommu-map`) must strictly match the target provider's defined cell count (`#iommu-cells`).
*   **Memory Safety:** When parsing variable-length structures from firmware, verify that the declared length does not exceed the actual remaining buffer size to prevent out-of-bounds access.
*   **Concurrency:** When iterating over a shared list using read-copy-update mechanisms, ensure the traversed objects are protected by the appropriate lock or RCU read-side critical section.
*   **Initialization Order:** Ensure the object state is fully validated and initialized before it is registered with the central subsystem and exposed to external consumers.
*   **Device-Managed Cleanup:** Use `devm_of_node_put()` if the node reference is tied to a device's lifecycle to ensure automatic cleanup upon driver unbind.
*   **Flag Management:** Use `of_node_check_flag()` and `of_node_set_flag()` for atomic state tracking of `OF_DYNAMIC`, `OF_DETACHED`, and `OF_POPULATED` states.
*   **Memory Barriers:** When publishing a node to a global list, ensure proper memory ordering to prevent other CPUs from observing a partially initialized `device_node`.
