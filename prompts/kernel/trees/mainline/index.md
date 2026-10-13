# Mainline Kernel 3-Part Prompt Catalog (Linux 7.3)

Select any relevant subsystem/file code contexts (`contexts/<file>`) or kernel API guides (`apis/<file>`) from the tables below when reviewing a patch.

## Subsystem & File Code Contexts (`contexts/`)

| Subsystem / Area | Triggers (Paths, Symbols, Concepts) | File |
| :--- | :--- | :--- |
| ARM64 Architecture | `arch/arm64/`, `.sysreg`, `ESR_ELx`, `untagged_addr`, `write_sysreg`, `TLBI`, `isb` | `contexts/arm64.md` |

## Kernel API Guides (`apis/`)

| API / Header | Triggers (Functions, Macros, Types) | File |
| :--- | :--- | :--- |
| Alignment Helpers | `ALIGN`, `ALIGN_DOWN`, `IS_ALIGNED`, `PAGE_ALIGN`, `PAGE_ALIGN_DOWN`, `PAGE_ALIGNED`, `pageblock_align`, `pageblock_aligned`, `pageblock_start_pfn`, `pageblock_end_pfn` | `apis/alignment.md` |
