# CXL Subsystem Code Context Guide (Linux v7.3-rc5)

This guide provides architectural and safety invariants for the CXL subsystem (`drivers/cxl/`) and its integration with HMAT (`drivers/acpi/numa/hmat.c`).

## 1. Resource Type Initialization for HMAT Queries (`drivers/cxl/acpi.c`, `drivers/acpi/numa/hmat.c`)

- `hmat_get_extended_linear_cache_size(struct resource *backing_res, int nid, resource_size_t *cache_size)` matches `backing_res` against HMAT memory target resources using `resource_contains()` (`include/linux/ioport.h`).
- `resource_contains(r1, r2)` returns `false` immediately if `resource_type(r1) != resource_type(r2)`. Because HMAT target resources have type `IORESOURCE_MEM`, passing a `struct resource` initialized via `DEFINE_RES(start, size, 0)` (type `0`) causes every comparison to fail and `hmat_get_extended_linear_cache_size()` to return `0` with `*cache_size = 0`.
- Callers such as `cxl_setup_extended_linear_cache()` (`drivers/cxl/acpi.c`) must construct CXL HPA `struct resource` instances using `DEFINE_RES_MEM(start, size)` so `IORESOURCE_MEM` is set.

## 2. Subsystem Architecture, Core Data Structures & Object Lifetimes
- **Core Objects:** The CXL subsystem relies on a hierarchical model: `cxl_memdev` (Type-3 device), `cxl_port` (switch/root port), and `cxl_decoder` (HDM decoder).
- **Ownership & Lifetimes:** 
    - `struct cxl_memdev` is the primary handle for user-space ioctls. It contains a `struct cxl_dev_state *cxlds` which tracks hardware state.
    - **Rule:** Never access `cxlds` or `endpoint` pointers after the `cxl_memdev` has initiated unregistration. The `detach_work` mechanism handles asynchronous port loss; ensure all work is flushed via `cancel_work_sync()` before freeing the parent `cxl_memdev`.
    - **Rule:** Object references must be held until all potential accesses, including error logging, are completed. Use `get_device()`/`put_device()` symmetrically.

## 3. Synchronization Model, Lock Hierarchy & Concurrency Rules
- **Locking Hierarchy:** 
    - The CXL subsystem uses `cxl_region_rwsem` (for region configuration) and device-specific mutexes. 
    - **Rule:** When acquiring multiple locks, follow the topology depth: Root -> Switch Port -> Endpoint.
- **Interrupt Context:** 
    - CXL mailbox operations and register accesses are performed in process context. 
    - **Rule:** If a lock is shared between process context and an IRQ handler, it **must** be acquired using `spin_lock_irqsave()`/`spin_unlock_irqrestore()`.
- **Serialization:** Multi-part mailbox transfers must be serialized as a single atomic transaction. Use the `cxlds->mbox_mutex` to protect the entire sequence from command interleaving.

## 4. Initialization, Registration & Teardown Ordering
- **Publication Boundary:** A `cxl_memdev` must be fully initialized (including `cxlds` and `endpoint` discovery) before calling `device_add()`. 
- **Error Unwind:** 
    - Use `devm_*` managed resources to ensure symmetric cleanup. 
    - **Rule:** If manual allocation is required, ensure every `get_device()` or reference increment has a corresponding `put_device()` on all error paths. 
- **Teardown:** The teardown sequence must be the reverse of initialization. Ensure `cxl_memdev` is unregistered before the underlying `cxl_port` hierarchy is torn down to prevent NULL pointer dereferences in `cxled_to_port()`.

## 5. Error Handling, Resource Recovery & Boundary Validation
- **Pointer Validation:** Always check `IS_ERR()` for pointers returned by subsystem helpers (e.g., `devm_kzalloc`, `cxl_await_media_ready`). 
- **Hardware Limits:** 
    - Register reads (e.g., `CXL_HDM_DECODER0_BASE_LOW_OFFSET`) must be bounded by the hardware-reported count (`CXL_HDM_DECODER_COUNT_MASK`). 
- **Endianness:** All multi-byte values from the CXL mailbox or hardware registers must be converted from bus-endianness to host-endianness using `le32_to_cpu()` or similar.

## 6. High-Frequency Bug Patterns & False-Positive Guards
- **Bitmask vs. Bit Index:** 
    - **Guard:** Use bitwise operators (`&`) for flags (e.g., `status & CXLMDEV_DEV_FATAL`). Do not use bit index functions (like `test_bit`) on raw register masks.
- **Integer Overflows:** 
    - **Guard:** When calculating buffer sizes from user-provided input, use `size_add()` or `size_mul()`.
- **Format Specifiers:** 
    - **Guard:** Use `%pa` for `phys_addr_t` and `%pR` for `struct resource` to ensure architecture-independent printing.
- **Stale State:** 
    - **Guard:** Always verify `CXLMDEV_READY()` before attempting mailbox commands. If the device reports `CXLMDEV_MS_ERROR`, abort the operation and trigger a reset sequence.
- **Loop Termination:** 
    - **Guard:** Every loop processing device-provided data (e.g., poison lists) must have a hard limit or a guaranteed exit condition based on the hardware-reported count.
- **Probe Safety:** 
    - **Guard:** Always validate pointers for NULL before dereferencing during object discovery. Ensure error codes from initialization helpers are propagated correctly and not swallowed.
