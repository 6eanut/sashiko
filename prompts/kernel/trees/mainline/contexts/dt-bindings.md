# Device Tree YAML Bindings Context (`Documentation/devicetree/bindings/`)

## 1. Schema Validation and Compatibility Invariants

### `$id` Path Invariant
The `$id` URI must equal `http://devicetree.org/schemas/<relpath>#`, where `<relpath>` is the exact file path relative to `Documentation/devicetree/bindings/` (including the subdirectory, e.g., `gpio/vendor,device.yaml#`). A mismatched `$id` breaks `$ref` resolution and `dt_binding_check`.

### `allOf:` / `if-then` Compatible String Enumeration
When adding a new compatible string (such as a new SoC generation `vendor,dev-gen5`) to the top-level `properties:compatible:` list in a `.yaml` binding:
- Audit every `allOf:` `if:properties:compatible:contains:enum:` (or `const:`) block in the schema (and related family schemas) constraining `interrupts`, `clocks`, `resets`, `power-domains`, `reg`, or `required`.
- Omitting the new compatible string from an `if-then` block that applies to its hardware family leaves those constraints unenforced during `dt_binding_check`.

## 2. Provider Capability Required Properties
When a new compatible variant acts as a hardware provider, its conditional `required:` list and property definitions must include the standard provider properties with explicit `const:` cell counts:

| Provider Role | Required Properties |
|---|---|
| GPIO controller | `gpio-controller`, `#gpio-cells` |
| PWM provider | `#pwm-cells` |
| Clock provider | `#clock-cells` |
| Interrupt controller | `interrupt-controller`, `#interrupt-cells` |
| Reset provider | `#reset-cells` |

## 3. Kernel-Side DT Interaction and Resource Management

### Object Lifetime and Concurrency
- **Reference Counting:** Nodes are parsed during boot or via `of_platform_populate`. When traversing nodes (e.g., `of_find_node_by_path`), the caller must hold a reference. Every `of_node_get()` must be paired with an `of_node_put()` to prevent memory leaks or use-after-free during dynamic reconfiguration.
- **Atomic Context:** `of_find_*` functions are safe in atomic contexts, but the returned reference-counted pointer must be released via `of_node_put()`.
- **Resource Tracking:** Use `devm_` managed functions (e.g., `devm_of_iomap`) to ensure resources mapped via DT are automatically released on driver detach.

### Error Handling and Boundary Validation
- **Phandle Mapping:** When using `of_parse_phandle_with_args`, the number of arguments must strictly match the `#*-cells` property of the provider. Mismatches cause silent data corruption or kernel panics.
- **Required Properties:** If a required property is missing, the driver must return `-EINVAL` or `-ENODEV`. Do not proceed with default values if the hardware specification mandates a specific configuration.

## 4. Empirical Bug Prevention (100k Fixes Analysis)
- **Unit Addresses:** Ensure `reg` properties match the unit address in the node name. Mismatched unit addresses are a primary cause of probe failures.
- **Regulator/Clock Supplies:** Explicitly define all power supplies. Relying on "dummy" regulators masks hardware instability.
- **Pin Definitions:** Verify pin group names against the SoC datasheet. Referencing non-existent pin groups causes silent hardware misconfiguration.
- **Phandle Cell Counts:** The number of cells in a map (e.g., `iommu-map`, `dma-ranges`) must strictly match the target provider's `#*-cells` property.
- **Always-On Regulators:** Use `regulator-always-on` only when strictly necessary. Over-use prevents power-cycling required for features like SD card voltage switching.
- **Interrupt Triggers:** Always specify the trigger type (e.g., `IRQ_TYPE_LEVEL_HIGH`) in the `interrupts` property. Relying on hardware defaults is non-portable.
- **Reset Lines:** If a device has a hardware reset pin, it must be defined as a `reset-gpios` or `resets` property. Failure to toggle this line often leaves the device in a hung state.

## 5. Architectural Invariants for Subsystem Stability
- **Initialization Sequencing:** Ensure that `of_platform_device_create` or `of_platform_populate` is only called after the parent bus is fully initialized. Premature publication of child nodes leads to null pointer dereferences if the driver probes before the parent resources are ready.
- **Asynchronous Teardown:** When a device is removed, ensure all `of_node_put` calls are executed in the reverse order of acquisition. Use `devm_add_action_or_reset` to register cleanup callbacks for non-standard DT resources.
- **Input Validation:** When parsing `reg` or `ranges` properties, validate that the address space size does not exceed the architecture's physical address limit. Use `of_read_number` to handle 64-bit values correctly on 32-bit systems.
- **Cross-Layer Contracts:** When a driver consumes a phandle from another subsystem (e.g., `clocks`), it must verify the provider is active. If the provider is a module, use `try_module_get` or ensure the dependency is enforced via `Kconfig` to prevent use-after-free if the provider module is unloaded.
- **Locking Discipline:** The DT subsystem relies on internal mutexes for tree modification. Avoid holding locks across blocking calls. Use `of_node_check_flag` to verify node state before modification.
- **Memory Safety:** All string properties (e.g., `compatible`, `model`) must be validated for null-termination before being passed to `kstrdup` or `devm_kstrdup`.
- **Integer Integrity:** Use `of_property_read_u32` or `of_property_read_u64` to avoid manual pointer arithmetic on DT properties, which is prone to endianness and alignment errors.
