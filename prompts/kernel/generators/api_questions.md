# Kernel API Prompt Generation Questions

## 1. Object Ownership, Lifetime, and Reference Counting Semantics
**Targeted Bug Class:** Unbalanced reference counts, double-free memory corruptions, and use-after-free vulnerabilities.
**Extraction Mechanics:** The generated documentation must map how an object's lifecycle is tracked, which subsystem owns the primary reference at each execution stage, and the exact rules for transferring, duplicating, or dropping references across subsystem boundaries.

* When a subsystem lookup yields a shared object, does the caller assume ownership of a new reference that must be explicitly dropped, or is the reference strictly bound to a currently held lock?
* If an initialization sequence fails halfway, what is the exact rollback order to decrement usage counters and release memory without triggering a double release?
* When passing an object to an asynchronous worker, does the handoff transfer the primary reference, or must the caller duplicate the reference before scheduling the work?
* Are there any implicit reference acquisitions hidden within iteration helpers that require manual release if the loop terminates early?
* How does the teardown sequence guarantee that all cached pointers to an object are invalidated before the final memory deallocation occurs?
* In cases where an object is shared between a parent framework and a child driver, which entity is ultimately responsible for the final destruction step?
* When an operation is interrupted by a signal, does the abort path correctly balance all usage counters acquired prior to the interruption?

## 2. Execution Context, Preemption, and Sleeping Constraints
**Targeted Bug Class:** Sleeping in atomic context, invalid lock acquisitions, and context-mismatch panics.
**Extraction Mechanics:** The generated documentation must define the strict rules regarding which functions can yield the processor, what synchronization primitives disable preemption, and how memory allocations must adapt their behavior based on the calling environment.

* Which specific operations within this subsystem are strictly forbidden from yielding the processor, and how do they signal this requirement to their callers?
* If a memory allocation is required while holding a non-sleeping lock, what specific allocation flags or memory pools must be utilized to prevent scheduling?
* Does the execution context of hardware interrupt handlers or network polling mechanisms restrict the types of synchronization primitives that can be acquired?
* When a background worker accesses shared state, does it guarantee that the primary execution thread is not concurrently modifying that state under a different context assumption?
* Are there any hidden blocking operations, such as waiting for hardware completion or acquiring sleeping locks, buried deep within seemingly non-blocking state query functions?
* How does the subsystem handle the need to perform a blocking operation, such as a firmware load, when the initial request arrives in a context that cannot sleep?
* When transitioning between a non-preemptible context and a preemptible one, what state must be explicitly saved and restored to prevent data corruption?

## 3. Return Value Conventions, Error Sentinels, and Partial-Failure Recovery
**Targeted Bug Class:** Mismatched null versus error pointer checks, ignored return values, and resource leaks on error paths.
**Extraction Mechanics:** The generated documentation must extract the exact error signaling conventions (such as integer codes versus pointer error sentinels) and the mandatory cleanup steps required for every failure branch.

* Does the allocation or lookup function signal failure by returning a null pointer, or does it embed an error code within the pointer address that must be explicitly decoded?
* When a multi-step configuration process encounters a failure, does the error recovery path restore the hardware and software state to its exact previous condition, or does it apply a generic reset?
* Are callers required to inspect the return value of a state-update function, and what specific rollback actions are mandated if the update is rejected?
* If a resource acquisition succeeds but a subsequent validation step fails, is the acquired resource released using a specific error-handling label, or must it be handled inline?
* How are error codes propagated from deeply nested hardware interactions back to the user-space boundary without losing the original failure context?
* When an operation can partially succeed, such as a bulk memory transfer, how does the API communicate the amount of completed work alongside the failure reason?
* Are there any legacy interfaces within this subsystem that return success-like values on failure, requiring callers to check secondary status fields?

## 4. Concurrency Guarantees, Caller Serialization, and Memory Ordering
**Targeted Bug Class:** Deadlocks, lock inversions, missing read-copy-update protections, and data races.
**Extraction Mechanics:** The generated documentation must detail the lock hierarchy, the rules for read-side versus write-side critical sections, and the memory barriers required for lockless state publication.

* What is the strict acquisition order for multiple locks within this subsystem to prevent circular dependency deadlocks?
* When iterating over a shared list using read-copy-update mechanisms, what guarantees that the traversed objects are not prematurely destroyed by a concurrent writer?
* Are all modifications to shared state variables protected by a single, consistent synchronization primitive, or do different fields require different locks?
* If a lock is released temporarily to perform a blocking operation, what validation must occur upon reacquiring the lock to ensure the underlying object state has not changed?
* What explicit memory barriers are required when publishing a newly initialized object to a shared data structure to ensure all processors see the correct initial state?
* Does the subsystem rely on the caller to serialize access to a specific interface, or does it manage its own internal mutual exclusion?
* When a hardware event triggers a state change, how is this change synchronized with concurrent software-driven configuration updates?

