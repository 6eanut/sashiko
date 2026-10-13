# Device Model, Buses, DMA, Power Management, Clocks, Devicetree, and Peripherals — Subsystem Prompt Generation Questions

### Section 1: Hardware Description and Firmware Interfaces
**Bug Class Target:** Missing firmware properties, incorrect hardware matching, malformed data types, and missing module aliases.
**Extraction Mechanics:** The generated documentation must extract how the driver parses firmware nodes, validates mandatory versus optional properties, handles legacy data formats, and registers hardware match tables to ensure the module loader can bind the driver to the physical device.

1. Are all mandatory hardware description properties explicitly validated for existence and correct data type before being consumed by the initialization logic?
2. Does the driver provide safe fallback values or graceful degradation paths when optional firmware properties are absent?
3. Are all string identifiers used for hardware matching strictly synchronized with the corresponding firmware bindings documentation?
4. Does the driver correctly export its hardware match table to ensure the system module loader can automatically bind it to the physical device?
5. Is the firmware node reference safely released on all error paths and during device teardown to prevent memory leaks?
6. Does the driver handle all expected variations of firmware-provided data types, including legacy formats and newer hardware revisions?

### Section 2: Device Initialization and Teardown Lifecycles
**Bug Class Target:** Premature registration, unbalanced reference counts, error pointer sentinel versus null check mismatches, and resource leaks.
**Extraction Mechanics:** The generated documentation must extract the exact sequence of resource allocation, the reverse-order cleanup steps on error paths, the distinction between device-managed resources and manually managed allocations, and the synchronization required before exposing the device to the system.

7. Are all synchronization primitives, buffers, and state variables fully initialized before the device is registered and exposed to the system or interrupt handlers?
8. Does the error handling path during initialization strictly release all acquired resources in the exact reverse order of their allocation?
9. Are device-managed resources strictly left to the framework for cleanup, avoiding manual release calls that would cause double-free errors?
10. When evaluating the result of an allocation or lookup, does the code correctly distinguish between a null pointer and an error pointer sentinel based on the specific subsystem contract?
11. Are all dynamically allocated sub-devices and child nodes explicitly tracked and unregistered during the parent device's removal phase?
12. Is the reference count of the device object correctly decremented on every possible error exit path to prevent memory leaks?
13. Are all pointers to dynamically allocated objects cleared from global or shared structures immediately upon initialization failure to prevent dangling pointers?

### Section 3: Power Management and Clock Synchronization
**Bug Class Target:** Unbalanced runtime power management usage, suspend and resume deadlocks, missing clock enables, and stale thermal states.
**Extraction Mechanics:** The generated documentation must extract the runtime power management reference counting rules, the clock enablement sequence, and the specific hardware state required during system suspend and resume transitions.

14. Are runtime power management usage counts correctly balanced on all possible execution paths, including early error returns and timeouts?
15. Does the driver explicitly enable all required hardware clocks before attempting to read or write to any device registers?
16. Are all operations that require kernel thread activity or memory allocation completed before the system suspends and freezes user-space processes?
17. Does the suspend and resume logic check the actual hardware state independently of the software-level active flags?
18. Is the hardware state fully synchronized with the software configuration immediately after a power resume cycle?
19. Does the driver's removal path symmetrically undo all runtime power management configuration steps performed during initialization?

### Section 4: Direct Memory Access and Buffer Management
**Bug Class Target:** Scatter-gather list length errors, insufficient alignment, integer overflow in length calculations, and sleeping in atomic contexts.
**Extraction Mechanics:** The generated documentation must extract the alignment requirements, scatter-gather list iteration logic, buffer length calculation methods, and bounce buffer address adjustments used for direct memory access.

20. Does the scatter-gather list iteration logic use the length of the current segment rather than the total length of the list head?
21. Are all memory buffers mapped for hardware transfer strictly aligned to the boundary requirements specified by the hardware architecture?
22. When calculating addresses for bounce buffers, does the logic consistently apply the adjusted address for both the page frame and the offset?
23. Are length accumulators and offset counters sized appropriately to prevent integer overflow when handling large memory ranges?
24. Is a synchronization barrier or grace period enforced before freeing a memory buffer that might still be accessed by the hardware or read-copy-update readers?
25. Are all blocking or sleeping memory allocations strictly avoided when executing within an atomic context or while holding a spinlock?

### Section 5: Hardware Register Access and Bitwise Operations
**Bug Class Target:** Incorrect bitmasks, read-modify-write corruption, non-atomic reads of related registers, and division-by-zero errors.
**Extraction Mechanics:** The generated documentation must extract the register bitmask widths, read-modify-write synchronization rules, validation checks for hardware-derived values, and version-specific register offsets.

26. Are hardware-specific register bit encodings and masks verified against the logical interface values before being written to the device?
27. Does the bitmask used for register updates exactly match the hardware-defined width of the target field without overflowing into adjacent bits?
28. Are read-modify-write operations on shared hardware registers protected by appropriate locking to prevent concurrent updates from corrupting the state?
29. Do multiple related hardware registers require an atomic read sequence or a specific synchronization latch to ensure a consistent view of the hardware state?
30. Are all divisors derived from external hardware registers or clock sources explicitly validated to be non-zero before performing arithmetic operations?
31. Does the driver check the hardware version or revision before accessing registers that may have been deprecated or relocated in newer variants?
32. Are integer types explicitly cast to a wider type before performing bitwise shifts that might exceed the original bit width?

### Section 6: Concurrency, Asynchronous Execution, and Timers
**Bug Class Target:** Uncancelled work items or timers before free, missing locks in interrupt context, and race conditions in task state updates.
**Extraction Mechanics:** The generated documentation must extract the teardown synchronization mechanisms for timers, deferred work items, and interrupt handlers, ensuring no background task outlives its parent device.

33. Are all asynchronous callbacks, timers, and deferred work items guaranteed to be stopped and cancelled before the resources they access are released?
34. Does the teardown sequence use a mechanism that strictly prevents a timer or work item from rearming itself after the shutdown process has begun?
35. Are all execution paths that access shared hardware state, including interrupt handlers, protected by the same synchronization primitive?
36. Is the task state explicitly set before checking a wait condition to ensure no wake-up events are missed between the check and the sleep?
37. Are all necessary metadata fields cached locally before dropping the final reference to an object that might be concurrently freed by another thread?

### Section 7: Build Configuration and Module Integration
**Bug Class Target:** Missing build dependencies, unused variables, broken documentation cross-references, and missing module tables.
**Extraction Mechanics:** The generated documentation must extract the build-time dependencies, module selection rules, early boot parameter parsing stages, and documentation cross-references required for the subsystem.

38. Are all build-time configuration dependencies explicitly declared to prevent unresolved symbols when related subsystems are built as loadable modules?
39. Are default configuration dependencies verified against existing symbols to ensure they do not select deprecated or incompatible features?
40. Are all user-space interface attributes documented in the subsystem guides strictly consistent with the actual attribute names defined in the source code?
41. Are early boot parameters parsed at the correct stage in the architecture initialization sequence before dependent subsystems are initialized?
