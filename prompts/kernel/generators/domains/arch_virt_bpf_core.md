# CPU Architectures, Virtualization, Verifier/Bytecode Execution, Tracing, Scheduler, and Core Kernel — Subsystem Prompt Generation Questions

The following version-agnostic questions are designed to extract critical architectural rules, state machine invariants, and concurrency contracts from core kernel subsystems, virtualization layers, and architecture-specific code. 

### 1. Bytecode Verification and Execution Safety
**Target Bug Class:** Out-of-bounds memory accesses, integer underflow/overflow, unintended sign extension, invalid pointer poisoning, and state tracking failures during just-in-time compilation or bytecode interpretation.
**Documentation Goal:** Extract the exact mathematical bounds-checking logic, register width transition rules, pointer lifecycle constraints, and state-uniqueness guarantees enforced by the bytecode verifier before execution is permitted.

* How does the bytecode verifier ensure that all input offsets are strictly validated against expected ranges before they are used to calculate memory indices?
* What mechanisms prevent integer underflow or unintended sign extension when performing arithmetic operations on register values of varying bit widths?
* How does the execution state tracker guarantee uniqueness when comparing execution paths, particularly regarding stack frame identifiers and branch histories?
* When passing pointers to subprograms, how does the verifier ensure that references to volatile memory regions are not invalidated, leaked, or misused?
* What safeguards prevent unbounded recursion or kernel stack exhaustion when the bytecode engine destroys complex objects or follows nested pointers?
* How are control flow graphs validated after instruction relocation or patching to ensure no invalid branches or infinite loops are introduced?
* Does the verifier accurately account for the execution context of all possible callers when determining if a specific subprogram is permitted to sleep or block?

### 2. Virtualization and Hardware State Management
**Target Bug Class:** Missing processor cache maintenance, physical versus virtual address confusion, guest-to-host page size mismatches, incorrect hardware register caching, and state corruption on failed hypervisor operations.
**Documentation Goal:** Document the hardware-software boundary contracts, including cache flushing requirements on memory transitions, physical address translation rules, and the state machine for preserving or rolling back hardware state during interrupted virtualization events.

* When memory ownership or encryption state changes between host and guest environments, how are all relevant processor caches explicitly flushed or invalidated?
* How are memory addresses passed to hardware or firmware explicitly validated as physical addresses, and what translation mechanism is strictly enforced?
* What ensures that hardware-specific event constraints and data source encodings strictly match the documented capabilities of the target processor architecture?
* When consolidating hardware register accessors, how are all possible register indices and their specific privilege or access requirements accounted for?
* If a virtualization operation is interrupted or fails, how does the state machine preserve pending requests so they can be retried without causing state corruption?
* How does the memory mapping interface validate that requested protection flags are appropriate for the specific target memory type being mapped to the guest?
* When caching hardware state structures, what logic ensures these caches are immediately cleared or invalidated if the underlying hardware reports an invalid or transitional state?
* How does the architecture initialization sequence guarantee that early boot parameters are parsed before dependent hardware subsystems attempt to read them?

### 3. Scheduler Dynamics and Task Lifecycle
**Target Bug Class:** Task migration during critical sections, incorrect context accounting, processor hotplug topology desynchronization, and state flag inconsistencies.
**Documentation Goal:** Capture the synchronization rules for per-processor data, the distinction between execution and scheduling contexts, load balancing invariants, and the exact sequence of operations required during processor hotplug events.

* How does the accounting logic correctly distinguish between the execution context and the scheduling context when multiple tasks share a single processor time slice?
* What protects operations that modify per-processor hardware state against unexpected task migration or preemption mid-update?
* During processor hotplug events, what enforces the correct order of operations to ensure the scheduler topology accurately reflects the active hardware state?
* How do state counters tracking task properties maintain absolute consistency when tasks transition rapidly between runnable, blocked, and migrating states?
* When falling back to active migration, how does the load balancing logic preserve task-specific placement constraints and processor affinities?
* How are kernel threads explicitly filtered out when performing scheduler operations or telemetry accounting intended strictly for user-space tasks?
* What mechanisms ensure that objects accessed by the scheduler have a lifecycle entirely independent of the underlying memory management structures that might be torn down concurrently?

