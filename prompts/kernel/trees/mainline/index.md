# Mainline Kernel 3-Part Prompt Catalog (Linux 7.3)

Select any relevant subsystem/file code contexts (`contexts/<file>`) or kernel API guides (`apis/<file>`) from the tables below when reviewing a patch.

## Subsystem & File Code Contexts (`contexts/`)

| Subsystem / Area | Triggers (Paths, Symbols, Concepts) | File |
| :--- | :--- | :--- |
| ARM64 Architecture | `arch/arm64/`, `.sysreg`, `ESR_ELx`, `untagged_addr`, `write_sysreg`, `TLBI`, `isb` | `contexts/arm64.md` |
| ATA / libata | `drivers/ata/`, `ata_dev_`, `ata_port_`, `ata_read_log_`, `ATA_QUIRK_` | `contexts/ata.md` |
| Block Layer | `block/`, `drivers/nvme/`, `blk_mq_`, `elevator_mq_ops`, `depth_updated` | `contexts/block.md` |
| Bluetooth | `net/bluetooth/`, `hci_`, `HCI_LE_ADV`, `adv_instances`, `cur_adv_instance`, `mgmt_pending_` | `contexts/bluetooth.md` |
| BPF Verifier & Kfuncs | `kernel/bpf/`, `__bpf_kfunc`, `BTF_KFUNCS`, `check_kfunc_args` | `contexts/bpf.md` |
| BTF Special Map Fields | `map_check_btf`, `BPF_SPIN_LOCK`, `BPF_TIMER`, `BPF_KPTR`, `BPF_LIST_HEAD`, `BPF_RB_ROOT` | `contexts/btf.md` |
| Btrfs | `fs/btrfs/`, `extent_map`, `btrfs_extent_map_block_start`, `btrfs_get_dev_zone_info` | `contexts/btrfs.md` |
| Build System & Compiler | `Kbuild`, `Makefile`, `scripts/`, `gnu11`, `-funsigned-char`, `-fno-strict-aliasing` | `contexts/build.md` |

## Kernel API Guides (`apis/`)

| API / Header | Triggers (Functions, Macros, Types) | File |
| :--- | :--- | :--- |
| Alignment Helpers | `ALIGN`, `ALIGN_DOWN`, `IS_ALIGNED`, `PAGE_ALIGN`, `PAGE_ALIGN_DOWN`, `PAGE_ALIGNED`, `pageblock_align`, `pageblock_aligned`, `pageblock_start_pfn`, `pageblock_end_pfn` | `apis/alignment.md` |
| Block & Bio API | `blk_mq_freeze_queue`, `bio_has_data`, `op_is_write`, `bio_alloc_bioset`, `bio_integrity_prep`, `bio_kmalloc` | `apis/block.md` |
| Boot & Module Parameters | `__setup`, `early_param`, `module_param`, `module_param_named`, `core_param` | `apis/boot-params.md` |
| BPF Kfuncs & Skeletons | `__bpf_kfunc`, `BTF_KFUNCS_*`, `bpf_program__fd`, `bpf_map__fd`, `__open_and_load` | `apis/bpf.md` |
| BTF Map Field Helpers | `check_and_init_map_value`, `bpf_obj_cancel_fields`, `bpf_obj_free_fields`, `copy_map_value` | `apis/btf.md` |
