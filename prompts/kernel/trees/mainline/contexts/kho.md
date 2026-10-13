# Kexec Handover (KHO) and Live Update Orchestrator (LUO) Code Context Guide

This guide provides the authoritative technical context for the KHO and LUO subsystems in Linux v7.3-rc5.

## 1. Subsystem Architecture and Core Data Structures

*   **KHO Scratch Regions**: The `kho_scratch` array (managed by `kho_reserve_scratch()`) defines memory regions reserved for the successor kernel.
    *   **Invariant**: `kho_scratch_overlap()` must be checked before any memory preservation. Any overlap is a fatal configuration error.
*   **LUO Sessions (`struct luo_session`)**: The primary unit of state preservation.
    *   **Lifecycle**: Sessions are created via `luo_session_create()`, serialized into KHO subtrees, and retrieved in the successor kernel via `luo_session_retrieve()`.
    *   **Ownership**: The `luo_session` mutex protects the `file_set` and `retrieved` status. Once `retrieved` is true, the session is owned by the new kernel's consumer.
*   **Native-Endian FDTs**: KHO handover FDTs store property values in **native host endianness**. Do not flag native-endian reads or writes on KHO FDT properties as endianness bugs.

## 2. Synchronization Model and Lock Hierarchy

*   **Registration Serialization**: `luo_register_rwlock` (a `rw_semaphore`) protects the registration of file handlers and FLBs. It must be held when modifying the global list of sessions or file handlers.
*   **Session Mutex**: `struct luo_session->mutex` serializes access to individual session state.
*   **Atomic State Transitions**: During `liveupdate_reboot()`, the system enters a quiescent state.
    *   **Rule**: Use `luo_file_freeze()` to transition files to a serialized state. This must be atomic relative to any further I/O on the file descriptors.

## 3. Initialization, Registration, and Teardown

*   **Initialization Sequence**:
    1.  `kho_memory_init_early()`: Reserves scratch regions.
    2.  `kho_memory_init()`: Initializes KHO infrastructure.
    3.  `luo_session_setup_outgoing()`: Prepares the outgoing state tree.
*   **Error Unwind**:
    *   If `luo_session_deserialize()` fails, the system must `panic()` via `luo_restore_fail()`.
    *   **Symmetry**: Every `kho_preserve_*` call must have a corresponding `kho_unpreserve_*` call on error paths to prevent memory leaks into the scratch region.

## 4. Error Handling and Resource Recovery

*   **Scratch Region Overlap**: `kho_preserve_folio()` and `kho_preserve_pages()` reject ranges overlapping the CMA-backed KHO scratch regions with `WARN_ON` and `-EINVAL`.
*   **Return Value Sentinels**: Distinguish between `NULL` (allocation failure) and error pointers (e.g., `ERR_PTR(-ENOMEM)`).
*   **Partial Failure**: If `luo_file_freeze()` fails, the `luo_file_set` must be cleaned up via `luo_file_unfreeze()`.

## 5. Bug Patterns and False-Positive Guards

*   **Integer Overflow**: When calculating memory sizes for `kho_alloc_preserve()`, use 64-bit arithmetic for shifts (e.g., `(u64)nr_pages << PAGE_SHIFT`).
*   **Alignment**: KHO scratch regions and preserved folios must be aligned to `PAGE_SIZE`. Use `IS_ALIGNED()` checks.
*   **Use-After-Free**: Ensure `module_put()` is not called until all file operations on a preserved session are complete.
*   **False-Positive Guard (Endianness)**: Do not flag native-endian access to KHO FDTs as an endianness bug.
*   **False-Positive Guard (KASAN)**: When restoring memory via `kho_restore_vmalloc()`, ensure the memory is unpoisoned if it was previously tracked by KASAN.
*   **List Iterator Safety**: Do not dereference list iterators after the loop terminates unless the pointer was explicitly saved.

## 6. Untrusted Input Boundaries

*   **UAPI Validation**: All `luo_ucmd` inputs must be validated for length and null-termination.
*   **Reserved Fields**: Reserved fields in the KHO/LUO UAPI must be checked for zero values.
*   **String Handling**: Use `strscpy()` for session names and validate lengths against subsystem-defined maximums.

## 7. Cross-Layer Contracts

*   **Callback Context**: Callbacks registered with LUO must be prepared to execute in a non-preemptible context during the final `liveupdate_reboot()` sequence. Do not perform blocking operations (e.g., `mutex_lock`) inside these callbacks.
