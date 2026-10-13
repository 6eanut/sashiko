# Universal Subsystem Prompt Generation Questions

The following version-agnostic questionnaire is designed to extract the critical architectural rules, synchronization contracts, and lifecycle invariants from any kernel subsystem source tree. By answering these questions, the resulting documentation will directly address the root causes of the most common historical kernel vulnerabilities.

## 1. Data Synchronization, Lock Hierarchy, and Concurrent Access Serialization

**Target Bug Class & Extraction Mechanics:** 
This section targets deadlocks, lock inversions, data races, and illegal sleeping in atomic contexts. The generated documentation must extract the strict lock acquisition ordering rules, define the boundaries of read-copy-update protection, and explicitly map which execution contexts permit blocking operations versus those that strictly require busy-wait serialization.

1. How is the strict acquisition order defined when multiple mutual exclusion locks must be held simultaneously to prevent circular dependency deadlocks?
2. Which specific data structures require read-copy-update protection, and how are their read-side critical sections bounded to prevent use-after-free vulnerabilities during concurrent updates?
3. What are the exact rules for distinguishing between execution contexts that permit sleeping and those that strictly require busy-wait locks or atomic operations?
4. How are per-processor data structures protected against preemption and thread migration during multi-step read-modify-write sequences?
5. When a lock must be acquired from both hardware interrupt handlers and standard thread contexts, what mechanisms ensure local interrupts are masked to prevent reentrancy deadlocks?
6. How does the subsystem serialize access to shared hardware registers when multiple independent configuration threads attempt concurrent modifications?
7. What is the protocol for safely dropping a highly contended lock to perform a blocking operation and subsequently re-acquiring it without violating state invariants?

## 2. Object Graph Lifetimes, Reference Counting, and Destruction Ordering

**Target Bug Class & Extraction Mechanics:**
This section targets use-after-free vulnerabilities, memory leaks, unbalanced reference counts, and double-free errors. The documentation must extract the ownership semantics of shared objects, the parent-child teardown dependencies, and the exact sequence of reference decrements required to safely transition an object to its final memory release phase.

1. What is the exact sequence of reference count decrements required to safely transition a shared object from active use to its final memory release phase?
2. How does the subsystem guarantee that parent objects outlive their dynamically allocated children, especially during asynchronous teardown sequences?
3. When an object is shared between user-space handles and internal kernel queues, which entity holds the ultimate ownership responsibility for memory deallocation?
4. What mechanisms prevent a concurrent thread from obtaining a new reference to an object that has just initiated its destruction sequence?
5. How are circular dependencies in object graphs detected and broken to prevent permanent memory leaks?
6. In cases where object destruction is deferred via asynchronous workers, how is the memory context preserved until the final cleanup routine executes?
7. What is the contract for releasing device tree node references or firmware handles on every possible exit path, including early error returns?

## 3. Initialization Sequencing, Premature Publication, and Error-Path Unwind

**Target Bug Class & Extraction Mechanics:**
This section targets resource leaks on error paths, null pointer dereferences, uninitialized variable usage, and premature registration. The documentation must extract the step-by-step initialization order, the symmetry of error unwinding, the visibility boundaries of newly created objects, and the tracking mechanisms for device-managed resources.

1. At what exact point during the initialization sequence does a newly allocated object become visible to external subsystems, and how is it guaranteed to be fully configured before this publication?
2. How does the error recovery path guarantee a perfectly symmetric rollback of every resource allocated during a partially successful initialization?
3. What is the strategy for ensuring that device-managed resources are not mixed improperly with manually managed allocations, which could lead to double deallocations or leaks?
4. How are hardware-specific constraints and capabilities validated before any memory allocations or state changes occur in the probe sequence?
5. When a sub-component fails to initialize, how are transient hardware busy states distinguished from terminal failures to ensure correct error code propagation?
6. What mechanisms ensure that all local variables and dynamically allocated structures are fully initialized to safe default values before being evaluated in conditional logic?
7. How does the teardown logic safely handle partially constructed objects where some internal pointers may still be uninitialized or null?

## 4. Asynchronous Work, Timers, Interrupts, and Teardown Quiescing

**Target Bug Class & Extraction Mechanics:**
This section targets uncancelled work before memory release, use-after-free in timer callbacks, and interrupt storms. The documentation must extract the quiescent state tracking logic, the protocols for pending work cancellation, the interrupt disabling sequences, and the lifecycle management of deferred execution timers.

1. What is the precise protocol for synchronously canceling all pending deferred work and active timers before the memory backing their execution context is released?
2. How does the subsystem ensure that hardware interrupts are fully masked at the device level before the software interrupt handlers are unregistered?
3. When a processor core is taken offline, how are its local packet queues and deferred execution lists safely drained or migrated without losing data?
4. How are race conditions prevented when a timer callback attempts to re-arm itself concurrently with a subsystem teardown request?
5. What state tracking mechanisms are used to ignore or safely discard asynchronous events that arrive after the logical disconnection of a device?
6. How does the driver handle interrupt storms or spurious hardware events without exhausting system memory or locking up the processor?
7. In multi-stage asynchronous pipelines, how is the completion signal propagated back to the originating thread when an intermediate stage encounters a fatal error?

