# Device-Managed Resources (Devres) API Guide - Linux v7.3-rc5

The Devres (Device Resource Management) framework provides a mechanism to associate the lifetime of allocated resources (memory, I/O mappings, IRQs, etc.) with a `struct device`. This ensures that resources are automatically released when a driver is detached from a device or when a `probe()` function fails.

## 1. Core API Overview & Ownership Lifecycles

### 1.1 Resource Ownership
In the Devres model, the `struct device` acts as the primary owner. Resources are encapsulated in a `struct devres` container and linked to the device's `devres_head` list.

*   **Allocation:** Resources are allocated via `devres_alloc()` or high-level `devm_*` wrappers.
*   **Registration:** Once allocated, a resource must be added to the device via `devres_add(dev, ptr)`. High-level `devm_` functions perform this step automatically.
*   **Lifetime:** The resource remains valid until the driver is unbound, the `probe()` function returns an error, or the resource is explicitly released via `devm_kfree()` or `devres_release()`.

### 1.2 Reference Counting and Anchoring
Devres uses a Last-In, First-Out (LIFO) stack approach for teardown.
*   **Anchor Device Selection:** Resources must be anchored to the `struct device` that matches the resource's functional lifetime. 
*   **Invariant:** If a resource is accessed by a background worker or an IRQ handler, it must be anchored to a device that outlives those asynchronous contexts. Anchoring to a child device that is destroyed before the parent driver finishes cleanup results in Use-After-Free (UAF).

## 2. Calling Context, Locking & Synchronization

### 2.1 Context Constraints
*   **Standard Allocators:** `devm_kzalloc()` and variants follow `kmalloc` rules. They may sleep if `GFP_KERNEL` is used.
*   **Atomic Context:** `devres_alloc()` and `devm_kzalloc()` are safe in atomic context if `GFP_ATOMIC` is provided. `devres_add()` acquires `dev->devres_lock` (a spinlock), which is safe in atomic context but impacts latency.
*   **Interrupt Handlers:** While `devm_request_irq()` manages the IRQ lifecycle, the handler itself must not call `devm_` allocation functions unless using `GFP_ATOMIC`.

### 2.2 Internal Synchronization
The Devres subsystem protects the device's resource list using `dev->devres_lock`.
*   **Lock Ordering:** `devres_lock` is a leaf lock. It must not be held while calling into subsystems that sleep or acquire high-level mutexes.
*   **Reentrancy:** Release callbacks are invoked with `devres_lock` **released**. This allows release functions to perform complex cleanup or call other `devm_` functions.

## 3. Return Values & Unwind Symmetry

### 3.1 Return Value Conventions
*   **Pointer-based Allocators (`devm_kzalloc`, `devm_kcalloc`):** Return `NULL` on failure.
*   **Resource Helpers (`devm_ioremap_resource`, `devm_clk_get`):** Return `ERR_PTR` values. Use `IS_ERR()` for validation.
*   **Action Registration (`devm_add_action`):** Returns `0` on success or a negative error code (e.g., `-ENOMEM`).

### 3.2 Unwind Symmetry
*   **Automatic Unwind:** On `probe()` failure, Devres releases resources in the reverse order of addition (LIFO).
*   **Manual Unwind:** Use corresponding `devm_` release functions (e.g., `devm_kfree()`, `devm_iounmap()`) for early release.
*   **Action Or Reset:** `devm_add_action_or_reset(dev, action, data)` is the preferred pattern. If registration fails, it executes the `action` immediately to prevent resource leaks.

## 4. High-Frequency Bug Patterns & Review Invariants

### 4.1 The "Double Free" Trap
*   **Invariant:** Never use `kfree()`, `iounmap()`, or `free_irq()` on resources managed by `devm_`. 
*   **Actionable Check:** Verify that any pointer passed to a non-managed free function was not initialized via a `devm_` prefix function.

### 4.2 Asynchronous Work & Use-After-Free
*   **Pattern:** A `delayed_work` or `timer` is scheduled, but the driver is unbound. Devres frees the private data, but the work item triggers later.
*   **Invariant:** All asynchronous tasks (workqueues, timers, threaded IRQs) must be synchronously cancelled **before** the memory they access is freed by Devres.
*   **Requirement:** Use `devm_add_action()` to call `cancel_delayed_work_sync()` or equivalent cleanup before the device-managed memory is released.

### 4.3 Incorrect Error Check (IS_ERR vs NULL)
*   **Invariant:** 
    *   `devm_kzalloc` -> Check `if (!ptr)`
    *   `devm_ioremap_resource` -> Check `if (IS_ERR(ptr))`
*   **Note:** `devm_ioremap_resource` returns an `ERR_PTR`. Checking for `NULL` will fail to catch errors and lead to invalid pointer dereferences.

### 4.4 Ordering of Registration vs. Resource Acquisition
*   **Invariant:** A device must only be registered (e.g., `input_register_device`) **after** all resources required by its callbacks (e.g., `open`, `ioctl`) have been secured via `devm_`.
*   **Failure Case:** If registration succeeds but a subsequent `devm_` allocation fails, the `probe` error path triggers. However, the subsystem may already have a pointer to the device, creating a race during teardown.

### 4.5 Managed Resource Lifetime Mismatch
*   **Invariant:** `devm_` is for resources tied to the **hardware device's presence**, not the **file handle's lifetime**.
*   **Rule:** If a resource must be freed when a file descriptor is closed (e.g., per-open buffers), do not use `devm_`.

### 4.6 Missing Resource Reservation
*   **Invariant:** Prefer `devm_ioremap_resource()` over `devm_ioremap()`.
*   **Reason:** `devm_ioremap_resource()` performs both `devm_request_mem_region()` and `devm_ioremap()`. Raw `devm_ioremap()` fails to reserve the MMIO range in `/proc/iomem`.

## 5. Summary Table of Managed Symbols (v7.3-rc5)

| Resource Type | Allocation/Registration | Manual Release (Optional) | Error Sentinel |
| :--- | :--- | :--- | :--- |
| Memory | `devm_kzalloc()` | `devm_kfree()` | `NULL` |
| IO Memory | `devm_ioremap_resource()` | `devm_iounmap()` | `IS_ERR()` |
| IRQ | `devm_request_irq()` | `devm_free_irq()` | `Integer < 0` |
| Custom Action | `devm_add_action_or_reset()` | `devm_remove_action()` | `Integer < 0` |
| DMA | `dmam_alloc_coherent()` | `dmam_free_coherent()` | `NULL` |
| GPIO | `devm_gpiod_get()` | `devm_gpiod_put()` | `IS_ERR()` |
