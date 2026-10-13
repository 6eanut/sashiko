# Networking Core, Protocols, Wireless, and Network Drivers — Subsystem Prompt Generation Questions

The following version-agnostic meta-prompt questions are designed to extract critical architectural rules, synchronization contracts, and error-handling invariants from Linux networking and driver source trees. These questions target the most frequent sources of regressions in the networking stack, ensuring that generated documentation provides actionable guidance for code reviewers and developers.

## Section 1: Packet Buffer Management and Header Validation
**Bug Class & Extraction Mechanics:** This section targets out-of-bounds memory accesses, corrupted packet headers, and integer underflows caused by assuming a packet buffer has sufficient contiguous linear data. The generated documentation must extract how the subsystem verifies packet length, ensures sufficient linear headroom before parsing or modifying headers, and handles the overhead of stacked devices or tunneling protocols.

1. Does the code verify that the packet buffer contains sufficient linear data for the expected protocol header before attempting to read or modify its fields?
2. When merging or fragmenting packet buffers, does the logic respect all boundary constraints that prevent combining payloads with differing hardware offload properties?
3. Does the headroom calculation explicitly account for the maximum possible size of all headers that might be prepended by underlying stacked devices or tunneling protocols?
4. Are all length fields received from external hardware or untrusted network peers strictly validated against the actual received buffer size before being used as offsets?
5. When stripping headers or pulling data from a packet buffer, is the total packet length validated against the expected minimum size to prevent integer underflow?
6. Does the checksum calculation logic clearly distinguish between the entire packet payload and specific headers that must be excluded for virtual devices?
7. Are all packet parameters that influence hardware descriptor fields validated against hardware-specific constraints at the earliest possible point in the transmit pipeline?

## Section 2: Object Lifecycle, Reference Counting, and Teardown
**Bug Class & Extraction Mechanics:** This section targets use-after-free vulnerabilities and memory leaks stemming from unbalanced reference counts, dangling pointers, or improper teardown sequencing. The generated documentation must extract the exact ownership rules, reference increment and decrement pairs, and the invalidation sequence for shared objects across asynchronous boundaries.

8. Does every function that increments a reference count on a network device or shared object have a guaranteed corresponding path to decrement it, including all error exit points?
9. Are all cached pointers to a shared object immediately invalidated or cleared from persistent structures before the object is formally deallocated?
10. Is the ownership of a packet buffer clearly defined across all asynchronous boundaries to prevent double-frees or memory leaks on transmission failure?
11. When an application programming interface transfers ownership of an object reference to the caller, is there a documented contract for releasing that reference in both success and failure scenarios?
12. Are pointers to internal object extensions or connection tracking entries stored in global lists only after all potential memory reallocations have successfully completed?
13. Does the teardown sequence ensure that an object is protected against new lookups or reference acquisitions before its final destruction phase begins?
14. Are device tree node references or similar hierarchical topology objects explicitly decremented when breaking out of iteration loops or returning early on error?

## Section 3: Concurrency, Synchronization, and Lockless Access
**Bug Class & Extraction Mechanics:** This section targets race conditions, deadlocks, and invalid context execution (such as sleeping in atomic context) caused by missing locks or incorrect lockless annotations. The generated documentation must extract the locking hierarchy, read-copy-update critical sections, and context restrictions (atomic versus sleepable) for shared state variables.

15. Are all shared state variables and lockless pointers accessed exclusively under the protection of the appropriate read-copy-update critical section or explicit lock?
16. Does the code strictly avoid invoking memory allocations that can sleep or blocking operations while holding a spin lock or executing within an atomic context?
17. Are concurrent accesses to shared configuration variables or system control parameters protected by explicit atomic read and write annotations to prevent compiler reordering?
18. Is the reference count of a shared object safely incremented only while the object is still protected by the lookup lock or read-side critical section?
19. Are state transitions and list modifications protected by the exact same synchronization primitive used to guard the list traversal?
20. Does the locking strategy enforce a strict acquisition order to prevent deadlocks when multiple nested locks or hierarchical device locks are required?
21. Are all shared objects between asynchronous workers and the main packet processing path protected by proper serialization?

