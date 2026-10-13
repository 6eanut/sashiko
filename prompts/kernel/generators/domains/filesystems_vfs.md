# Filesystems and Virtual Filesystem Switch — Subsystem Prompt Generation Questions

This guide provides a version-agnostic, identifier-free framework for generating deep architectural review prompts for filesystem and virtual filesystem switch code. The questions are designed to expose subtle bugs in error unwinding, object lifecycles, concurrency, memory safety, and metadata state machines.

### 1. Error Handling, Unwind Paths, and Resource Cleanup
**Bug Class Target:** Resource leaks, unbalanced reference counts, double frees, and improper handling of error pointer sentinels versus null pointers.
**Target Mechanics to Extract:** Error exit blocks, allocation sequences, pointer validation logic, and lock release sequences during failure paths.

*   When a multi-step allocation fails midway, does the error path explicitly reverse every successful allocation and reference increment in the exact reverse order of acquisition?
*   Are return values from nested parsing or metadata operations strictly checked for error pointer sentinels versus null pointers before being dereferenced?
*   If a compound network filesystem request fails partially, does the cleanup logic correctly account for the intermediate state without double-freeing resources?
*   Are all state-tracking flags and temporary buffers reset or freed on every possible error exit path before returning to the caller?
*   Does the error recovery path ensure that sufficient transaction resources are reserved to complete the cleanup operation itself?
*   Are transient error codes correctly distinguished from permanent corruption errors, and are they propagated upward without being masked by generic failure codes?
*   When an operation involves multiple locks, does the error path guarantee that all acquired locks are released, avoiding unbalanced lock states?

### 2. Object Lifecycle, Teardown, and Reference Counting
**Bug Class Target:** Use-after-free vulnerabilities, dangling pointers, premature object destruction, and list corruption during teardown.
**Target Mechanics to Extract:** Reference counting mechanics, asynchronous teardown callbacks, cache eviction logic, and list removal sequences.

*   Are all pending asynchronous tasks, delayed work items, and timers explicitly cancelled and flushed before the underlying object memory is freed?
*   When an object is removed from a global cache or list, is it unhashed or marked invalid before the final reference is dropped to prevent concurrent access?
*   Are pointers to shared buffers or structures immediately set to null after the underlying memory is released to prevent dangling pointer dereferences?
*   Does the filesystem correctly follow the required sequence for hashing and unlocking new index nodes before exposing them to the virtual filesystem layer?
*   When a task's identity or namespace ownership changes, are all associated filesystem resources correctly re-validated or detached from the old context?
*   Are objects reused across multiple operations properly reset to a clean, initialized state before being reinserted into active lists?
*   Do mount ownership rules and namespace bindings avoid creating circular reference dependencies that would permanently prevent object destruction?

### 3. Concurrency, Locking, and Race Conditions
**Bug Class Target:** Deadlocks, time-of-check to time-of-use races, invalid context execution (sleeping in atomic), and missing read-copy-update protections.
**Target Mechanics to Extract:** Lock ordering rules, read-copy-update critical sections, context-dependent allocation flags, and shared state mutation blocks.

*   Are all operations that might sleep, such as memory allocations or blocking input-output, strictly performed outside of atomic contexts and read-copy-update critical sections?
*   Does the locking sequence during mount or eviction operations introduce new dependencies that could lead to lock inversion deadlocks under specific configurations?
*   Are resource requirements and buffer lengths sampled exactly once and copied to a private local variable to prevent time-of-check to time-of-use race conditions?
*   Is list iteration safe against the concurrent removal of elements, especially if the lock must be temporarily dropped during the loop body?
*   Are all modifications to shared filesystem data structures protected by the appropriate synchronization primitives before asserting ownership?
*   When reading shared statistics or diagnostic counters, are they protected by locks or atomic snapshots to prevent reading torn or inconsistent values?
*   Are pointers that are re-sampled after reacquiring a lock explicitly re-validated for null or invalid states before being dereferenced?

### 4. Memory Safety, Bounds Checking, and Data Integrity
**Bug Class Target:** Buffer overflows, out-of-bounds accesses, integer overflows, uninitialized variables, and unaligned memory access.
**Target Mechanics to Extract:** Arithmetic bounds checks, payload parsing logic, structure offset calculations, and memory copy boundaries.

*   Are buffer length checks strictly performed before accessing fixed-width fields in variable-length network or disk data structures?
*   Do all offset-based string pointers and variable-length structure offsets verify that the offset is strictly greater than the header size and within the total buffer limit?
*   Are arithmetic operations on resource counters, buffer sizes, and array indices checked for potential integer overflow or underflow before being applied to memory operations?
*   When allocating a buffer based on an existing data structure that will be modified, does the calculation account for the maximum possible size increase of all variable-length fields?
*   Are all reserved, padding, or unused fields in user-provided structures explicitly validated to be zero to prevent information leaks or future compatibility issues?
*   Does the buffer size calculation for memory copy operations account for the exact length of the data being copied, preventing heap buffer overflows?
*   Are all inputs used in bitwise shift operations validated to ensure the shift amount is within the safe, defined range for the target data type?
*   Are pointer arithmetic operations that could result in unaligned memory access handled using the architecture-independent unaligned access helpers?

### 5. Filesystem State Machines, Metadata, and Transactions
**Bug Class Target:** Corrupted on-disk state, missing transaction aborts, incorrect metadata cross-referencing, and vestigial error handling.
**Target Mechanics to Extract:** Transaction lifecycle hooks, metadata validation routines, state transition logic, and block boundary calculations.

*   Are all critical filesystem operations that modify on-disk metadata wrapped in a transaction that is explicitly aborted if any intermediate step fails?
*   Does the metadata allocator use a dynamic reservation strategy that adapts to the actual on-disk layout rather than relying on hardcoded block limits?
*   When cross-referencing metadata, does the validation logic verify the entire extent or range rather than just checking the starting block address?
*   Are all checksums and cryptographic signatures calculated only after all data modifications and state transitions are completely finalized?
*   Does the filesystem iteration logic correctly handle directory entries or extents that span across or align exactly with underlying block boundaries?
*   When repairing a file or recovering a journal, are all metadata attributes, including the logical file size, correctly updated to reflect the recovered state?
*   Does every configuration change path correctly trigger all dependent state recomputations, cache invalidations, and quota updates?

### 6. Virtual Filesystem Integration and API Contracts
**Bug Class Target:** Incorrect delegation to the virtual filesystem, improper error reporting to user space, and backward compatibility breaks.
**Target Mechanics to Extract:** Interface boundaries, delegation callbacks, user-space reporting formats, and index node attribute updates.

*   Does the filesystem correctly delegate non-regular file handling, such as device nodes and sockets, to the virtual filesystem instead of forcing a custom open operation?
*   Are read operations on symbolic links strictly bounded by the actual reported file size to prevent reading beyond the intended allocated data?
*   Does the filesystem correctly update the index node size based on the actual number of bytes successfully written to the remote server or block device?
*   Do changes to exported identifiers, diagnostic statistics, or binary interfaces maintain strict backward compatibility for existing user-space applications?
*   Is the error reporting mechanism aware of the subsystem's initialization state to avoid logging spurious warnings for expected failures during early boot?
*   When processing compound or multi-part messages, is the length state updated correctly for each individual sub-component before passing it to the virtual filesystem layer?
