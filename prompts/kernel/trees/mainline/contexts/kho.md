# KHO (Kexec Handover) Code Context (`kernel/liveupdate/kexec_handover.c`)

## Scratch Region Overlap and Native-Endian FDTs

* **Scratch Region Overlap (`kho_scratch_overlap`)**: `kho_preserve_folio()` and `kho_preserve_pages()` reject ranges overlapping the CMA-backed KHO scratch regions (`kho_reserve_scratch()`, scaled by `kho_scratch=`) with `WARN_ON` and `-EINVAL`, because scratch regions are reserved for the successor kernel's early-boot allocations.
* **Native Host Endianness in KHO FDTs**: Unlike standard devicetree blobs (which are big-endian), KHO handover FDTs store property values in **native host endianness** because the producer and consumer kernels run on the same machine and architecture. Do not flag native-endian reads or writes on KHO FDT properties as endianness bugs.
