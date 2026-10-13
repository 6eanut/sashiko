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
| CXL | `drivers/cxl/`, `cxl_`, `hmat_get_extended_linear_cache_size` | `contexts/cxl.md` |
| DAX | `fs/dax.c`, `drivers/dax/`, `dax_layout_busy_page`, `dax_iomap_`, `dax_writeback_mapping_range` | `contexts/dax.md` |
| DRM / GPU | `drivers/gpu/drm/`, `include/drm/`, `drm_atomic_`, `drm_gpuvm`, `amdgpu_`, `hwseq`, `xe_pm_`, `intel_cx0_` | `contexts/drm.md` |
| Device Tree Bindings | `Documentation/devicetree/bindings/`, `$id: http://devicetree.org/schemas/` | `contexts/dt-bindings.md` |
| Filesystem Encryption (fscrypt) | `fs/crypto/`, `fscrypt_`, `FSCRYPT_` | `contexts/fscrypt.md` |
| FUSE | `fs/fuse/`, `fuse_uring_`, `fuse_chan_`, `fuse_dev_`, `FUSE_IO_URING` | `contexts/fuse.md` |
| ARM GICv3 / GICv4 & vGIC | `drivers/irqchip/irq-gic-v3*`, `drivers/irqchip/irq-gic-v4*`, `arch/arm64/kvm/vgic/`, `GICD_`, `GICR_`, `GITS_`, `ICH_LR` | `contexts/gic-v3.md` |
| ARM GICv5 & vGICv5 | `drivers/irqchip/irq-gic-v5*`, `vgic-v5`, `arm-gic-v5.h`, `gicv5_`, `GICV5_`, `ICC_PPI_`, `gsb_sys`, `gsb_ack` | `contexts/gic-v5.md` |

## Kernel API Guides (`apis/`)

| API / Header | Triggers (Functions, Macros, Types) | File |
| :--- | :--- | :--- |
| Alignment Helpers | `ALIGN`, `ALIGN_DOWN`, `IS_ALIGNED`, `PAGE_ALIGN`, `PAGE_ALIGN_DOWN`, `PAGE_ALIGNED`, `pageblock_align`, `pageblock_aligned`, `pageblock_start_pfn`, `pageblock_end_pfn` | `apis/alignment.md` |
| Block & Bio API | `blk_mq_freeze_queue`, `bio_has_data`, `op_is_write`, `bio_alloc_bioset`, `bio_integrity_prep`, `bio_kmalloc` | `apis/block.md` |
| Boot & Module Parameters | `__setup`, `early_param`, `module_param`, `module_param_named`, `core_param` | `apis/boot-params.md` |
| BPF Kfuncs & Skeletons | `__bpf_kfunc`, `BTF_KFUNCS_*`, `bpf_program__fd`, `bpf_map__fd`, `__open_and_load` | `apis/bpf.md` |
| BTF Map Field Helpers | `check_and_init_map_value`, `bpf_obj_cancel_fields`, `bpf_obj_free_fields`, `copy_map_value` | `apis/btf.md` |
| Cleanup & Guard API | `__free`, `guard`, `scoped_guard`, `DEFINE_FREE`, `DEFINE_GUARD`, `no_free_ptr`, `return_ptr`, `retain_and_null_ptr` | `apis/cleanup.md` |
| DRM, Overflow & fwnode APIs | `struct_size`, `fwnode_create_software_node`, `stack_depot_save`, `xe_device_get_gt`, `msm_context_vm`, `amdgpu_bo_create_kernel`, `intel_cx0_rmw` | `apis/drm.md` |