## 5. Power Management, Runtime Suspend-Resume, and Resource Balance

**Target Bug Class & Extraction Mechanics:**
This section targets unbalanced power references, hardware state loss, and sleeping in atomic context during resume operations. The documentation must extract the power state transition rules, clock gating requirements, context save and restore procedures, and the handling of wake-up events.

1. How does the subsystem ensure that all required hardware clocks and power domains are fully enabled before any register access is attempted?
2. What is the exact sequence for saving volatile hardware context into persistent memory before transitioning to a low-power state?
3. How are concurrent requests from user-space serialized against asynchronous runtime power management transitions?
4. What mechanisms guarantee that power management reference counts are perfectly balanced across all success and error paths to prevent devices from remaining permanently active?
5. How does the resume sequence handle transient hardware failures or timeout conditions without leaving the subsystem in an inconsistent state?
6. What restrictions exist regarding memory allocation flags and blocking operations during the execution of system-wide suspend and resume callbacks?
7. How are wake-up interrupts configured and acknowledged to ensure the system successfully transitions out of a suspended state without dropping events?

## 6. Dynamic Reconfiguration, Hotplug, Reset, and Stale State Cleanup

**Target Bug Class & Extraction Mechanics:**
This section targets subsystem state machine violations, stale pointer dereferences, and incorrect hardware register configuration. The documentation must extract the state machine invariants, hotplug serialization protocols, hardware reset sequencing, and the rules for invalidating stale caches.

1. What is the strict sequence of operations required to safely reset a hardware pipeline without dropping in-flight transactions or corrupting shared memory?
2. How does the state tracking logic account for unexpected physical removal of a device while active data transfers are still pending?
3. When dynamic reconfiguration requests alter the size of active queues or buffers, how is the transition synchronized with concurrent data processing threads?
4. What mechanisms ensure that all stale configuration data, routing tables, and cached capabilities are completely purged before a device is re-initialized?
5. How are hardware register values verified against the specification for all operating modes during a dynamic capability switch?
6. What is the protocol for notifying upper-layer subsystems that a previously advertised feature is no longer available due to a hardware reset?
7. How does the driver state machine handle implicit exits or aborted transitions when a hotplug event interrupts a multi-step configuration sequence?

## 7. Untrusted Input Boundaries, Sizing Arithmetic, and Buffer Validation

**Target Bug Class & Extraction Mechanics:**
This section targets out-of-bounds memory access, integer overflows, missing bounds checks, and memory safety violations. The documentation must extract the input validation rules, arithmetic overflow prevention strategies, buffer sizing logic, and the accounting for protocol-specific overhead.

1. How are all user-provided offsets, lengths, and array indices strictly validated against the current dynamic limits of the target object before any memory access occurs?
2. What specific arithmetic safeguards are employed to prevent integer overflows when calculating memory allocation sizes derived from external inputs?
3. How does the buffer allocation logic account for all protocol-specific headers, footers, and alignment padding required by the hardware?
4. When parsing complex nested structures from untrusted sources, how does the subsystem prevent infinite loops or excessive recursion?
5. What is the strategy for ensuring that statistics counters and telemetry data are correctly adjusted to exclude software-injected protocol overhead?
6. How are string inputs and variable-length arrays guaranteed to be properly terminated and bounded before being copied into internal kernel structures?
7. What mechanisms verify that memory addresses provided by hardware direct memory access descriptors fall within safely mapped physical memory regions?

## 8. Cross-Layer Callbacks, Reentrancy, and Subsystem Boundary Contracts

**Target Bug Class & Extraction Mechanics:**
This section targets application programming interface contract misuse, incorrect error code propagation, missing return value checks, and deadlocks due to nested locking. The documentation must extract the callback context rules, error code translation requirements, reentrancy guarantees, and layer separation contracts.

1. What are the strict execution context requirements for cross-layer callbacks, specifically regarding whether they are permitted to sleep or must execute atomically?
2. How are internal hardware-specific error conditions translated into standard negative error codes before being propagated across subsystem boundaries?
3. When a lower-layer function returns an error pointer sentinel, how is this condition explicitly distinguished from a null pointer before the result is evaluated?
4. What mechanisms prevent circular locking dependencies when a callback invokes a function in a higher layer that subsequently attempts to re-enter the lower layer?
5. How does the subsystem guarantee that all required library dependencies and exported symbols are explicitly satisfied by the build configuration system?
6. What is the contract for ensuring that structures shared between user-space and kernel-space are explicitly padded to maintain consistent alignment across different processor architectures?
7. When registering a callback function table, how are optional hooks identified, and what is the protocol for validating them prior to execution?
