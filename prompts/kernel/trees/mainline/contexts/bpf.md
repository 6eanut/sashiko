# BPF Verifier & Kfunc Subsystem Context (`kernel/bpf/`)

## Kfunc Scalar and Enum Argument Validation (`kernel/bpf/verifier.c`)

In `check_kfunc_args()`, parameters matching `btf_type_is_scalar(t)` (which includes `int`, `u32`, and C `enum` types) are checked only to verify that the register holds a `SCALAR_VALUE`.

- **No enum range validation by the verifier:** The verifier does **not** check that an `enum` argument falls within the declared enum constants. Unless the parameter has a `__const` suffix annotation (which forces `tnum_is_const()` at verification time), a BPF program can pass any arbitrary scalar register value (from map lookups or arithmetic).
- **Mandatory bounds checks in kfuncs:** Any `__bpf_kfunc` that uses a non-`__const` integer or `enum` parameter to index an array must validate both upper and lower bounds at runtime. Because C `enum` types may be signed `int`, a single `idx >= MAX` check is bypassed by negative values; cast to `unsigned int` (`(unsigned int)event >= MAX`) or check `idx < 0 || idx >= MAX`.
