# Scope-Based Cleanup Style Policy (`include/linux/cleanup.h`)

**MAINTAINERS Sections:** `GENERIC INCLUDE/LINUX/ HEADERS`

## No Mixing of `goto` Cleanup and `__free()` / `guard()`

Per `include/linux/cleanup.h`, never mix `goto`-based error-cleanup labels with `__free()`, `guard()`, or `scoped_guard()` in the same function. Either convert all resources in a function to scope-based cleanup, or keep traditional `goto` unwind throughout the function.

## Combined Declaration and Initialization

Declare and initialize `__free()` variables at the point of allocation rather than declaring `struct foo *p __free(...) = NULL;` at the top of the function and assigning `p` later, which makes LIFO teardown ordering relative to subsequent `guard()` acquisitions error-prone.
