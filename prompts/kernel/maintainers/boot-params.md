# Boot Parameters Documentation Policy

**MAINTAINERS Sections:** `DOCUMENTATION`

## `kernel-parameters.txt` Documentation Requirement

Every user-facing kernel command-line parameter or boot-accessible module parameter added or modified in the kernel must be documented in `Documentation/admin-guide/kernel-parameters.txt` in the same patch or series.

- **Required entry contents:** Parameter name, syntax/value format (e.g., `param=off` or `param=[val1,val2]`), architecture/Kconfig tags in square brackets, and behavior description.
- **Exceptions:**
  - Debug-only parameters gated by `#ifdef CONFIG_DEBUG_KERNEL` that are documented in comments at the definition site.
  - Internal driver parameters gated by `CONFIG_EXPERIMENTAL` with no user-facing contract.
