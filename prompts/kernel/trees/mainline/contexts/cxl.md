# CXL Subsystem Context (`drivers/cxl/`)

## Resource Type Initialization for HMAT Queries (`drivers/cxl/acpi.c`, `drivers/acpi/numa/hmat.c`)

- `hmat_get_extended_linear_cache_size(struct resource *backing_res, int nid, resource_size_t *cache_size)` matches `backing_res` against HMAT memory target resources using `resource_contains()` (`include/linux/ioport.h`).
- `resource_contains(r1, r2)` returns `false` immediately if `resource_type(r1) != resource_type(r2)`. Because HMAT target resources have type `IORESOURCE_MEM`, passing a `struct resource` initialized via `DEFINE_RES(start, size, 0)` (type `0`) causes every comparison to fail and `hmat_get_extended_linear_cache_size()` to return `0` with `*cache_size = 0`.
- Callers such as `cxl_setup_extended_linear_cache()` (`drivers/cxl/acpi.c`) must construct CXL HPA `struct resource` instances using `DEFINE_RES_MEM(start, size)` so `IORESOURCE_MEM` is set.
