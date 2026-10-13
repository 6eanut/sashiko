# Device Tree Bindings Maintainer Notes

**MAINTAINERS Sections:** `OPEN FIRMWARE AND FLATTENED DEVICE TREE BINDINGS`

## Schema Organization & Splitting Policy

When `allOf:` / `if-then` conditional blocks in a single YAML binding schema become unwieldy due to divergent hardware variants (different register layouts, clocks, interrupts, or provider roles), split the binding into separate YAML schema documents rather than accumulating complex nested conditionals (per `Documentation/devicetree/bindings/example-schema.yaml`).
