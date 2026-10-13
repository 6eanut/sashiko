# BPF Verifier & Kfunc Subsystem Context (`kernel/bpf/`)

## 1. Kfunc Scalar and Enum Argument Validation (`kernel/bpf/verifier.c`)

In `check_kfunc_args()`, parameters matching `btf_type_is_scalar(t)` (which includes `int`, `u32`, and C `enum` types) are checked only to verify that the register holds a `SCALAR_VALUE`.

- **No enum range validation by the verifier:** The verifier does **not** check that an `enum` argument falls within the declared enum constants. Unless the parameter has a `__const` suffix annotation (which forces `tnum_is_const()` at verification time), a BPF program can pass any arbitrary scalar register value (from map lookups or arithmetic).
- **Mandatory bounds checks in kfuncs:** Any `__bpf_kfunc` that uses a non-`__const` integer or `enum` parameter to index an array must validate both upper and lower bounds at runtime. Because C `enum` types may be signed `int`, a single `idx >= MAX` check is bypassed by negative values; cast to `unsigned int` (`(unsigned int)event >= MAX`) or check `idx < 0 || idx >= MAX`.

## 2. Subsystem Architecture, Core Data Structures & Object Lifetimes

### Core Data Structures
*   **`struct bpf_prog`**: Primary container for BPF programs. Managed via `bpf_prog_alloc()` and `bpf_prog_free()`.
*   **`struct bpf_lru`**: LRU list management for maps. Uses `rqspinlock_t` for synchronization.
*   **`struct bpf_lru_node`**: Storage unit in LRU lists. The `pending_free` flag must be checked during `flush`/`shrink` operations to prevent illegal `del_from_htab()` calls.
*   **`struct bpf_mem_alloc`**: Global memory allocator (`bpf_global_ma`) for BPF objects.

### Object Lifetimes & Reference Counting
*   **Ownership**: BPF objects (maps, programs, links) are reference-counted. Use `bpf_prog_inc()`/`bpf_prog_put()` and `bpf_map_inc()`/`bpf_map_put()`.
*   **Teardown**: Objects must be fully quiesced before destruction. `bpf_lru_destroy()` must be called only after all nodes are removed or the map is unreachable.
*   **Transient States**: Iterators (e.g., `bpf_iter`) must handle objects in transient states. Accessing a link or map before `bpf_link_init()` completes leads to UAF.

## 3. Synchronization Model, Lock Hierarchy & Concurrency Rules

### Locking Invariants
*   **`rqspinlock_t`**: Used in `bpf_lru_list` and `bpf_lru_locallist`.
*   **Atomic Context**: BPF programs often run in atomic contexts (`rcu_read_lock` held). Do not call `kmalloc(..., GFP_KERNEL)` or any function that can sleep unless the kfunc is marked `KF_SLEEPABLE`.
*   **RCU**: All map lookups and program access must be protected by `rcu_read_lock()`.

### Lock Hierarchy
*   **`btf_idr_lock`**: Global lock for BTF ID management. Must be acquired before accessing `btf_idr`.
*   **Per-CPU Lists**: `bpf_lru_locallist` uses per-CPU locks. Mask local interrupts when performing cross-CPU list manipulation.

## 4. Initialization, Registration & Error Unwind

### Initialization Sequencing
*   **Publication**: Objects must be fully initialized (refcount at 1) before insertion into global lists or IDR.
*   **Symmetric Unwind**: Every `bpf_prog_alloc` must have a corresponding `bpf_prog_free` on all error paths. Use `goto` label patterns (e.g., `err_free_prog: bpf_prog_free(prog);`).

### Error Handling
*   **Sentinel Values**: Distinguish between `NULL` and `ERR_PTR()`. Use `IS_ERR()` before dereferencing helper return values.
*   **Partial Initialization**: If `bpf_lru_init()` fails, ensure `bpf_lru_destroy()` is not called on an uninitialized `bpf_lru` struct.

## 5. Integer & Bounds Safety

*   **Overflows**: Use 64-bit arithmetic (`u64`) for map sizes or memory offsets to prevent overflow before `kmalloc`.
*   **Sign Extension**: BPF registers are 64-bit. When comparing 32-bit keys, use `u32` casts to ensure zero-extension and prevent sign-extension bypasses.
*   **Rescheduling**: Long-running loops in BPF batch operations must include `cond_resched()` to prevent soft lockups.
*   **Zero-Initialization**: Always zero-initialize structures (e.g., `memset(&attr, 0, sizeof(attr))`) before passing to syscalls.

## 6. Untrusted Input Boundaries

*   **User-Provided Offsets**: Validate all `offset` and `len` fields from `union bpf_attr` against the target object's `max_entries` or `size`.
*   **BTF Metadata**: Validate that BTF type sizes are non-zero to prevent division-by-zero in verifier type-walking logic.
*   **Alignment**: Ensure structures copied to/from user space are explicitly padded to maintain consistent alignment across architectures.
