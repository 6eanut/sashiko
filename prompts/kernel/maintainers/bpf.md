# BPF Subsystem Maintainer Notes

**MAINTAINERS Sections:** `BPF [GENERAL] (Safe Dynamic Programs and Tools)`, `BPF [CORE]`, `BPF [SELFTESTS] (Test Runners & Infrastructure)`

## Multi-Line Comment Style

Files under `kernel/bpf/`, `net/core/filter.c`, `include/linux/bpf*.h`, `tools/lib/bpf/`, and `tools/testing/selftests/bpf/` follow the standard kernel multi-line comment style with the opening `/*` on its own line (as opposed to the networking `/* text` style), even when surrounding code in the file uses the older style:

```c
/*
 * Preferred multi-line comment style in BPF code.
 */
```

Single-line comments (`/* text */`) remain on one line.

## BPF Selftest Assertion Macros (`tools/testing/selftests/bpf/`)

- All new or modified BPF selftests must use the `ASSERT_*()` macro family (`ASSERT_OK()`, `ASSERT_ERR()`, `ASSERT_EQ()`, `ASSERT_OK_PTR()`, `ASSERT_OK_FD()`, etc. from `tools/testing/selftests/bpf/test_progs.h`).
- Do **not** introduce new uses of the deprecated `CHECK()`, `CHECK_FAIL()`, or `CHECK_ATTR()` macros (which require a `duration` variable).
