# Device Tree YAML Bindings Context (`Documentation/devicetree/bindings/`)

## `allOf:` / `if-then` Compatible String Enumeration

When adding a new compatible string (such as a new SoC generation `vendor,dev-gen5`) to the top-level `properties:compatible:` list in a `.yaml` binding:

- Audit every `allOf:` `if:properties:compatible:contains:enum:` (or `const:`) block in the schema (and related family schemas) constraining `interrupts`, `clocks`, `resets`, `power-domains`, `reg`, or `required`.
- Omitting the new compatible string from an `if-then` block that applies to its hardware family leaves those constraints unenforced during `dt_binding_check`.

## Provider Capability Required Properties

When a new compatible variant acts as a hardware provider, its conditional `required:` list and property definitions must include the standard provider properties with explicit `const:` cell counts:

| Provider Role | Required Properties |
|---|---|
| GPIO controller | `gpio-controller`, `#gpio-cells` |
| PWM provider | `#pwm-cells` |
| Clock provider | `#clock-cells` |
| Interrupt controller | `interrupt-controller`, `#interrupt-cells` |
| Reset provider | `#reset-cells` |

## `$id` Path Invariant

The `$id` URI must equal `http://devicetree.org/schemas/<relpath>#`, where `<relpath>` is the exact file path relative to `Documentation/devicetree/bindings/` (including the subdirectory, e.g., `gpio/vendor,device.yaml#`). A mismatched `$id` breaks `$ref` resolution and `dt_binding_check`.
