# Input Subsystem Maintainer Notes

## Coding Style and Cleanup Conventions

* **Comments**: Prefer C-style `/* ... */` comments over C++ `// ...` comments, using the standard multi-line block comment layout.
* **Single Capability Registration**: Prefer `input_set_capability(dev, type, code)` over direct `__set_bit(code, dev->evbit)` manipulation when registering single capabilities (direct bit manipulation remains fine inside loops).
* **Scope-Based Cleanup**: Prefer `guard(mutex)(&input_dev->mutex)` and `__free()` (`__free(kfree)`, `__free(firmware)`) in new or substantially refactored input driver code.
* **Error Variable Naming and Return Path**: Name variables that hold negative errno values `error` or `err`, and write explicit failure and success paths in functions with multiple failure points rather than tail-returning a fallible call:
  ```c
  error = action(...);
  if (error)
          return error;

  return 0;
  ```
* **Redundant `input_unregister_device()` on `devm` Devices**: When a driver allocates its device with `devm_input_allocate_device(dev)`, recommend removing an explicit `input_unregister_device()` call in `remove()` *only* after verifying that (1) no non-`devm` asynchronous resources (timers, workqueues, non-`devm` IRQs) destroyed in `remove()` can be re-triggered by input callbacks (`open`, `play_effect`), and (2) all `devm` resources that the input device depends on were allocated before `devm_input_allocate_device()` (LIFO teardown order). Also recommend removing redundant manual `input_dev->dev.parent = dev` assignments on `devm_input_allocate_device(dev)` devices.
