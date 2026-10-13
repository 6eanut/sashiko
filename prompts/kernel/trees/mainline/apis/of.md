# Open Firmware (Device Tree) Node Reference & Iterator API (`include/linux/of.h`)

## OF Node Iterator Macros and Reference Counting

The OF node iterator macros automatically release the current node's reference (`of_node_put()`) when advancing to the next iteration:
- `for_each_child_of_node(parent, child)`
- `for_each_available_child_of_node(parent, child)`
- `for_each_reserved_child_of_node(parent, child)`
- `for_each_node_by_type(dn, type)`
- `for_each_compatible_node(dn, type, compatible)`
- `for_each_matching_node(dn, matches)`
- `for_each_matching_node_and_match(dn, matches, match)`
- `for_each_node_with_property(dn, prop_name)`

Reference rules:
- Normal loop advancement (including `continue`) automatically drops the current node reference. Calling `of_node_put(child)` before `continue` or at the end of every iteration causes a double-free.
- Early loop exits (`break`, `return`, `goto` out of the loop) leave the current iteration's node reference held; callers must call `of_node_put(child)` before exiting early (unless transferring ownership of `child`).
- **Scoped variants (`for_each_child_of_node_scoped()`, `for_each_available_child_of_node_scoped()`):** Declare the loop variable with `__free(device_node)`, releasing the reference automatically on both normal completion and early exit (`break` / `return`). Never call manual `of_node_put()` on the scoped loop variable unless ownership is transferred with `no_free_ptr()`.
- **`for_each_property_of_node(dn, pp)`:** Iterates the property list directly without modifying reference counts.

## OF Node Reference Acquisition and `from`-Argument Consumption

The following functions return a `struct device_node *` with an incremented reference count that must be released with `of_node_put()` (or `struct device_node *np __free(device_node)`):
- `of_find_node_by_path()`, `of_find_node_by_phandle()`, `of_parse_phandle()`, `of_get_parent()`, `of_get_child_by_name()`, `of_node_get()`
- **Functions that consume (`of_node_put`) their input/`from` argument:**
  - `of_get_next_parent(node)` acquires a reference on the returned parent and drops the reference on the input `node`.
  - `of_find_node_by_name(from, ...)`, `of_find_node_by_type(from, ...)`, `of_find_compatible_node(from, ...)`, `of_find_matching_node_and_match(from, ...)`, and `of_find_node_with_property(from, ...)` drop the reference on `from`; callers must not call `of_node_put(from)` after passing a non-NULL `from`.
