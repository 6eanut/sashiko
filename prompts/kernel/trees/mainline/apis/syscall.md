# Syscall Parameter Validation & User Copy API Guide (Linux v7.3-rc5)

This guide defines the authoritative rules for implementing system calls and interacting with user-space memory. Adherence to these invariants is mandatory for maintaining kernel memory safety and preventing information leaks.

## 1. Flag-Gated Syscall Parameter Trust Boundaries (`SYSCALL_DEFINE*`)

*   **Untrusted Input**: Syscall arguments come directly from user-controlled registers or stack slots. Arguments (or `copy_struct_from_user()` / `copy_from_user()` fields) that are only defined when a specific `flags` bit is set may contain arbitrary uninitialized values when that flag bit is absent—userspace is not required to zero-fill unused register arguments.
*   **Inherited Garbage**: Copying raw syscall arguments into an internal kernel struct does not sanitize them: each struct field inherits the flag gate of its source argument and remains untrusted garbage when that flag is clear.
*   **Conditional Scope Invariant**: When adding or refactoring parameter checks in `SYSCALL_DEFINE*` handlers or helper functions, never read, compare, or perform arithmetic on a flag-gated parameter outside the conditional scope of its governing flag check.

## 2. Core API Overview & User-Space Interaction

*   **`copy_from_user(void *to, const void __user *from, unsigned long n)`**: Copies data from user space to kernel space. If the copy is partial (returns non-zero), this function zero-pads the remainder of the destination buffer.
*   **`copy_to_user(void __user *to, const void *from, unsigned long n)`**: Copies data from kernel space to user space.
*   **`copy_struct_from_user(void *dst, size_t ksize, const void __user *src, size_t usize)`**: The preferred interface for extensible syscall structures. It handles versioning by zero-padding if `usize < ksize` and returning `-E2BIG` if `usize > ksize` (with trailing user bytes checked for non-zero values).
*   **`get_user(x, ptr)` / `put_user(x, ptr)`**: Optimized accessors for simple types.

### Ownership & Lifecycle
*   **No Ownership Transfer**: These functions do not transfer ownership of memory.
*   **Partial Failure**: If a copy fails, the return value is the number of bytes *not* copied. A non-zero return must be treated as an error. Never assume that a partial copy leaves the destination in a valid state unless the specific API guarantees zero-padding.

## 3. Calling Context, Locking & Synchronization

*   **Sleeping Constraints**: `copy_from_user` and `copy_to_user` may sleep. They **must not** be called from atomic context (interrupt handlers, `spin_lock` critical sections, or with `preempt_disable()`).
*   **Atomic Access**: If access is required in atomic context, use `__copy_from_user_inatomic()` or `__copy_to_user_inatomic()`. These do not sleep and require the caller to ensure user pages are resident (e.g., via `fault_in_readable()`).
*   **Context Verification**: Accessing user memory from a kernel worker thread without a valid `mm_struct` will trigger a kernel oops.

## 4. High-Frequency Bug Patterns & Review Invariants

### Memory Safety & Bounds
*   **TOCTOU (Time-of-Check to Time-of-Use)**: Always copy user-provided parameters into a local kernel-stack or heap variable *before* validation. Never validate a pointer and then perform a second `copy_from_user` on the same user address.
*   **Information Leaks**: 
    *   Always zero-initialize structures before copying them to user space to prevent leaking kernel stack/heap data.
    *   When using `copy_from_user` for partial updates, ensure the destination is fully zeroed first.
*   **Integer Overflows**: Validate all user-provided sizes against a reasonable maximum. Use `size_t` for all length calculations.

### Review Checklist for `SYSCALL_DEFINE`
1.  **Flag-Gated Parameters**: Are flag-gated fields accessed only after the flag is verified?
2.  **User-provided size**: Is it validated against `PAGE_SIZE` or specific struct limits?
3.  **Zero-initialization**: Is the destination buffer zero-initialized to prevent leaks?
4.  **Extensible Structs**: Is `copy_struct_from_user` used for versioned structures?
5.  **Return Value**: Is the copy return value checked and converted to `-EFAULT` on failure?
6.  **Context**: Is the code free of `spin_lock` or `preempt_disable()` when calling copy functions?

## 5. Summary of Mandatory Invariants
| Bug Class | Mandatory Guard |
| :--- | :--- |
| **Information Leak** | Zero-initialize structs before `copy_to_user`. |
| **TOCTOU** | Copy user data to local kernel buffer before validation. |
| **Atomic Context** | Use `_inatomic` variants only if `preempt_disable()` or locks are held. |
| **Flag-Gated Data** | Access only within the governing flag's conditional scope. |
| **Partial Copy** | Treat non-zero return as `-EFAULT`; do not proceed with partial data. |
