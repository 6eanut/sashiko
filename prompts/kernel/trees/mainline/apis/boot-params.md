# Kernel Boot & Module Parameter Registration API Guide (v7.3-rc5)

This guide defines the authoritative usage, lifecycle, and safety invariants for the Linux kernel parameter registration subsystem (`include/linux/init.h`, `include/linux/moduleparam.h`).

## 1. Core API Overview & Registration Macros

*   **Early/Boot Parameters (`include/linux/init.h`):** `__setup(str, fn)` and `early_param(str, fn)` register handlers processed during early kernel boot.
*   **Module/Core Parameters (`include/linux/moduleparam.h`):** 
    *   `module_param(name, type, perm)`: Registers a parameter for a module.
    *   `module_param_named(name, value, type, perm)`: Allows naming the parameter differently from the variable.
    *   `core_param(name, var, type, perm)`: Registers a parameter accessible on the kernel command line.

### Registration Invariants
- **Documentation:** Every new parameter added via these macros **must** be documented in `Documentation/admin-guide/kernel-parameters.txt`. Failure to do so is a defect unless the parameter is strictly gated by `CONFIG_DEBUG_KERNEL` or `CONFIG_EXPERIMENTAL`.
- **Naming:** Parameters are prefixed by `KBUILD_MODNAME` and a dot (e.g., `my_module.param_name`). The macro handles the conversion of `-` to `_` in variable names.
- **Visibility:** The `perm` argument controls `sysfs` visibility. If `perm` is non-zero, the parameter is exported.

## 2. Calling Context, Locking & Synchronization

- **Atomic Context:** Parameter handlers (especially `__setup` and `early_param`) are often called in contexts where memory allocation or sleeping is prohibited. Do not perform blocking operations within these handlers.
- **Sysfs Access:** If a parameter is exported to `sysfs` (`perm != 0`), the kernel does not automatically serialize access to the underlying variable. If the variable is a `charp` (string) or a complex structure, you **must** use `kernel_param_lock()` to prevent race conditions during concurrent `sysfs` writes and module unloading.
- **Initialization Order:** Parameters are processed in a specific order. Ensure your parameter does not require initialization before its dependent subsystems (e.g., IRQs, SMP, or memory allocators) are ready.

## 3. Return Values, Error Sentinels & Unwind Symmetry

- **Handler Return Values:** 
    - `__setup` and `early_param` handlers must return `1` if the parameter was successfully handled, or `0` if it was not. Returning an incorrect value causes the kernel to treat the parameter as "unknown," potentially triggering a panic or warning.
    - `module_param` `set` functions must return `0` on success or a negative `-errno` on failure.
- **Resource Cleanup:** If a parameter handler allocates memory, it must provide a `free` function within the `kernel_param_ops` structure to prevent memory leaks during module unloading.
- **Partial Failure:** If initialization fails, the handler must perform a manual rollback of any partially acquired resources. The kernel does not provide an automatic "undo" for failed parameter registration.

## 4. High-Frequency Bug Patterns & Review Invariants

### API Contract & Return/Context Misuse
- **Incorrect Return Values:** Verify that `__setup` and `early_param` handlers return `1` for success and `0` for unknown parameters.
- **Lazy Initialization:** If a parameter is lazily initialized, ensure the pointer is validated against `NULL` or `IS_ERR()` before use.

### Memory Safety & Resource Leaks
- **Module Unload Leaks:** Ensure that any memory allocated during module parameter initialization is explicitly freed in the module's exit path or via the `kernel_param_ops->free` callback.
- **Bounds Checking:** When parsing strings or arrays, always validate the input length against the destination buffer size to prevent buffer overflows.

### Documentation & ABI
- **Stale Documentation:** When removing a parameter, ensure the corresponding entry in `kernel-parameters.txt` is removed.

## 5. Review Checklist for Patch Submission

1.  **Documentation:** Is `Documentation/admin-guide/kernel-parameters.txt` updated?
2.  **Return Codes:** Does the `__setup` handler return `1` on success?
3.  **Concurrency:** If `perm != 0`, is there a plan for synchronization (e.g., `kernel_param_lock`)?
4.  **Cleanup:** If the module is unloaded, is all memory associated with the parameter freed?
5.  **Validation:** Are all user-provided inputs (lengths, values) validated before being applied to the system state?
6.  **Consistency:** Does the parameter name and behavior match existing conventions for similar drivers/subsystems?
7.  **Ownership:** Are references to shared objects correctly managed during module teardown to prevent use-after-free?
8.  **Context:** Are all operations within the parameter handler safe for the current execution context (e.g., no sleeping in atomic)?
9.  **Idempotency:** Are state transitions during parameter updates idempotent and safe against concurrent hotplug events?
10. **Alignment:** Are buffer sizes and memory alignments validated for hardware-specific parameters?
11. **Registration:** Are `core_param` and `module_param_named` used correctly to expose parameters to the bootloader and sysfs?