### 4. Tracing, Performance Monitoring, and Callbacks
**Target Bug Class:** Uninitialized trace fields leaking memory, missing context in callbacks, variable-length array termination failures, and object lifetime mismatches between events and monitoring units.
**Documentation Goal:** Identify the initialization requirements for telemetry structures, the context-passing contracts for callbacks, array boundary sentinels, and the reference counting rules tying hardware monitoring units to active events.

* How are all fields within a trace context or performance monitoring structure explicitly initialized before being exposed to external consumers or user space?
* What ensures that callbacks provide sufficient context to uniquely identify the specific event being reported without relying on fragile global state?
* When parsing variable-length arrays or data structures in tracing events, what explicit sentinel or terminator prevents the parser from reading past valid entries?
* How is the lifetime of a performance monitoring unit structure synchronized with the active events that reference it to prevent use-after-free vulnerabilities on teardown?
* What validates that all required event flags are set for hardware events that perform direction-sensitive sampling or instruction profiling?
* How does the driver filter configuration order map exactly to the bitfield positions defined in the official hardware specification?
* Are all pointers passed to tracing callbacks strictly validated or guaranteed to be non-null by the calling context before any dereferencing occurs?

### 5. Concurrency, Preemption, and Atomic Contexts
**Target Bug Class:** Sleeping in atomic contexts, missing read-copy-update protections, lock recursion deadlocks, and time-of-check to time-of-use race conditions.
**Documentation Goal:** Map out the strict boundaries of atomic execution contexts, the required read-copy-update locking primitives for shared data, deadlock-prevention hierarchies, and data-copying rules for layered architectures.

* How does the subsystem guarantee that memory allocations or blocking operations are never invoked while holding spinlocks or operating in atomic contexts?
* What ensures that every shared object subject to concurrent updates or deletion is protected by appropriate read-copy-update primitives during read access?
* How are callbacks that might attempt to acquire a lock structured to ensure they are only invoked after the caller has explicitly released that same lock?
* In stacked filesystems or layered architectures, how is data validated and atomically copied to a private buffer to prevent time-of-check to time-of-use race conditions?
* How is the ownership of an object protected during concurrent teardown and processing to prevent use-after-free vulnerabilities?
* What guarantees that long-running operations within the core kernel remain preemptible or responsive to signals to avoid stalling the processor?
* When a function requires specific synchronization primitives, how does the context guarantee that these locks are already held by the caller prior to entry?

### 6. Resource Teardown and Error Path Recovery
**Target Bug Class:** Unbalanced reference counts on error paths, memory leaks during initialization failures, premature subsystem registration, and missing asynchronous cleanup.
**Documentation Goal:** Extract the exact teardown sequence required for multi-step initializations, the symmetric reference counting rules, the gating logic for subsystem registration, and the cancellation requirements for asynchronous work.

* On failure paths involving multi-step operations, how are all dynamically allocated resources explicitly freed regardless of the object's intermediate state?
* When an error occurs, what ensures that reference counts are symmetrically decremented for every object acquired during the setup phase?
* How is the registration of secondary interfaces gated to ensure it only occurs after the successful completion of the primary object's initialization?
* If a function already has access to a reference-counted object, how does it reuse that existing reference instead of unnecessarily acquiring and potentially leaking a new one?
* What ensures that pending timers or asynchronous work items are explicitly canceled and flushed before freeing the memory structures they reference?
* How do error reporting mechanisms handle cases where the associated device object is not yet fully available or has already been removed from the system?
* When checking for allocation or lookup failures, how does the subsystem consistently distinguish between a null pointer and an error pointer sentinel to prevent invalid dereferences?
