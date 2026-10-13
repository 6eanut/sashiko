# Syscall Parameter Validation & User Copy API (`include/linux/syscalls.h`, `include/linux/uaccess.h`)

## Flag-Gated Syscall Parameter Trust Boundaries (`SYSCALL_DEFINE*`)

- Syscall arguments come directly from user-controlled registers or stack slots. Arguments (or `copy_struct_from_user()` / `copy_from_user()` fields) that are only defined when a specific `flags` bit is set may contain arbitrary uninitialized values when that flag bit is absent—userspace is not required to zero-fill unused register arguments.
- Copying raw syscall arguments into an internal kernel struct does not sanitize them: each struct field inherits the flag gate of its source argument and remains untrusted garbage when that flag is clear.
- When adding or refactoring parameter checks in `SYSCALL_DEFINE*` handlers or helper functions, never read, compare, or perform arithmetic on a flag-gated parameter outside the conditional scope of its governing flag check.
