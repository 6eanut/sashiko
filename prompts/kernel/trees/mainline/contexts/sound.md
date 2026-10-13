# Linux Sound Subsystem Code Context Guide (v7.3-rc5)

This guide provides authoritative technical invariants for the Linux Sound subsystem (`sound/`, `include/sound/`). It is designed to prevent architectural, synchronization, and lifecycle errors in the v7.3-rc5 kernel.

---

## 1. Subsystem Architecture, Core Data Structures & Object Lifetimes

The Sound subsystem is hierarchical, anchored by `struct snd_card`.

*   **Ownership & Teardown:** `struct snd_card` manages the lifecycle of all associated components. When `snd_card_free()` is called, it triggers a recursive teardown. **Invariant:** Never manually `kfree()` a component registered to a `snd_card`; use `snd_device_free()` or allow the card's release path to handle it.
*   **Component Registration:** Components are registered via `snd_device_new()`. The `ops` structure must define the `dev_free` callback. Failure to define `dev_free` for custom components causes memory leaks during card removal.
*   **Reference Counting:** Use `snd_card_ref()` and `snd_card_unref()` when passing card pointers across asynchronous boundaries. Every successful acquisition of a reference count must have a corresponding release on all possible error exit paths.

## 2. Synchronization Model, Lock Hierarchy & Concurrency Rules

*   **Lock Hierarchy:**
    1.  `card->controls_rwsem`: Protects the control element list.
    2.  `pcm->open_mutex`: Protects PCM stream state transitions.
    3.  `substream->self_group.lock`: Protects individual stream state.
*   **Atomic vs. Sleeping Contexts:**
    *   **Control API:** `kcontrol->put()` and `get()` callbacks may be invoked under spinlocks. **Rule:** Do not perform blocking operations (e.g., `mutex_lock`, `msleep`) inside `kcontrol` callbacks.
    *   **PCM Operations:** `trigger()` callbacks are called in atomic context. **Rule:** Use `spin_lock()` and avoid any operation that can sleep.
*   **Interrupt Masking:** When accessing shared hardware registers (e.g., `ACP_DMA_CNTL_0`), use `spin_lock_irqsave()` to prevent reentrancy deadlocks between process-context configuration and hardware interrupt handlers.

## 3. Initialization, Registration & Error Unwind

*   **Premature Publication:** A `snd_card` must be fully initialized before `snd_card_register()` is called. Registering before initializing internal components allows user-space to access uninitialized devices, leading to NULL pointer dereferences.
*   **Symmetric Unwind:** Use the `devres` (managed) API where possible. If manual allocation is required, ensure the error path follows the reverse order of allocation.
*   **IS_ERR vs NULL:** Many sound functions return `ERR_PTR` on failure. **Rule:** Always use `IS_ERR()` before checking for `NULL`. Distinguish between a successful zero-length result and a failure condition.

## 4. Asynchronous Work, Timers & Teardown Quiescing

*   **Quiescing:** Before freeing any structure that uses `work_struct` or `timer_list`, you **must** call `cancel_work_sync()` or `timer_delete_sync()`.
*   **Race Conditions:** Ensure that the hardware is quiesced (e.g., `snd_pcm_stop()`) *before* the teardown of the associated URB or DMA buffer to prevent use-after-free during callbacks.
*   **Asynchronous Safety:** Ensure that asynchronous objects (e.g., URBs) are not freed while the hardware or HCD layer still holds a reference.

## 5. Power Management & Hardware State

*   **Runtime PM:** Use `pm_runtime_get_sync()` and `pm_runtime_put_autosuspend()`. **Rule:** Never access hardware registers without holding a runtime PM reference.
*   **State Save/Restore:** During `suspend`, save volatile register states to the `snd_card` private data. Restore them in `resume` before re-enabling the hardware.

## 6. Untrusted Input, Sizing & Arithmetic Integrity

*   **Bounds Validation:** User-provided indices (e.g., `kcontrol` element IDs) must be validated against array bounds.
*   **Integer Overflows:** When calculating buffer sizes (e.g., `period_bytes * periods`), use `size_mul()` or `check_mul_overflow()` to prevent integer wrapping.
*   **Arithmetic Precedence:** All complex arithmetic expressions using bitwise operators must be parenthesized (e.g., `(val & MASK) >> SHIFT`) to ensure correct order of operations.
*   **Format Strings:** Never pass user-supplied strings directly to logging functions. Use `%s` with a literal format string.

## 7. High-Frequency Bug Patterns & Guards

| Bug Pattern | Prevention Strategy |
| :--- | :--- |
| **Unbalanced Refcount** | Ensure every `snd_card_ref()` has a corresponding `snd_card_unref()` on all exit paths. |
| **Missing `of_node_put`** | If using `of_parse_phandle`, call `of_node_put()` on error paths. |
| **Uninitialized Variables** | Always zero-initialize structs (e.g., `struct snd_pcm_hw_params params = {};`). |
| **Missing Change Notification** | `kcontrol->put()` must return `1` if the value changed, `0` otherwise. |
| **Macro Precedence** | Enclose all macro arguments in parentheses to prevent operator precedence issues. |

## 8. Domain-Specific Invariants (Storage/DMA)

*   **DMA Alignment:** Ensure buffers are aligned to the hardware's required boundary.
*   **Register Access:** Always use `readl()`/`writel()` for memory-mapped I/O. Do not use direct pointer dereferencing.
*   **Hardware Quiesce:** Before device removal, ensure all DMA descriptors (e.g., `ACP_DMA_DSCR_CNT_0`) are cleared and the DMA controller is stopped to prevent hardware writes to freed memory.
*   **Module Loading:** Export device ID tables to ensure the module can be automatically loaded by the kernel's device matching logic.
*   **Context Safety:** In `snd_miro_aci` contexts, ensure `aci_mutex` is held during command execution (`snd_aci_cmd`) to prevent concurrent register access.
