# Kernel Selftests (`kselftest`) Maintainer Notes (`tools/testing/selftests/`)

## TAP Result Reporting (`kselftest.h` / `kselftest_harness.h`)

- **Use standard kselftest TAP helpers:** Do not write ad-hoc `printf("PASS/FAIL...")` output or exit via raw `exit()` / `return` from `main()` in C selftests.
  - For simple sequential tests, use `ksft_print_header()`, `ksft_set_plan(n)`, `ksft_test_result*()` (`ksft_test_result_pass()`, `ksft_test_result_fail()`, `ksft_test_result_skip()`, `ksft_test_result_error()`), and exit via `ksft_exit_pass()` / `ksft_exit_fail()` / `ksft_exit_skip()`.
  - For structured multi-case or parameterized tests, use `kselftest_harness.h` (`TEST()`, `TEST_F()`, `FIXTURE()`, `FIXTURE_SETUP()`, `FIXTURE_TEARDOWN()`, `FIXTURE_VARIANT()`, and `ASSERT_*` / `EXPECT_*`).

## Skip vs Fail for Unconfigured or Unsupported Features

- **Skip with a reason when a prerequisite is absent:** When a syscall, ioctl, or device open fails with `EOPNOTSUPP`, `ENOSYS`, or `ENODEV` because a kernel `CONFIG_*`, hardware feature, or filesystem capability is not present on the test system, call `ksft_test_result_skip()`, `SKIP(return, "reason...")`, or `TEST_REQUIRE()` with a clear message explaining what prerequisite is missing rather than failing `make run_tests`.

## Reuse Shared Test Utilities and Extend Existing Tests

- **Reuse subsystem test helpers:** Before writing custom sysfs/file I/O, network namespace creation, or polling loops, use the subsystem's shared utility headers/scripts:
  - `tools/testing/selftests/mm/vm_util.h` (`read_sysfs`, `write_sysfs`, `read_file`, `write_file`, `log_test_result`)
  - `tools/testing/selftests/net/lib.sh` (`setup_ns`, `cleanup_ns`, `cleanup_all_ns`, `busywait`, `log_test`, `create_netdevsim`)
  - `tools/testing/selftests/bpf/DENYLIST*` and `vmtest.sh`
- **Test the contract and extend existing binaries:** Test the observable uAPI/kernel contract rather than internal implementation details. When adding a new size, flag, or mapping variant for an interface already covered by an existing test binary in the same directory (such as `gup_test.c` or `cow.c` in `mm/`), add a test case or fixture variant to the existing test rather than creating a new standalone binary.
- **`Fixes:` tags on test bugfixes:** Ensure `Fixes:` tags in selftest bugfix commits reference the commit that actually introduced the test bug.
