# BPF Kfunc & Skeleton API Guide (Linux v7.3-rc5)

This guide provides authoritative rules for BPF kfunc usage, skeleton lifecycle management, and verifier invariants. It is derived from `include/linux/btf.h`, `tools/lib/bpf/libbpf.h`, and empirical bug patterns identified in the Linux v7.3-rc5 development cycle.

---

## 1. Kfunc Reference & Ownership (`include/linux/btf.h`)
The BPF verifier enforces a strict ownership model for kernel objects accessed via kfuncs using BTF annotations.

### Reference Semantics
*   **Acquisition (`KF_ACQUIRE`):** Kfuncs marked with `KF_ACQUIRE` return an "owning reference." The BPF program is responsible for this reference.
*   **Release (`KF_RELEASE`):** Every acquired reference **must** be passed to a `KF_RELEASE` kfunc before program termination. Failure results in verifier rejection.
*   **Trusted Pointers:** Pointers obtained from `KF_ACQUIRE` or specific tracepoint/struct_ops arguments are "trusted."
    *   **Invariant:** A trusted pointer remains valid only as long as it is not modified or "walked" (e.g., `task->last_wakee` is NOT trusted even if `task` is).
    *   **Bug Prevention:** Passing a walked pointer to a `KF_RELEASE` or `KF_ACQUIRE` kfunc is a source of use-after-free (UAF) and is strictly rejected.
*   **RCU Lifecycle (`KF_RCU`, `KF_RCU_PROTECTED`):** 
    *   `KF_RCU` indicates the kfunc accepts RCU-protected pointers.
    *   `KF_RCU_PROTECTED` requires invocation within an RCU read-side critical section.
    *   **Critical Bug Pattern:** When an RCU-protected pointer transitions to an "untrusted" state (e.g., after `rcu_read_unlock`), the verifier must invalidate metadata. Failure to clear "non-owning reference" flags leads to UAF (Fix [cd6f72d7f38e]).

## 2. Execution Context & Constraints
BPF programs must respect execution boundaries (NMI, IRQ, Task) defined by kfunc flags.

### Context Flags
*   **`KF_SLEEPABLE`:** Indicates a kfunc may yield (e.g., I/O or mutex acquisition).
    *   **Constraint:** Can **only** be called from BPF programs marked sleepable (e.g., `SEC("syscall")`, `SEC("fentry.s/")`).
    *   **Bug Pattern:** Calling faultable helpers from atomic contexts (Fix [9d02927fdf4e], [d05524794240]).
*   **`KF_SPINLOCK_SAFE`:** Allows the kfunc to be called while holding a `bpf_spin_lock`. Most kfuncs are forbidden in spinlock regions to prevent deadlocks.
*   **`KF_DESTRUCTIVE`:** Requires `CAP_SYS_BOOT`. These functions can crash or halt the system and are restricted to administrative contexts.
*   **Context Propagation:** A subprogram called from a sleepable context must still be verified against the constraints of its own definition (Fix [369f4ce73457]).

## 3. Return Values & Error Handling
### Pointer Returns
*   **`KF_RET_NULL`:** The kfunc may return NULL.
    *   **Mandatory Check:** The verifier forces an `if (!ptr)` check. Accessing the pointer before this check results in "R0 invalid mem access".
    *   **Bug Pattern:** Missing precision tracking for NULL. If the verifier fails to mark a register as precise when NULL, it may incorrectly prune branches (Fix [e726fc6b9afe]).
*   **Error Codes:** BPF kfuncs typically return `NULL` for pointer failures and negative integers (e.g., `-EINVAL`) for scalar failures. **Never** use `IS_ERR()` inside a BPF program; check for `NULL` or `< 0`.

### Teardown Symmetry
*   **Skeleton Cleanup:** Use `<skel>__destroy(skel)` to release all maps, programs, and links.
*   **Recursion Guard:** Object destruction logic must prevent unbounded recursion (e.g., self-referential maps) which exhausts the kernel stack (Fix [bfc888f04588]).

## 4. Memory Integrity & Scalar Validation
### Scalar/Enum Array Indexing (`include/linux/btf.h`)
*   **Unconstrained Scalars:** Non-`__const` scalar or `enum` parameters passed to a `__bpf_kfunc` are unconstrained `SCALAR_VALUE` registers at runtime.
*   **Invariant:** Programs **must** perform explicit bounds-checks (including negative value checks for signed integers/enums) before using these parameters to index kernel arrays.
*   **Constant Enforcement:** Parameters with the `__const` suffix are enforced by the verifier to be compile-time constants (`tnum_is_const()`).

### Concurrency & Safety Invariants
*   **Sign Extension:** Ensure 32-bit values are not incorrectly sign-extended to 64-bit during map lookups or bounds checks, which allows large unsigned values to bypass safety (Fix [70504de0bb62]).
*   **Atomic Operations:** 32-bit atomic operations (e.g., `cmpxchg`) targeting `arena` memory (`KF_ARENA_RET`) must be zero-extended to 64-bit to maintain register integrity (Fix [4814ed6406f3]).
*   **Buffer Validation:** All user-provided buffer size validations must be performed **before** accessing any fields within that buffer (Fix [a11212910cf0]).
*   **Precision Tracking:** Security-critical scalar values must be marked as "precise" in the verifier (Fix [ecdc5043794c]).
*   **State Equivalence:** When comparing execution states, all context-specific identifiers (e.g., stack frame numbers) must be included to ensure state uniqueness (Fix [8901cee9316d]).

## 5. BPF Skeleton Lifecycle (`bpftool gen skeleton`)
### Success Invariants
*   After `<skel>__open_and_load()` returns a valid pointer (verified via `ASSERT_OK_PTR(skel, ...)`), **all** skeleton program pointers (`skel->progs.<name>`) and map pointers (`skel->maps.<name>`) are guaranteed to be loaded and hold valid file descriptors (`>= 0`).
*   **False Positive Guard:** Do **NOT** flag missing FD checks on `bpf_program__fd(skel->progs.<name>)` or `bpf_map__fd(skel->maps.<name>)` after a successful `<skel>__open_and_load()`.

### Manual Lookup Exception
*   Manual lookup APIs (`bpf_object__find_program_by_name()`, `bpf_object__find_map_by_name()`) can return `NULL` even if the object is loaded. These **always** require pointer and FD checks.

## 6. Summary of Kfunc Flags (`v7.3-rc5`)

| Flag | Meaning | Verifier Action |
| :--- | :--- | :--- |
| `KF_ACQUIRE` | Returns a reference | Must be paired with `KF_RELEASE` |
| `KF_RELEASE` | Consumes a reference | Invalidates the pointer register |
| `KF_RET_NULL` | May return NULL | Forces an `if (!ptr)` check |
| `KF_SLEEPABLE` | May sleep | Only allowed in sleepable BPF progs |
| `KF_RCU` | Accepts RCU pointers | Validates RCU read-side or trusted status |
| `KF_FASTCALL` | Optimized calling | Skips certain register spills |
| `KF_ARENA_RET` | Returns arena pointer | Marks register as `PTR_TO_ARENA` |
| `KF_SPINLOCK_SAFE` | Spinlock compatible | Allowed in `bpf_spin_lock` regions |
| `KF_DESTRUCTIVE` | Requires `CAP_SYS_BOOT` | Restricts to admin contexts |
