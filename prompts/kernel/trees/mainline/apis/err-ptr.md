# Linux Kernel API Guide: Error Pointer Handling (`include/linux/err.h`)

This guide defines the authoritative usage patterns for the Linux kernel's error pointer encoding mechanism as of v7.3-rc5.

## 1. Core API Overview & Ownership
The kernel uses a reserved range of high memory addresses (the last `MAX_ERRNO` bytes, typically 4095) to encode negative error codes into pointer-sized return values. This allows functions to return either a valid memory address or a specific error code without requiring an out-parameter.

*   **Opaque Encoding:** Users must treat the result of `ERR_PTR()` as an opaque value. Never perform arithmetic on an error pointer.
*   **Ownership:** An error pointer does **not** represent a valid memory allocation. No reference counting or memory deallocation (e.g., `kfree`) is required for an error pointer.
*   **Lifecycle:** Error pointers are transient. They should be converted to an integer error code via `PTR_ERR()` immediately upon receipt to facilitate error handling and then discarded.

## 2. Calling Context, Locking & Synchronization
The `err.h` macros are pure arithmetic operations on pointer values.
*   **Context Safety:** `IS_ERR`, `PTR_ERR`, and `ERR_PTR` are **always safe** to call in any context, including hard IRQ handlers, NMI context, and atomic sections. They do not perform memory allocation, do not sleep, and do not acquire locks.
*   **Concurrency:** These macros are stateless and re-entrant. They do not require external synchronization.

## 3. Return Values, Error Sentinels & Unwind Symmetry
The most frequent source of kernel panics (307+ commits in the 100k corpus) is the confusion between `NULL` and `ERR_PTR`.

*   **The Golden Rule:** A function returning a pointer must document whether it returns `NULL` on failure, an `ERR_PTR` on failure, or both.
*   **`IS_ERR(ptr)` vs `IS_ERR_OR_NULL(ptr)`:**
    *   Use `IS_ERR()` if the function is guaranteed to return either a valid pointer or an error code (e.g., `ERR_PTR(-ENOMEM)`).
    *   Use `IS_ERR_OR_NULL()` if the function may return `NULL` as a valid "not found" or "no-op" state, in addition to error codes.
*   **Unwind Symmetry:** If a function returns an error pointer, the caller **must not** attempt to free the pointer. If a function returns a valid pointer, the caller assumes ownership and must ensure a corresponding `kfree` or reference-drop occurs on all exit paths.

## 4. High-Frequency Bug Patterns & Review Invariants

### The "IS_ERR vs NULL" Mismatch
*   **Bug:** Checking `if (!ptr)` when the function returns `ERR_PTR(-EIO)`. The check fails, and the subsequent dereference of the error pointer causes a kernel oops.
*   **Invariant:** Always verify the API contract of the callee. If the callee uses `ERR_PTR`, you **must** use `IS_ERR()` or `IS_ERR_OR_NULL()`.

### Error Masking & Propagation
*   **Bug:** Swallowing specific error codes (e.g., `return -ENOMEM` when the callee returned `-EACCES`).
*   **Invariant:** Use `PTR_ERR_OR_ZERO(ptr)` to propagate errors cleanly.
    ```c
    struct foo *p = get_foo();
    return PTR_ERR_OR_ZERO(p); // Correctly returns 0 on success, or the error code
    ```

### Resource Leak on Error Path
*   **Bug:** Failing to release resources (mutexes, IRQs, memory) when an error pointer is returned.
*   **Invariant:** The error cleanup path must mirror the allocation sequence. If a resource is acquired, the error path must release it **before** returning the error code to the caller.

### Uninitialized Variable Usage
*   **Bug:** Using an error-reporting structure that was not initialized because the function returned early.
*   **Invariant:** Ensure all error-reporting structures are initialized at the start of the function or immediately upon error detection.

## 5. Summary of Symbols (v7.3-rc5)

| Symbol | Purpose |
| :--- | :--- |
| `ERR_PTR(err)` | Encodes integer `err` into a pointer. |
| `PTR_ERR(ptr)` | Decodes pointer `ptr` back into an integer. |
| `IS_ERR(ptr)` | Returns `true` if `ptr` is an error pointer. |
| `IS_ERR_OR_NULL(ptr)` | Returns `true` if `ptr` is `NULL` or an error pointer. |
| `PTR_ERR_OR_ZERO(ptr)` | Returns `PTR_ERR(ptr)` if error, else `0`. |
| `ERR_CAST(ptr)` | Casts an error pointer to a different pointer type. |
| `MAX_ERRNO` | The threshold (4095) for error pointer detection. |

**Reviewer Checklist:**
1.  Does the function return `NULL` or `ERR_PTR` on failure? (Check the callee's documentation).
2.  Are you checking the return value **before** dereferencing the pointer?
3.  If an error is detected, are all previously acquired resources (locks, memory, IRQs) released?
4.  Are you using `PTR_ERR_OR_ZERO` to simplify error propagation?
5.  Are you including `<linux/err.h>`? (Missing this is a common build-time error).
