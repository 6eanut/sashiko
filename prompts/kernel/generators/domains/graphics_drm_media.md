# Graphics, Display Pipelines, GPU Memory Management, Media, and Audio — Subsystem Prompt Generation Questions

The following version-agnostic questionnaire is designed to extract deep architectural documentation regarding hardware state machines, memory coherency, concurrency invariants, and error-recovery contracts within graphics, display, media, and audio subsystems. 

### 1. Hardware State, Power Management, and Register Synchronization
**Targeted Bug Class:** Hardware lockups, stale register state, and race conditions caused by accessing powered-down devices, concurrent read-modify-write operations, or missing cache flushes. The generated documentation must extract power transition rules, register locking schemes, and hardware flush requirements from the target source tree.

* How does the driver ensure that the device is fully transitioned to an active power state before any hardware registers are accessed during resume or asynchronous work?
* Are all read-modify-write operations on shared hardware registers protected by appropriate locking mechanisms to prevent concurrent modification by interrupt handlers or other threads?
* When a hardware feature is disabled, what is the exact sequence used to clear cached hardware state masks to prevent stale data from affecting future transactions?
* How are hardware-specific timeouts calculated and applied to account for variations in device response times across different platform configurations?
* Does the driver explicitly perform all required hardware cache flushes after device-to-host transfers, and how is this synchronized with central processing unit access?
* Are all runtime power management references acquired during initialization or asynchronous tasks strictly balanced with corresponding release calls on every possible exit path?
* When querying hardware capabilities, how does the logic ensure that the correct device node in the topology hierarchy is being accessed?

### 2. Display Pipeline, Media Topology, and State Machine Invariants
**Targeted Bug Class:** Premature device registration, incomplete atomic display updates, mismatched media formats, and invariant violations during state transitions. The generated guide must detail the exact sequence of topology discovery, capability negotiation, and the boundaries of atomic state transitions.

* Is the hardware device and its associated topology fully initialized and in a valid state before it is registered and made visible to user-space or other kernel subsystems?
* During an atomic display update, how is the ownership of reference-counted objects tracked at each stage to prevent double-release or use-after-free vulnerabilities?
* How does the driver validate that requested pixel formats, including those with alpha channels, are fully supported by the underlying hardware blending capabilities?
* Are all dependent hardware states and routing configurations correctly re-evaluated and updated when a single configuration parameter changes in the media pipeline?
* How does the atomic check phase validate all hardware-specific constraints against the proposed state before committing the changes to the hardware?
* When a sink device reports specific capabilities, how are these flags propagated from the generic display layer down to the hardware-specific driver layer?
* Does the driver logic correctly account for all possible combinations of connector properties and output color formats without falling into undefined states?

### 3. GPU Memory Management, Direct Memory Access, and Buffer Coherency
**Targeted Bug Class:** Memory corruption, direct memory access mapping violations, cache aliasing, and virtual memory area mismanagement. Documentation must extract buffer alignment rules, cache coherency attributes, and boundary calculation logic.

* Does the resource calculation logic account for all alignment, padding, and offset requirements of the target memory range to prevent out-of-bounds access?
* How does the driver ensure that direct memory access mapping segment sizes respect platform-specific constraints to maintain memory coherency?
* Does the page protection attribute used for virtual memory area mapping strictly match the cache coherency requirements of the underlying physical memory?
* If the fault handler relies on a virtual memory area spanning an entire buffer object, how does the driver explicitly forbid user-space from splitting that area?
* Are all dynamically calculated buffer sizes validated to ensure they exactly match the total size of the physical memory region they are intended to cover?
* How does the memory management logic ensure that the device is powered on and accessible before attempting to shrink or reclaim device-specific memory regions?
* Are hardware acceleration operations strictly restricted to a verified set of supported memory formats and layout configurations to prevent memory corruption?

### 4. Object Lifecycle, Asynchronous Work, and Teardown Sequences
**Targeted Bug Class:** Use-after-free vulnerabilities, uncancelled background tasks, improper read-copy-update usage, and dangling pointers during device unbinding. The documentation must map out the exact teardown hierarchy, asynchronous task cancellation requirements, and lifetime rules for shared objects.

* Are all pending asynchronous work items, timers, and hardware events explicitly cancelled and flushed before the underlying hardware resources are shut down?
* Is the teardown sequence for shared resources ordered such that all dependent objects are unlinked and destroyed before the primary parent structure is freed?
* When iterating over objects protected by read-copy-update mechanisms, how does the code guarantee that the object remains valid for the entire duration of the iteration?
* Are all hardware interrupts explicitly disabled and flushed before the data structures accessed by those interrupt handlers are destroyed?
* How is the lifecycle of device-managed resources strictly tied to the device lifetime, and what manual management is required if a resource must be re-acquired?
* When a task's identity or ownership changes, how are all associated graphics or media resources correctly re-validated or detached from the old task context?
* Are pointers to external objects that can be destroyed independently properly tracked, locked, or invalidated before they are dereferenced?

### 5. Error Unwinding, Reference Counting, and Resource Recovery
**Targeted Bug Class:** Memory leaks, unbalanced locks, double frees, and incorrect pointer validation on failure paths. The guide must extract the specific conventions for distinguishing error pointer sentinels from null values, and the exact order of resource cleanup.

* Does the error handling logic correctly distinguish between a successful zero-length result, a null pointer, and an error pointer sentinel when evaluating return values?
* Are all reference-counted objects acquired during a multi-step setup sequence explicitly released in the exact reverse order on every possible error exit path?
* When a memory reallocation operation fails, does the error path correctly preserve and eventually free the original memory pointer to prevent a leak?
* Are error codes returned by critical hardware operations preserved and propagated to the caller without being overwritten by subsequent non-critical cleanup failures?
* Does the driver use the appropriate reference counting release functions instead of bypassing them with direct memory deallocation on error paths?
* Are all locks acquired during an operation unconditionally released on error paths, including those triggered by memory allocation failures or signal interruptions?
* Can complex resource initialization be deferred until a point in the sequence where failure is no longer possible, thereby simplifying the error cleanup logic?

### 6. User-Space Boundaries, Integer Integrity, and Input Validation
**Targeted Bug Class:** Integer overflows, out-of-bounds array accesses, uninitialized stack variable usage, and missing capability checks originating from user-space or firmware inputs. The documentation must define the validation boundaries for all external inputs and arithmetic operations.

* Are all user-provided or firmware-provided counts and dimensions validated against the maximum capacity of the target buffers before any data transfer occurs?
* Are arithmetic operations on unsigned types, especially those derived from external inputs, checked for potential underflow or overflow before being used in range-limited fields?
* Are all structures passed to initialization functions or copied to user-space fully zero-initialized to prevent the leakage of uninitialized stack memory?
* Is every array index or identifier derived from external masks or hardware registers validated against the maximum supported range before being used for array access?
* Are all timeout parameters explicitly converted to the required time unit before being passed to helper functions, and is this unit tracked clearly to prevent double conversion?
* Does the control update function correctly return a specific status value when the underlying state has actually changed, ensuring that user-space receives the necessary notifications?
* Are all complex arithmetic expressions using bitwise operators correctly parenthesized to ensure the intended order of operations, especially when calculating buffer offsets?
