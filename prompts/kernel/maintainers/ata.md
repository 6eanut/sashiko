# LibATA Maintainer Notes

**MAINTAINERS Sections:** `LIBATA SUBSYSTEM (Serial and Parallel ATA drivers)`

## Device Validation & Backward Compatibility Policy

Real-world ATA devices frequently deviate from ACS specifications in benign ways (such as reporting version `0x0000` instead of `0x0001` in the General Purpose Log Directory, leaving reserved bits non-zero, or zero-filling optional log fields).

- **Justification required for strict checks:** Any patch adding stricter validation to device initialization or log-parsing paths that could reject or disable features on previously working drives must explicitly justify why strict enforcement is required and assess compatibility impact in the commit description.
- **Warn rather than fail on cosmetic deviations:**
  - **Fail with error:** Data integrity failures (bad checksums, corrupted structures), I/O read errors, or invalid configuration of safety-critical features (NCQ, TRIM, security).
  - **Warn and continue (`ata_dev_warn()` / `ata_dev_warn_once()`):** Version field mismatches on otherwise valid structures, non-zero reserved bits, or minor optional-field formatting deviations.
- **Avoid permanent runtime quirk disabling on cosmetic mismatches:** Do not set `ATA_QUIRK_*` flags dynamically or clear log directories on benign spec deviations without a fallback recovery path.