## 5. Registration, Publication-Before-Initialization, and Synchronous Cancellation
**Targeted Bug Class:** Premature exposure of uninitialized devices, uncancelled background work, and teardown races.
**Extraction Mechanics:** The generated documentation must map the exact sequence of initialization, the precise point of public visibility, and the mandatory synchronization steps required before destruction.

* Is the internal state of the object completely configured and validated before it is registered with the central subsystem and exposed to external consumers?
* During the teardown sequence, what mechanism ensures that all pending asynchronous tasks and background timers are synchronously cancelled before their associated memory is freed?
* If a device is removed unexpectedly, how does the subsystem prevent new requests from being queued while simultaneously draining the existing request backlog?
* When a network interface or block device is brought down, are all hardware polling mechanisms explicitly disabled before the software queues are destroyed?
* Does the registration function advertise capabilities to the broader system only after confirming that the underlying hardware actually supports them?
* How does the subsystem handle concurrent access attempts that occur in the narrow window between object publication and the completion of its final initialization steps?
* Are all dynamically allocated resources associated with a lifecycle object guaranteed to be freed regardless of the object's state transitions during a sudden removal?

## 6. Buffer Sizing, Alignment, Direct Memory Access, and Boundary Validation
**Targeted Bug Class:** Out-of-bounds memory accesses, integer underflows, unvalidated user input, and direct memory access coherency violations.
**Extraction Mechanics:** The generated documentation must extract the rules for buffer headroom reservation, hardware alignment constraints, and input sanitization requirements for network buffers and memory mappings.

* Before modifying a network packet buffer or similar data structure, does the code explicitly verify that sufficient headroom exists for all potential protocol headers?
* Are all user-provided lengths and offsets validated against the maximum capacity of the target buffer to prevent out-of-bounds memory accesses?
* When mapping memory for direct hardware access, does the implementation respect all platform-specific segment size and boundary alignment constraints?
* How does the subsystem ensure that memory shared between the processor and a peripheral device is properly synchronized to prevent reading stale cached data?
* Are all arithmetic operations involved in calculating buffer boundaries protected against integer underflow or overflow before being used as memory offsets?
* When parsing variable-length structures from firmware or user space, does the parser verify that the declared length does not exceed the actual remaining buffer size?
* Does the logic that merges adjacent memory buffers respect all hardware boundaries that prohibit combining segments with differing physical properties?

## 7. Device-Managed Automatic Cleanup versus Manual Release Rules
**Targeted Bug Class:** Double frees, premature resource reclamation, and mixed-management lifecycle bugs.
**Extraction Mechanics:** The generated documentation must delineate the boundaries between resources tied automatically to the device lifecycle and those that require explicit, manual deallocation by the driver.

* Which specific resources are automatically reclaimed when the device driver unbinds, and which require explicit manual cleanup in the removal routine?
* If a device-managed resource is reallocated or resized during the lifetime of the device, how is the old resource safely detached and freed without waiting for the final device teardown?
* Does the driver mix manual memory allocation with automatic device-managed allocation in a way that could cause manual cleanup routines to access already-freed memory?
* When a probe function fails after acquiring multiple device-managed resources, does it rely entirely on the automatic cleanup framework, or are there manual rollback steps required for specific hardware states?
* Are references to external subsystems, such as power management domains or clock trees, properly integrated into the automatic cleanup framework to prevent reference leaks?
* How does the subsystem ensure that device-managed memory is not prematurely freed if an asynchronous worker is still executing after the primary device structure has been unregistered?
* When transferring ownership of a resource from a manual allocation context to a device-managed context, what specific interface must be used to prevent double-free scenarios?

## 8. Reconfiguration, Reset, Hotplug, and State-Transition Idempotency
**Targeted Bug Class:** State machine violations, stale hardware configurations, and hotplug race conditions.
**Extraction Mechanics:** The generated documentation must extract the rules for applying dynamic updates, handling unexpected hardware resets, and ensuring state transitions do not leave the system in an inconsistent or vulnerable state.

* When applying a dynamic configuration update, does the logic enforce the same uniqueness and validity constraints as the initial creation sequence?
* If the underlying hardware path changes due to a failover or hotplug event, does the driver automatically re-evaluate and update its advertised feature capabilities?
* Are hardware-specific configuration indices and register values derived dynamically from the current system state, rather than relying on hardcoded assumptions that may become stale?
* When a device undergoes a power management transition or reset, are all relevant processor caches explicitly flushed or invalidated to reflect the new memory ownership?
* Does the error recovery path for a failed state transition restore the exact previous configuration, or does it risk leaving the hardware in an undefined intermediate state?
* How does the subsystem handle requests to enable a feature that is mutually exclusive with a currently active feature, and is this conflict detected before any state is modified?
* Are cleanup operations for optional hardware features guarded by the exact same presence checks that were used during their initialization?
