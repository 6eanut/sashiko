# Open Firmware (Device Tree) Core & MSI Resolution Subsystem Guide (v7.3-rc5)

This guide defines the architectural invariants, synchronization contracts, and lifecycle protocols for the Linux Device Tree (`drivers/of/`) subsystem as of v7.3-rc5.

## 1. Subsystem Architecture & Object Lifetimes

*   **Core Locks:**
    *   `of_mutex`: Global mutex protecting the device tree structure during modifications (changeset applications, dynamic reconfig).
    *   `devtree_lock`: `raw_spinlock_t` for atomic access to the tree, required for early boot or interrupt-context lookups.
*   **Object Lifecycle:**
    *   `struct device_node` objects are reference-counted via `kref`.
    *   **Destruction:** `of_node_release()` is the final callback; it must only be invoked when the reference count reaches zero.
    *   **Dynamic Reconfig:** Use `of_changeset` APIs to ensure atomic application/reversion of tree modifications.
*   **Synchronization:** Lookups (e.g., `of_find_node_by_path`) are safe in atomic contexts for read-only traversals, provided they do not trigger memory allocations. Always use `of_node_get()`/`of_node_put()` when holding a node pointer across potential sleep points.

## 2. MSI Controller Device Tree Binding Variants (`drivers/of/irq.c`)

Device Tree platforms describe MSI controller routing using two distinct binding styles that must both be supported by MSI lookup paths:

1.  **`msi-map` binding:** Maps requester/device IDs to MSI controller phandles using `msi-map` and `msi-map-mask` properties, resolved via `of_map_id()`.
2.  **`msi-parent` binding:** References the MSI controller via a phandle (`of_check_msi_parent()`). When `#msi-cells` is absent or `0`, it defines a 1:1 ID mapping; when `#msi-cells` is non-zero, the phandle carries specifier cells.

`of_msi_xlate()` (`drivers/of/irq.c`) walks up the device hierarchy checking both `msi-map` and `msi-parent` to translate the MSI ID and resolve the target MSI controller node, while `of_msi_get_domain()` resolves the MSI IRQ domain across `msi-parent` entries.

## 3. Initialization, Publication, and Error Unwinding

*   **Publication:** A node is "published" once attached to the global tree. Ensure all properties (`reg`, `interrupts`, `clocks`) are fully populated before calling `of_attach_node()`.
*   **Error Unwind:** If a changeset fails, revert entries to restore the previous state. Failure to do so results in a corrupted tree state causing stale or invalid pointer returns.
*   **Sysfs Integration:** `__of_add_property_sysfs()` and `__of_detach_node_sysfs()` must remain synchronized with the internal tree state.
*   **Initialization Order:** Ensure address and size cell defaults are respected during early parsing to prevent address translation errors.

## 4. High-Frequency Bug Patterns & Invariants

*   **Property Mismatch:** `#*-cells` (e.g., `#iommu-cells`, `#msi-cells`) must strictly match the number of cells provided in the phandle argument list. Mismatches here are a primary cause of IOMMU/MSI configuration failures.
*   **Pointer Arithmetic:** Never perform manual pointer arithmetic on DT property values. Use `of_property_read_u32_array()` or equivalent helpers to verify buffer length.
*   **Namespace Collisions:** Node names must be unique within their parent to prevent overrides during DT overlay application.
*   **MSI Resolution Contract:** `of_msi_xlate` must return a valid `struct irq_domain *` or an error pointer; it must never return `NULL` on a successful lookup path.
*   **MSI Validation:** Validate the presence of the MSI controller node before domain resolution. If the controller is missing, return `-ENODEV` rather than triggering a kernel panic.
*   **Alias Management:** The `aliases_lookup` list and `struct alias_prop` must be kept in sync with the `aliases` node. Ensure `id` and `stem` are correctly parsed from the alias string.

## 5. Security & Input Validation

*   **Untrusted Input:** When parsing complex nested structures, ensure recursion depth is bounded to prevent stack exhaustion.
*   **Integer Integrity:** When calculating memory allocation sizes for DT properties, use `check_add_overflow` or similar helpers to prevent integer overflows.
*   **Bounds Checking:** All property reads must be validated against the actual buffer size returned by `of_get_property()`. Never assume a property exists or has a specific length without explicit verification.
*   **Hardware Dependencies:** All hardware dependencies (clocks, regulators, resets) must be explicitly defined in the DT. Relying on default dummy regulators or clocks is a common source of system instability.
*   **Memory Safety:** All pointer arithmetic operations and character lookups must be preceded by a bounds check to ensure they remain within the allocated memory region.
*   **Schema Compliance:** Ensure all device tree nodes conform to the required schema, including all mandatory properties and naming conventions, to avoid runtime discovery failures.
*   **Resource Tracking:** Every resource acquisition and reference increment must have a corresponding, balanced release path that executes under all possible error conditions.
*   **Context Awareness:** Ensure that blocking operations, memory allocations, and external synchronization waits are strictly excluded from atomic critical sections and spin-locked regions.
