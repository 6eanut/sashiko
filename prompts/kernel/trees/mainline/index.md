# Mainline Kernel 3-Part Prompt Catalog (Linux 7.3)

Select any relevant subsystem/file code contexts (`contexts/<file>`) or kernel API guides (`apis/<file>`) from the tables below when reviewing a patch.

## Subsystem & File Code Contexts (`contexts/`)

| Subsystem / Area | Triggers (Paths, Symbols, Concepts) | File |
| :--- | :--- | :--- |
| ARM64 Architecture | `arch/arm64/`, `.sysreg`, `ESR_ELx`, `untagged_addr`, `write_sysreg`, `TLBI`, `isb` | `contexts/arm64.md` |
| ATA / libata | `drivers/ata/`, `ata_dev_`, `ata_port_`, `ata_read_log_`, `ATA_QUIRK_` | `contexts/ata.md` |
| Block Layer | `block/`, `drivers/nvme/`, `blk_mq_`, `elevator_mq_ops`, `depth_updated` | `contexts/block.md` |
| Bluetooth | `net/bluetooth/`, `hci_`, `HCI_LE_ADV`, `adv_instances`, `cur_adv_instance`, `mgmt_pending_` | `contexts/bluetooth.md` |

## Kernel API Guides (`apis/`)

| API / Header | Triggers (Functions, Macros, Types) | File |
| :--- | :--- | :--- |
| Alignment Helpers | `ALIGN`, `ALIGN_DOWN`, `IS_ALIGNED`, `PAGE_ALIGN`, `PAGE_ALIGN_DOWN`, `PAGE_ALIGNED`, `pageblock_align`, `pageblock_aligned`, `pageblock_start_pfn`, `pageblock_end_pfn` | `apis/alignment.md` |
| Block & Bio API | `blk_mq_freeze_queue`, `bio_has_data`, `op_is_write`, `bio_alloc_bioset`, `bio_integrity_prep`, `bio_kmalloc` | `apis/block.md` |
