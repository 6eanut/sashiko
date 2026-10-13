# Memory Management, Page Cache, and Virtual Memory — Subsystem Prompt Generation Questions

This guide provides version-agnostic, identifier-free probing questions designed to extract critical architectural rules, state machine invariants, and concurrency contracts from Linux kernel source trees. These questions target the most frequent and severe bug classes in the memory management, page cache, and virtual memory domains.

### 1. Allocation Contexts and Concurrency Constraints
**Bug Class Target:** Sleeping in non-preemptible contexts, lock inversion, and race conditions on shared memory state. 
**Extraction Mechanics:** The generated documentation must extract the locking context of callers, the specific memory allocation constraints required for interrupt handlers, and the synchronization primitives protecting shared memory structures and page flags.

1. Does the memory allocation path strictly avoid operations that might sleep when executing within a non-preemptible or interrupt context?
2. Are all updates to shared memory flags and page states performed using atomic operations when accessed concurrently or from interrupt handlers?
3. Does the implementation prevent lock inversion by strictly adhering to the documented lock acquisition hierarchy when traversing memory hierarchies?
4. Are appropriate memory barriers utilized when publishing newly initialized memory objects to ensure their state is fully visible to concurrent readers?
5. Does the long-running memory scanning loop periodically report quiescent states to the read-copy-update subsystem to prevent stalling system grace periods?
6. Are all shared data structures modified by multiple threads protected by appropriate synchronization primitives rather than relying on implicit execution ordering?
7. Is the locking strategy robust against intermediate states where memory pointers might be temporarily null during concurrent object initialization?

### 2. Resource Teardown and Error Unwinding
**Bug Class Target:** Memory leaks, unbalanced reference counts, missing cleanup on allocation failure, and improper error code propagation.
**Extraction Mechanics:** The documentation must detail the exact sequence of error-path labels, the reference counting semantics for each memory object type, and the distinction between null pointers and error pointer sentinels in return values.

8. Does the error handling path explicitly release all memory and hardware resources in the exact reverse order of their acquisition?
9. Are reference counts strictly decremented on every possible error exit path to prevent memory leaks and un-reclaimable objects?
10. Does the code correctly distinguish between a null pointer and an error pointer sentinel when validating the return value of a memory allocation or lookup?
11. Are error variables explicitly reset to a success status after a transient memory failure has been successfully mitigated in a retry loop?
12. Does the teardown logic ensure that all child devices and device-managed resources are fully cleaned up during a probe failure?
13. Are error codes from internal memory operations accurately propagated to the caller instead of being silently collapsed into a generic failure code?
14. Does the error handling path avoid excessive logging that could lead to log flooding when a recurring memory allocation failure is triggered by user space?

### 3. Object Lifecycle and Asynchronous Work
**Bug Class Target:** Use-after-free vulnerabilities, premature exposure of memory objects, and uncancelled background tasks.
**Extraction Mechanics:** The documentation must map out the lifecycle phases of memory objects, the cancellation protocols for background workers, and the visibility boundaries during initialization and teardown.

15. Are all asynchronous background tasks, timers, and delayed work items explicitly cancelled and drained before the associated memory resources are destroyed?
16. Is the memory object fully initialized and in a consistent internal state before it is registered and made visible to concurrent subsystems?
17. Does the resource release logic verify that the memory object is no longer tracked by any active cache or subsystem list before freeing it?
18. Are pointers to internal object states explicitly invalidated or cleared immediately after the object is removed from its primary management structure?
19. Does the charging logic for memory control groups correctly resolve the target owner even when the original owning task is actively being torn down?
20. Is the validity of a cached memory object explicitly verified before attempting to use it in a subsequent operation after a potential state change?
21. Does the subsystem ensure that transient loop counters and temporary states are kept as local variables rather than persisting in long-lived device structures?

### 4. Page Table, Cache, and Hardware State Management
**Bug Class Target:** Stale hardware state, incorrect cache invalidation, unaligned memory access, and direct pointer dereferencing of hardware structures.
**Extraction Mechanics:** The documentation must extract the architecture-specific accessors used for page tables, the cache invalidation ranges, and the synchronization mechanisms for hardware-shared memory.

22. Are architecture-specific accessors strictly used instead of direct pointer dereferencing to ensure compatibility with hardware-specific page table formats?
23. When performing cache maintenance on memory regions, is the range size correctly calculated to cover the entire affected area without missing boundary bytes?
24. Is the dirty state of the underlying memory correctly synchronized and propagated when tearing down or modifying page table entries?
25. Are memory addresses passed to hardware page table walkers properly aligned to the required page boundary?
26. Are hardware configuration registers explicitly reset to a known-good state upon the failure of a memory calibration or tuning sequence?
27. When dealing with direct memory access aliases, are cache invalidations performed using the correct hardware requester identifier for each specific alias?
28. Are atomic operations used when reading and modifying shared hardware-managed page table entries to prevent lost updates from concurrent hardware modifications?

### 5. Memory Safety, Bounds, and Arithmetic Integrity
**Bug Class Target:** Out-of-bounds accesses, integer overflows in size calculations, uninitialized memory usage, and false sharing.
**Extraction Mechanics:** The documentation must capture the arithmetic bounds checking logic, the loop termination conditions for memory iteration, and the initialization guarantees for newly allocated structures.

29. Are all arithmetic operations involving user-provided offsets, lengths, and page frame numbers checked for potential integer overflows against the system memory layout?
30. Does the loop termination condition correctly account for the size of compound memory objects when iterating over granular entries?
31. Are all fields in a newly allocated memory structure explicitly set to a known safe state before the structure is evaluated by conditional logic?
32. Does the iteration logic correctly traverse all bits in a memory bitmask rather than prematurely stopping at the first matched bit?
33. Are frequently accessed, independent memory structures properly aligned and padded to avoid performance degradation from false sharing of cache lines?
34. Are bit flags for memory state tracking placed safely outside the bit range of any counters they are embedded within?
35. Does the implementation strictly adhere to documented behavior and apply proper clamping for boundary or zero-value inputs during memory sizing?

### 6. Subsystem Invariants and State Machine Transitions
**Bug Class Target:** Logical flaws in memory accounting, incorrect handling of special memory types, and invalid state transitions.
**Extraction Mechanics:** The documentation must extract the rules for global resource accounting, the fallback logic for memory reclaim, and the preconditions for virtual memory state machine transitions.

36. Is resource accounting correctly balanced when a memory operation creates a new object without immediately destroying the source object?
37. Does the memory reclaim logic correctly handle scenarios where the requested memory type is unavailable, preventing unintended fallbacks to restricted memory zones?
38. Are all state machine flags and action indicators explicitly reset to their baseline state before entering a retry loop for a failed memory operation?
39. Do complex virtual memory operations maintain all internal invariants regarding address space boundaries and page offsets after completing a state transition?
40. Are feature-specific memory tuneables strictly restricted to objects and architectures that actually support the underlying hardware mechanism?
41. Does the interface design ensure that per-session or per-file memory states are strictly isolated from other concurrent users to prevent state corruption?
42. Are global memory resource counters updated consistently across all execution paths that modify the tracked objects, including error and fast-path exits?
