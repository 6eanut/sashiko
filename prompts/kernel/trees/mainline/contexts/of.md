# Open Firmware (Device Tree) Core & MSI Resolution (`drivers/of/`)

## MSI Controller Device Tree Binding Variants (`drivers/of/irq.c`)

Device Tree platforms describe MSI controller routing using two distinct binding styles that must both be supported by MSI lookup paths:
1. **`msi-map` binding:** Maps requester/device IDs to MSI controller phandles using `msi-map` and `msi-map-mask` properties, resolved via `of_map_id()`.
2. **`msi-parent` binding:** References the MSI controller via a phandle (`of_check_msi_parent()`). When `#msi-cells` is absent or `0`, it defines a 1:1 ID mapping; when `#msi-cells` is non-zero, the phandle carries specifier cells.

`of_msi_xlate()` (`drivers/of/irq.c`) walks up the device hierarchy checking both `msi-map` and `msi-parent` to translate the MSI ID and resolve the target MSI controller node, while `of_msi_get_domain()` resolves the MSI IRQ domain across `msi-parent` entries.
