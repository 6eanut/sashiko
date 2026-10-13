# Kernel Boot & Module Parameter Registration API (`include/linux/init.h`, `include/linux/moduleparam.h`)

## Parameter Registration Macros

- `__setup(str, fn)` and `early_param(str, fn)` (`include/linux/init.h`) register early or standard kernel command-line parameter handlers.
- `core_param(name, var, type, perm)`, `module_param(name, type, perm)`, and `module_param_named(name, value, type, perm)` (`include/linux/moduleparam.h`) register parameters accessible on the kernel command line (`<name>=` or `<module>.<name>=`).

## Review Check when Adding or Changing Parameters

When a patch adds, renames, or changes the accepted values of `__setup()`, `early_param()`, `core_param()`, `module_param()`, or `module_param_named()`:
1. Verify whether `Documentation/admin-guide/kernel-parameters.txt` is updated in the patch/series.
2. Report missing `Documentation/admin-guide/kernel-parameters.txt` updates as a defect unless the parameter is strictly gated by `CONFIG_DEBUG_KERNEL` or `CONFIG_EXPERIMENTAL`.
