# Linux Kernel DMA Mapping API Guide (v7.3-rc5)

This guide provides authoritative rules for the Linux DMA Mapping API. Adherence to these invariants is mandatory for all drivers interacting with the DMA subsystem to prevent memory corruption, IOMMU leaks, and hardware-induced system instability.

---

## 1. Core API Overview & Ownership
The DMA API manages the translation between CPU virtual addresses and device-visible bus addresses.

*   **Ownership:** When a buffer is mapped (e.g., `dma_map_single`), the device gains ownership of the memory. The CPU must **not** access the buffer until it is unmapped or synchronized via `dma_sync_single_for_cpu`.
*   **Reference Counting:** The DMA API uses **strict symmetry**. Every `dma_map_*` call must have a corresponding `dma_unmap_*` call using the exact same parameters (size, direction, and device pointer).
*   **Teardown Symmetry:** If a probe function fails after a successful `dma_map_*`, the error path **must** invoke the corresponding `dma_unmap_*` before returning. Failure to do so results in IOMMU address space exhaustion and memory leaks.

## 2. Calling Context, Locking & Synchronization
*   **Sleeping Constraints:** `dma_alloc_coherent` and `dmam_alloc_coherent` may sleep. They must not be called from atomic context (e.g., hard IRQ, spinlock-held sections). Use `GFP_ATOMIC` if allocation is required in restricted contexts, noting that this increases the risk of allocation failure.
*   **Synchronization:** If `DMA_ATTR_SKIP_CPU_SYNC` is not used, the CPU must explicitly synchronize caches.
    *   **Device Access:** Use `dma_sync_single_for_device` before the device reads/writes the buffer.
    *   **CPU Access:** Use `dma_sync_single_for_cpu` before the CPU reads the buffer after a device-to-host transfer.
*   **Memory Barriers:** The DMA API provides necessary memory barriers for hardware visibility. When using custom ring buffers or lockless structures, ensure explicit `wmb()` or `rmb()` calls are used to order memory writes before updating the hardware descriptor.

## 3. Return Values & Error Sentinels
*   **Mapping Errors:** Always check the result of `dma_map_single` or `dma_map_page` using `dma_mapping_error(dev, dma_addr)`. A return of `DMA_MAPPING_ERROR` indicates failure.
*   **Allocation Errors:** `dma_alloc_coherent` returns `NULL` on failure. Do not use `IS_ERR()` on these pointers; they are standard `NULL` sentinels.
*   **Scatterlist Mapping Return Value (`dma_map_sg`):** `dma_map_sg()` returns the number of mapped DMA address segments (`count`). A return value of `0` indicates mapping failure (`dma_map_sgtable()` returns a negative errno instead). A return value `0 < count < nents` is normal success when the IOMMU coalesces adjacent segments: iterate over `count` segments during DMA programming (`for_each_sg(sgl, sg, count, i)`), and pass the original `nents` to `dma_unmap_sg()`.

## 4. High-Frequency Bug Patterns & Review Invariants
*   **Stack Buffers:** **NEVER** map buffers located on the kernel stack. DMA engines cannot reliably access stack memory, and cache coherency operations on stack addresses are undefined.
*   **Alignment:** DMA buffers must be aligned to the architecture's cache line size. Use `dma_get_cache_alignment()` to verify requirements. Misaligned buffers lead to silent data corruption due to "cache line sharing."
*   **Scatterlist Iteration:** When iterating over a scatterlist, always use the `dma_length` (via `sg_dma_len(sg)`) for the DMA operation, not the original `length` field. The DMA mapping process may coalesce or split segments.
*   **Direction Mismatch:** The `enum dma_data_direction` must be consistent. Using `DMA_TO_DEVICE` for a buffer that the device writes to will cause cache incoherency on architectures with non-coherent DMA.
*   **Zero-Initialization:** When using bounce buffers or padding, ensure the entire allocated region is zeroed. Failure to do so leaks uninitialized kernel memory to the device.

## 5. Device-Managed Cleanup (`dmam_*`)
The `dmam_*` family automates resource release.

*   **Rule:** Do not mix manual `dma_free_coherent` with `dmam_alloc_coherent`.
*   **Probe Failure:** If a probe function fails, the `dmam` framework automatically releases resources. Do not manually free them in the error path to avoid double-free.
*   **Manual Detach:** If a resource must be freed *before* the driver detaches, use `dmam_free_coherent`.

## 6. Scatterlist & Table Integrity
*   **SG Table Ownership:** After `dma_map_sgtable`, the device owns the buffer.
*   **Chaining:** When using `sg_next()`, the API handles the `SG_CHAIN` bit automatically. Do not manually parse `page_link` bits.
*   **Validation:** Always verify that `nents` passed to `dma_map_sg` does not exceed the `orig_nents` of the `sg_table`.

## 7. Summary Checklist for Reviewers
1.  **Is the buffer on the stack?** (Reject if yes).
2.  **Is `dma_mapping_error()` checked?** (Reject if no).
3.  **Is the DMA direction correct?** (Verify against usage).
4.  **Are `dma_map` and `dma_unmap` symmetric?** (Check all error paths).
5.  **Is the buffer aligned?** (Check `dma_get_cache_alignment`).
6.  **Are we using `sg_dma_len` for DMA operations?** (Reject if using `sg->length`).
7.  **Is the device pointer valid?** (Ensure it is the `struct device` that performs the DMA).