## Section 4: Error Handling, Resource Unwinding, and Pointer Validation
**Bug Class & Extraction Mechanics:** This section targets resource leaks, silent failures, and null pointer dereferences caused by incomplete rollback paths or misinterpreting return values. The generated documentation must extract the step-by-step resource unwinding order, error code propagation rules, and the specific sentinel values (null versus error pointers) returned by dependencies.

22. Does the error handling path perform a complete, reverse-order rollback of all memory allocations, hardware mappings, and locks acquired during the initialization sequence?
23. Are return values from resource allocators correctly checked against the specific failure sentinel expected by that allocator, strictly distinguishing between null pointers and error pointer sentinels?
24. Are all error return paths correctly propagating the actual failure code to the caller without being accidentally masked by local variable shadowing or default success values?
25. If a hardware probe function fails after allocating direct memory access coherent buffers, are those specific buffers explicitly unmapped and freed in the error path?
26. Are cleanup operations in the error path guarded by the exact same conditional checks that were used to determine if the optional resource was allocated in the first place?
27. Does the driver provide a functional fallback or gracefully degrade rather than failing the entire initialization if a non-critical optional hardware resource is unavailable?

## Section 5: Hardware State Machines, Polling, and Asynchronous Work
**Bug Class & Extraction Mechanics:** This section targets premature device exposure, uncancelled background tasks, and infinite loops on hardware failure. The generated documentation must extract the exact sequence of hardware initialization, the lifecycle of polling mechanisms, and the cancellation guarantees for deferred work before memory is freed.

28. Are all required internal device structures and hardware registers fully initialized before the network interface is registered and exposed to the kernel network stack?
29. Are all asynchronous timers, deferred work items, and per-processor packet queues guaranteed to be synchronously cancelled and drained before the associated object is destroyed?
30. Is the network polling mechanism explicitly stopped and disabled before shutting down hardware queues to prevent race conditions during device suspend or removal?
31. Does the driver handle hardware-reported error values or all-ones register reads by breaking out of polling loops instead of assuming the input is valid and looping infinitely?
32. Are hardware-specific feature checks and capability flags evaluated dynamically when the underlying hardware path changes, such as during a failover event?
33. Does the state machine ensure that hardware-visible transmission descriptors are only marked as ready after all associated memory barriers and resource preparations have completed?
34. Are all hardware register reads explicitly masked to ignore reserved or status bits before the value is used in subsequent arithmetic or logic calculations?

## Section 6: Memory Safety, Boundary Checks, and Data Integrity
**Bug Class & Extraction Mechanics:** This section targets buffer overflows, integer overflows, and uninitialized memory disclosures. The generated documentation must extract the validation rules for user-space inputs, the sizing logic for dynamic arrays, and the zeroing requirements for memory copied across privilege boundaries.

35. Are all arithmetic operations involving user-provided sizes or packet lengths checked for potential integer overflows before being used to allocate memory or calculate offsets?
36. Are all variable-length input buffers and configuration payloads validated for minimum size before the code attempts to read fixed-size protocol fields from them?
37. Are all array indices derived from external network traffic or hardware descriptors validated against the current dynamic limit of the subsystem before being used?
38. Are all structures and buffers that are copied back to user space fully zeroed during initialization to prevent the leakage of uninitialized kernel stack memory?
39. Are all user-provided strings explicitly checked for proper null-termination before being passed to internal lookup functions or string comparison routines?
40. Does the buffer size estimation logic for configuration dumps account for all variable-length components, nested attributes, and alignment padding that might be included in the final output?
41. Are statistics and hardware counters read using explicit fixed-width types rather than relying on architecture-dependent integer sizes that might truncate data?
