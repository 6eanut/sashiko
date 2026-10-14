# Mainline Kernel 3-Part Prompt Catalog (Linux v7.3-rc5)

Select any relevant subsystem/file code contexts (`contexts/<file>`) or kernel API guides (`apis/<file>`) from the tables below when reviewing a patch.

## Subsystem & File Code Contexts (`contexts/`)

| Subsystem / Area | Triggers (Paths, Symbols, Concepts) | File |
| :--- | :--- | :--- |
| ARM64 Architecture & Subsystem Context | `arch/arm64/`, `sysreg`, `ESR_ELx`, `untagged_addr`, `write_sysreg`, `read_sysreg` | `contexts/arm64.md` |
| ATA / libata Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/ata/`, `ata_dev_`, `ata_port_`, `ata_read_log_`, `ATA_QUIRK_` | `contexts/ata.md` |
| Block Layer Subsystem Context (Linux v7.3-rc5) | `block/`, `drivers/nvme/`, `blk_mq_`, `elevator_mq_ops`, `depth_updated`, `nr_requests` | `contexts/block.md` |
| Bluetooth Subsystem Context (`net/bluetooth/`) | `net/bluetooth/`, `hci_`, `HCI_LE_ADV`, `adv_instances`, `cur_adv_instance`, `mgmt_pending_` | `contexts/bluetooth.md` |
| BPF Verifier & Kfunc Subsystem Context (`kernel/bpf/`) | `kernel/bpf/`, `__bpf_kfunc`, `BTF_KFUNCS`, `check_kfunc_args`, `verifier`, `BTF_ID_FLAGS`, `KF_*` | `contexts/bpf.md` |
| BPF Subsystem Code Context Guide (Linux v7.3-rc5) | `map_check_btf`, `btf_parse_fields`, `BPF_SPIN_LOCK`, `BPF_RES_SPIN_LOCK`, `BPF_TIMER`, `BPF_KPTR`, `BPF_LIST_HEAD`, `BPF_RB_ROOT` | `contexts/btf.md` |
| Btrfs Subsystem Context (`fs/btrfs/`) | `fs/btrfs/`, `extent_map`, `btrfs_extent_map_block_start`, `btrfs_get_dev_zone_info` | `contexts/btrfs.md` |
| Kernel Build System & Compiler Environment (`Makefile`, `scripts/`, `tools/`) | `Kbuild`, `Makefile`, `scripts/`, `gnu11`, `-funsigned-char`, `-fno-strict-aliasing` | `contexts/build.md` |
| Linux v7.3-rc5 Subsystem Code Context Guide: Clock Framework | `drivers/clk/`, `clk_hw`, `clk_ops`, `clk_init_data`, `devm_clk_hw_register`, `of_clk_add_hw_provider` | `contexts/clk.md` |
| CXL Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/cxl/`, `cxl_`, `hmat_get_extended_linear_cache_size` | `contexts/cxl.md` |
| DAX (Direct Access) Subsystem Context (Linux v7.3-rc5) | `fs/dax.c`, `drivers/dax/`, `dax_layout_busy_page`, `dax_iomap_`, `dax_writeback_mapping_range` | `contexts/dax.md` |
| DMA Engine Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/dma/`, `dma_device`, `dma_chan`, `dma_async_tx_descriptor`, `dmaengine_terminate_sync`, `vchan_` | `contexts/dmaengine.md` |
| DRM / GPU Subsystem Context (`drivers/gpu/drm/`) | `drivers/gpu/drm/`, `include/drm/`, `drm_atomic_`, `drm_crtc_`, `drm_gpuvm`, `amdgpu_`, `amdkfd`, `hwseq` | `contexts/drm.md` |
| Device Tree YAML Bindings Context (`Documentation/devicetree/bindings/`) | `Documentation/devicetree/bindings/`, `#gpio-cells`, `#clock-cells`, `#interrupt-cells` | `contexts/dt-bindings.md` |
| Ext4 and JBD2 Subsystem Code Context Guide (Linux v7.3-rc5) | `fs/ext4/`, `fs/jbd2/`, `ext4_inode_info`, `ext4_sb_info`, `jbd2_journal_start`, `jbd2_journal_stop`, `ext4_ext_` | `contexts/ext4.md` |
| Filesystem Encryption (`fscrypt`) Context (`fs/crypto/`, `include/linux/fscrypt.h`) | `fs/crypto/`, `fscrypt_`, `FSCRYPT_` | `contexts/fscrypt.md` |
| FUSE Subsystem Code Context Guide (Linux v7.3-rc5) | `fs/fuse/`, `fuse_uring_`, `fuse_chan_`, `fuse_dev_`, `FUSE_IO_URING`, `FUSE_OVER_IO_URING` | `contexts/fuse.md` |
| ARM GICv3 / GICv4 Subsystem Context (`drivers/irqchip/irq-gic-v3*`, `arch/arm64/kvm/vgic/`) | `drivers/irqchip/irq-gic-v3`, `drivers/irqchip/irq-gic-v4`, `vgic-v3`, `vgic-v4`, `vgic-its`, `vgic-mmio`, `arm-gic-v3.h`, `arm-gic-v4.h` | `contexts/gic-v3.md` |
| ARM GICv5 Subsystem Context (`drivers/irqchip/irq-gic-v5*`, `arch/arm64/kvm/vgic/vgic-v5.c`) | `drivers/irqchip/irq-gic-v5`, `vgic-v5`, `arm-gic-v5.h`, `gicv5_`, `GICV5_`, `ICC_PPI_`, `ICH_PPI_`, `ICC_ICSR` | `contexts/gic-v5.md` |
| HID Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/hid/`, `include/linux/hid.h`, `hid_device`, `hid_driver`, `hid_ll_driver`, `report_fixup`, `input_mapping` | `contexts/hid.md` |
| ARM64 Hyp (EL2 / pKVM / nVHE) Code Context Guide (v7.3-rc5) | `arch/arm64/kvm/hyp/`, `drivers/iommu/arm/arm-smmu-v3/pkvm/`, `pkvm_`, `__pkvm_`, `kern_hyp_va`, `hyp_pool`, `hyp_alloc_pages`, `hyp_fixmap_` | `contexts/hyp-arm64.md` |
| I2C Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/i2c/`, `i2c_client`, `i2c_adapter` | `contexts/i2c.md` |
| Input Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/input/`, `input_dev`, `input_handler`, `evdev` | `contexts/input.md` |
| io_uring Subsystem Code Context (`io_uring/`) | `io_uring/`, `io_uring_`, `io_ring_`, `io_sq_`, `io_cq_`, `io_wq_`, `io_kiocb`, `IORING_` | `contexts/io_uring.md` |
| Kconfig Code Context (`scripts/kconfig/`) | `Kconfig`, `COMPILE_TEST` | `contexts/kconfig.md` |
| Kexec Handover (KHO) and Live Update Orchestrator (LUO) Code Context Guide | `kernel/liveupdate/`, `kexec_handover`, `kho_` | `contexts/kho.md` |
| ARM64 KVM (Host / EL1) Code Context (`arch/arm64/kvm/`) | `arch/arm64/kvm/`, `vcpu_has_run_once`, `kvm_vcpu_initialized`, `vcpu_sysreg`, `user_mem_abort`, `ICH_HCR_EL2` | `contexts/kvm-arm64.md` |
| KVM Subsystem Code Context (`virt/kvm/`, `arch/x86/kvm/`) | `virt/kvm/`, `arch/x86/kvm/`, `kvm_mmu_`, `is_page_fault_stale`, `mmu_invalidate_seq`, `gfn_to_pfn_cache`, `slots_lock` | `contexts/kvm.md` |
| libbpf Subsystem Code Context Guide (Linux v7.3-rc5) | `tools/lib/bpf/`, `LIBBPF_API`, `libbpf_err`, `libbpf_err_ptr`, `libbpf_ptr` | `contexts/libbpf.md` |
| Media Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/media/`, `include/media/`, `v4l2_subdev_`, `V4L2_SUBDEV_`, `MEDIA_BUS_FMT_` | `contexts/media.md` |
| MIPS Architecture Code Context (`arch/mips/`) | `arch/mips/`, `tlb_probe`, `tlb_read`, `tlb_write_indexed`, `write_c0_entryhi`, `read_c0_index`, `TLBP`, `TLBR` | `contexts/mips.md` |
| MM Page Allocator, SLUB, and vmalloc Code Context (`mm/`) | `mm/page_alloc.c`, `mm/slub.c`, `mm/slab_common.c`, `mm/vmalloc.c`, `get_page_from_freelist`, `__alloc_pages_slowpath`, `lowmem_reserve`, `allocate_slab` | `contexts/mm-alloc.md` |
| MM Folio Lifecycle and Page Cache Code Context (Linux v7.3-rc5) | `mm/filemap.c`, `mm/swap.c`, `mm/truncate.c`, `page_folio`, `compound_head`, `filemap_`, `mapping_set_update`, `folio_set_swapbacked` | `contexts/mm-folio.md` |
| MM Large Folios, THP, Hugetlb, and Memory Failure Context (Linux v7.3-rc5) | `mm/huge_memory.c`, `mm/hugetlb.c`, `mm/memory-failure.c`, `hugetlb`, `hstate`, `huge_pmd_`, `memory_failure`, `unmap_poisoned_folio` | `contexts/mm-largepage.md` |
| MM Page Table Internals (`mm/memory.c`, `mm/mprotect.c`, `mm/pagewalk.c`) | `mm/memory.c`, `mm/mprotect.c`, `mm/pagewalk.c`, `mm/page_vma_mapped.c`, `mm/pt_reclaim.c`, `zap_pte_range`, `change_pte_range`, `try_to_free_pte` | `contexts/mm-pagetable.md` |
| MM Reclaim, Swap, and Migration Internals (`mm/vmscan.c`, `mm/swap_state.c`, `mm/migrate.c`, `mm/memcontrol.c`, `mm/page-writeback.c`, `mm/shmem.c`) | `mm/vmscan.c`, `mm/swap_state.c`, `mm/swapfile.c`, `mm/migrate.c`, `mm/memcontrol.c`, `mm/page-writeback.c`, `mm/shmem.c`, `shrink_folio_list` | `contexts/mm-reclaim.md` |
| MM VMA Internals (`mm/vma.c`, `mm/mmap.c`, `mm/mmap_lock.c`) | `mm/vma.c`, `mm/mmap.c`, `mm/mmap_lock.c`, `lock_vma_under_rcu`, `vma_start_read`, `vma_prepare`, `vma_complete`, `vma_modify` | `contexts/mm-vma.md` |
| Linux v7.3-rc5 Network Subsystem Code Context Guide | `drivers/net/ethernet/`, `net_device_ops`, `napi_enable`, `napi_disable`, `netif_napi_add`, `ndo_open`, `ndo_stop`, `ndo_start_xmit` | `contexts/net-drivers.md` |
| Netfilter Subsystem Code Context Guide (Linux v7.3-rc5) | `net/netfilter/`, `nf_tables`, `nft_`, `nf_conntrack`, `nf_hook_ops`, `nft_trans` | `contexts/netfilter.md` |
| Networking Core Subsystem Code Context Guide (Linux v7.3-rc5) | `net/core/`, `net/xfrm/`, `skb->cb`, `destructor_arg`, `skb_uarg`, `xfrm_state`, `inner_mode`, `virtio_net_hdr` | `contexts/networking-core.md` |
| NFS Server (NFSD) Subsystem Internals (`fs/nfsd/`, `fs/lockd/`) | `fs/nfsd/`, `fs/lockd/`, `fh_verify`, `nfsd4_`, `nfs4_stid`, `nfs4_preprocess_stateid_op`, `nfsd_user_namespace`, `release_openowner` | `contexts/nfsd.md` |
| NVMe Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/nvme/`, `nvme_ctrl`, `nvme_ns`, `nvme_queue`, `nvme_change_ctrl_state`, `nvme_quiesce_io_queues` | `contexts/nvme.md` |
| Objtool Control-Flow Validation (`tools/objtool/`) | `tools/objtool/`, `INSN_BUG`, `INSN_TRAP`, `validate_retpoline`, `validate_sls`, `ignore_unreachable_insn` | `contexts/objtool.md` |
| Open Firmware (Device Tree) Core & MSI Resolution Subsystem Guide (v7.3-rc5) | `drivers/of/`, `of_msi_xlate`, `of_msi_get_domain`, `of_check_msi_parent`, `msi-map`, `msi-parent` | `contexts/of.md` |
| PCI Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/pci/`, `pci_dev`, `pci_driver`, `pci_enable_device`, `pci_disable_device`, `pci_set_master`, `pcie_capability_` | `contexts/pci.md` |
| Perf Tools Subsystem Code Context Guide (Linux v7.3-rc5) | `tools/perf/`, `perf_tool`, `perf_env`, `DECLARE_RC_STRUCT`, `thread__get`, `thread__put`, `maps__get`, `dso__get` | `contexts/perf.md` |
| Generic Power Domains (`drivers/pmdomain/`, `include/linux/pm_domain.h`) | `drivers/pmdomain/`, `pm_genpd_`, `of_genpd_`, `GENPD_FLAG_NO_STAY_ON`, `genpd_power_off`, `exynos_pd_` | `contexts/pmdomain.md` |
| RISC-V Subsystem Code Context Guide (Linux v7.3-rc5) | `arch/riscv/`, `riscv_`, `csr_read`, `csr_write`, `sbi_ecall`, `local_flush_tlb_` | `contexts/riscv.md` |
| Kernel Rust Toolchain, Bindings, and Pin-Init (`rust/`) | `rust/`, `*.rs` files, `__rust_helper`, `rust_helper_`, `RUST_CONST_HELPER_`, `try_pin_init`, `pin_init`, `build_assert!` | `contexts/rust.md` |
| Scheduler Internals (`kernel/sched/`) | `kernel/sched/`, `double_rq_lock`, `lockdep_assert_rq_held`, `__pick_eevdf`, `update_load_avg`, `sched_cpu_dying`, `dl_server_` | `contexts/scheduler.md` |
| Linux SCSI Subsystem Code Context Guide (v7.3-rc5) | `drivers/scsi/`, `drivers/ufs/`, `Scsi_Host`, `scsi_cmnd`, `scsi_device`, `scsi_host_template`, `scsi_done`, `ufshcd_` | `contexts/scsi.md` |
| Kernel Selftests Build, KVM, and Netns Mechanics (`tools/testing/selftests/`) | `tools/testing/selftests/`, `TEST_PROGS`, `TEST_FILES`, `TEST_GEN_PROGS`, `TEST_GEN_FILES`, `kselftest`, `vm_create_with_one_vcpu`, `kvm_arch_has_default_irqchip` | `contexts/selftests.md` |
| SMB/CIFS Subsystem Code Context Guide (Linux v7.3-rc5) | `fs/smb/smbdirect/`, `fs/smb/server/`, `smbdirect_`, `smb_direct_`, `ksmbd_`, `refill_work`, `immediate_work` | `contexts/smb-ksmbd.md` |
| Linux Sound Subsystem Code Context Guide (v7.3-rc5) | `sound/`, `snd_soc_`, `snd_pcm_`, `snd_card`, `snd_kcontrol`, `snd_soc_dapm_` | `contexts/sound.md` |
| SunRPC Transport & GSS Internals (`net/sunrpc/`) | `net/sunrpc/`, `svc_xprt_`, `svc_rqst`, `rpc_task`, `rpc_clnt`, `xprtrdma`, `rpcrdma_`, `svc_rdma_` | `contexts/sunrpc.md` |
| Sysfs Attribute Group Internals (`fs/sysfs/`) | `fs/sysfs/`, `sysfs_group_attrs_change_owner`, `sysfs_groups_change_owner`, `kernfs_find_and_get`, `SYSFS_GROUP_INVISIBLE` | `contexts/sysfs.md` |
| TTY & 8250 Serial Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/tty/`, `8250_base`, `8250_port`, `8250_core`, `serial_core` | `contexts/tty.md` |
| USB Mass Storage Subsystem Code Context Guide (Linux v7.3-rc5) | `drivers/usb/storage/`, `unusual_devs.h`, `UNUSUAL_DEV`, `USB_SC_`, `USB_PR_`, `US_FL_` | `contexts/usb-storage.md` |
| Linux v7.3-rc5 USB Subsystem Code Context Guide | `drivers/usb/`, `usb_device`, `usb_interface`, `urb`, `usb_submit_urb`, `usb_kill_urb`, `usb_anchor` | `contexts/usb.md` |
| VFS Path Walk, Open, and Superblock Internals (`fs/namei.c`, `fs/open.c`, `fs/dcache.c`) | `fs/namei.c`, `fs/open.c`, `fs/dcache.c`, `LOOKUP_RCU`, `try_to_unlazy`, `lookup_fast`, `lookup_slow`, `do_dentry_open` | `contexts/vfs.md` |
| Wireless (`mac80211`) Subsystem Code Context Guide (v7.3-rc5) | `drivers/net/wireless/`, `net/mac80211/`, `BSS_CHANGED_`, `vif_cfg_changed`, `link_info_changed`, `bss_info_changed`, `ieee80211_vif_cfg_change_notify`, `ieee80211_link_info_change_notify` | `contexts/wireless.md` |
| x86 Subsystem Code Context Guide (Linux v7.3-rc5) | `arch/x86/`, `cpu_feature_enabled`, `boot_cpu_has`, `wrmsrl`, `rdmsrl`, `fpu_`, `sev_`, `tdx_` | `contexts/x86.md` |
| XFS Subsystem Code Context Guide (Linux v7.3-rc5) | `fs/xfs/`, `xfs_mount`, `xfs_inode`, `xfs_trans`, `xfs_ilock`, `xfs_iunlock`, `xfs_buf` | `contexts/xfs.md` |

## Kernel API Guides (`apis/`)

| API / Header | Triggers (Functions, Macros, Types) | File |
| :--- | :--- | :--- |
| Alignment Helpers API | `ALIGN`, `ALIGN_DOWN`, `IS_ALIGNED`, `PAGE_ALIGN`, `PAGE_ALIGN_DOWN`, `PAGE_ALIGNED`, `pageblock_align`, `pageblock_aligned` | `apis/alignment.md` |
| Block Layer & Bio API Rules (Linux v7.3-rc5) | `blk_mq_freeze_queue`, `blk_mq_unfreeze_queue`, `bio_has_data`, `op_is_write`, `bio_alloc`, `bio_alloc_bioset`, `bvec_alloc`, `bio_integrity_prep`, `bio_for_each_*` | `apis/block.md` |
| Kernel Boot & Module Parameter Registration API Guide (v7.3-rc5) | `__setup`, `early_param`, `module_param`, `module_param_named`, `core_param` | `apis/boot-params.md` |
| BPF Kfunc & Skeleton API Guide (Linux v7.3-rc5) | `__bpf_kfunc`, `bpf_program__fd`, `bpf_map__fd`, `bpf_object__find_program_by_name`, `bpf_object__find_map_by_name`, `BTF_KFUNCS_*` | `apis/bpf.md` |
| BPF Map Special Field Lifecycle API Guide (Linux v7.3-rc5) | `check_and_init_map_value`, `bpf_obj_cancel_fields`, `bpf_obj_free_fields`, `copy_map_value`, `copy_map_value_locked` | `apis/btf.md` |
| Scope-Based Cleanup & Guard API (`include/linux/cleanup.h`) | `__free`, `guard`, `scoped_guard`, `DEFINE_FREE`, `DEFINE_GUARD`, `DEFINE_LOCK_GUARD_1`, `DEFINE_LOCK_GUARD_0`, `no_free_ptr` | `apis/cleanup.md` |
| Linux Kernel API Guide: Common Clock Framework (CCF) | `clk_prepare`, `clk_unprepare`, `clk_enable`, `clk_disable`, `clk_prepare_enable`, `clk_disable_unprepare`, `clk_get`, `clk_put`, `clk_hw_*`, `devm_clk_*` | `apis/clk.md` |
| Device-Managed Resources (Devres) API Guide - Linux v7.3-rc5 | `devm_kmalloc`, `devm_kzalloc`, `devm_kcalloc`, `devm_kfree`, `devm_add_action`, `devm_add_action_or_reset`, `devm_remove_action`, `devm_release_action`, `devm_*`, `devres_*` | `apis/devres.md` |
| Linux Kernel DMA Mapping API Guide (v7.3-rc5) | `dma_map_single`, `dma_unmap_single`, `dma_map_page`, `dma_unmap_page`, `dma_map_sg`, `dma_unmap_sg`, `dma_map_sgtable`, `dma_mapping_error`, `dma_map_*`, `dma_unmap_*` | `apis/dma-mapping.md` |
| DRM, GPU Driver, & Overflow/Property Helper APIs (v7.3-rc5) | `struct_size`, `array_size`, `fwnode_create_software_node`, `stack_depot_save`, `xe_device_get_gt`, `xe_root_mmio_gt`, `msm_context_vm`, `amdgpu_bo_create_kernel` | `apis/drm.md` |
| Linux Kernel API Guide: Error Pointer Handling (`include/linux/err.h`) | `ERR_PTR`, `PTR_ERR`, `IS_ERR`, `IS_ERR_OR_NULL`, `ERR_CAST`, `PTR_ERR_OR_ZERO`, `MAX_ERRNO` | `apis/err-ptr.md` |
| GPIO Subsystem API Guide (Linux v7.3-rc5) | `gpio_get_value`, `gpio_set_value`, `gpio_get_value_cansleep`, `gpio_set_value_cansleep`, `gpiod_get_raw_value`, `gpiod_set_raw_value`, `gpiod_get_raw_value_cansleep`, `gpiod_set_raw_value_cansleep`, `gpiochip_*` | `apis/gpio.md` |
| HID Transport and Driver API Guide (Linux v7.3-rc5) | `hid_allocate_device`, `hid_add_device`, `hid_destroy_device`, `hid_parse`, `hid_open_report`, `hid_hw_start`, `hid_hw_stop`, `hid_device_io_start` | `apis/hid.md` |
| Hardware Monitoring (hwmon) API Guide (Linux v7.3-rc5) | `hwmon_device_register_with_info`, `devm_hwmon_device_register_with_info`, `hwmon_lock`, `hwmon_unlock` | `apis/hwmon.md` |
| I2C Subsystem API Guide (Linux v7.3-rc5) | `i2c_transfer`, `i2c_master_send`, `i2c_master_recv`, `i2c_master_send_dmasafe`, `i2c_master_recv_dmasafe`, `I2C_M_DMA_SAFE`, `i2c_get_dma_safe_msg_buf`, `i2c_put_dma_safe_msg_buf`, `i2c_smbus_*` | `apis/i2c.md` |
| Input Device and Force-Feedback APIs (`include/linux/input.h`) | `input_allocate_device`, `devm_input_allocate_device`, `input_register_device`, `input_unregister_device`, `input_free_device`, `input_device_enabled`, `input_event`, `input_sync`, `input_report_*` | `apis/input.md` |
| MMIO Register and Stream Accessor APIs (`include/asm-generic/io.h`) | `writesl`, `readsl`, `writesw`, `readsw`, `writesb`, `readsb`, `__raw_writel`, `__raw_readl` | `apis/io-accessors.md` |
| Linux v7.3-rc5 Kernel IRQ API Guide | `request_irq`, `request_threaded_irq`, `free_irq`, `devm_request_irq`, `devm_request_threaded_irq`, `enable_irq`, `disable_irq`, `disable_irq_nosync`, `irq_domain_*`, `generic_handle_*` | `apis/irq.md` |
| Kexec Handover (KHO) API Guide (Linux v7.3-rc5) | `kho_is_enabled`, `is_kho_boot`, `kho_add_subtree`, `kho_remove_subtree`, `kho_retrieve_subtree`, `kho_preserve_folio`, `kho_unpreserve_folio`, `kho_restore_folio`, `kho_*` | `apis/kho.md` |
| KVM Core Helper APIs (`include/linux/kvm_host.h`) | `gfn_to_hva`, `gfn_to_hva_many`, `gfn_to_hva_memslot`, `gfn_to_hva_memslot_prot`, `gfn_to_pfn*`, `kvm_vcpu_memslots`, `kvm_memslots`, `vcpu_load`, `vcpu_put` | `apis/kvm.md` |
| LED Class and Trigger API Guide (Linux v7.3-rc5) | `led_classdev_register`, `led_classdev_register_ext`, `led_classdev_unregister`, `devm_led_classdev_register`, `devm_led_classdev_register_ext`, `led_trigger_register`, `led_trigger_unregister`, `devm_led_trigger_register` | `apis/leds.md` |
| Kernel Locking, Concurrency, and Memory Ordering APIs | `local_lock`, `local_trylock`, `local_lock_irqsave`, `lockdep_pin_lock`, `lockdep_unpin_lock`, `DEFINE_WAIT_OVERRIDE_MAP`, `cpus_read_lock`, `cpus_read_unlock`, `spin_lock*`, `raw_spin_lock*` | `apis/locking.md` |
| Multi-Function Device (MFD) API Guide (Linux v7.3-rc5) | `mfd_add_devices`, `devm_mfd_add_devices`, `mfd_remove_devices`, `mfd_cell` | `apis/mfd.md` |
| Memory Allocation, GFP, Slab, and Mempool APIs (Linux v7.3-rc5) | `alloc_pages`, `alloc_pages_node`, `kmalloc`, `kzalloc`, `kmalloc_nolock`, `kfree_nolock`, `vmalloc`, `mempool_alloc`, `__GFP_*`, `kmem_cache_*` | `apis/mm-alloc.md` |
| Folio, Page Cache, and XArray API Guide (Linux v7.3-rc5) | `folio_expected_ref_count`, `folio_ref_count`, `folio_mapcount`, `folio_lock`, `folio_trylock`, `folio_end_read`, `folio_*`, `filemap_*` | `apis/mm-folio.md` |
| Large Folio, THP, and Hugetlb API Guide (Linux v7.3-rc5) | `split_folio`, `split_huge_page`, `split_folio_to_list`, `try_folio_split_to_order`, `min_order_for_split`, `mapping_min_folio_order`, `folio_put_refs`, `folio_test_large`, `split_huge_*` | `apis/mm-largepage.md` |
| Page Table Accessors, Batching, and Walkers (`include/linux/pgtable.h`, `include/linux/pagewalk.h`) | `set_ptes`, `folio_pte_batch`, `folio_pte_batch_flags`, `FPB_RESPECT_WRITE`, `folio_mk_pmd`, `folio_mk_pud`, `pte_mkspecial`, `pmd_folio`, `pte_*`, `ptep_*` | `apis/mm-pagetable.md` |
| Writeback, Swap, Memcg, and Folio Migration API (Linux v7.3-rc5) | `folio_start_writeback`, `__folio_start_writeback`, `wbc_to_tag`, `folio_test_swapbacked`, `folio_test_swapcache`, `get_swap_device`, `put_swap_device`, `__swap_entry_to_info`, `PAGECACHE_TAG_*`, `mem_cgroup_*` | `apis/mm-reclaim.md` |
| VMA Classification, Flags, Locking, and Mapping Helpers (`include/linux/mm.h`, `include/linux/mmap_lock.h`) | `vma_is_anonymous`, `vma_set_anonymous`, `vma_desc_set_anonymous`, `vm_flags_set`, `vm_flags_reset`, `vm_flags_init`, `vm_flags_clear`, `vm_flags_mod`, `vm_flags_*`, `mmap_read_lock*` | `apis/mm-vma.md` |
| Netlink & Generic Netlink API (`include/net/netlink.h`, `include/net/genetlink.h`) | `genl_notify`, `NLM_F_ECHO`, `NLM_F_DUMP_INTR`, `NL_SET_BAD_ATTR`, `NL_SET_ERR_ATTR_MISS`, `NLA_NUL_STRING`, `netlink_callback`, `genl_*`, `nla_*`, `NLA_*` | `apis/netlink.md` |
| Networking Core API Guide (Linux v7.3-rc5) | `skb_put`, `skb_push`, `skb_pull`, `pskb_may_pull`, `skb_shared`, `skb_cloned`, `skb_unshare`, `release_sock`, `skb_*`, `dst_*` | `apis/networking-core.md` |
| 64-Bit Network Driver Statistics API Guide (Linux v7.3-rc5) | `u64_stats_sync`, `u64_stats_init`, `u64_stats_update_begin`, `u64_stats_update_end`, `u64_stats_update_begin_irqsave`, `u64_stats_update_end_irqrestore`, `u64_stats_fetch_begin`, `u64_stats_fetch_retry`, `u64_stats_*` | `apis/networking-drivers.md` |
| Open Firmware (Device Tree) Node Reference & Iterator API (`include/linux/of.h`) | `for_each_child_of_node`, `for_each_available_child_of_node`, `for_each_reserved_child_of_node`, `for_each_child_of_node_scoped`, `for_each_available_child_of_node_scoped`, `for_each_node_by_type`, `for_each_compatible_node`, `for_each_matching_node`, `of_find_*` | `apis/of.md` |
| PCI Endpoint & IRQ Vector API Guide (Linux v7.3-rc5) | `pci_epc_get`, `pci_epc_put`, `pci_epf_create`, `pci_epf_destroy`, `pci_alloc_irq_vectors`, `pci_free_irq_vectors`, `pci_enable_msi`, `pci_disable_msi`, `pci_epc_*`, `pci_epf_*` | `apis/pci.md` |
| Power Management & Runtime PM API Guide (Linux v7.3-rc5) | `pm_runtime_get_sync`, `pm_runtime_resume_and_get`, `pm_runtime_get_noresume`, `pm_runtime_get_if_active`, `pm_runtime_put`, `pm_runtime_put_sync`, `pm_runtime_put_autosuspend`, `pm_runtime_put_sync_suspend`, `pm_runtime_*`, `__pm_runtime_*` | `apis/pm.md` |
| Read-Copy-Update (RCU) API Guide (Linux v7.3-rc5) | `rcu_read_lock`, `rcu_read_unlock`, `rcu_assign_pointer`, `rcu_dereference`, `synchronize_rcu`, `call_rcu`, `kfree_rcu`, `kfree_rcu_mightsleep`, `rcu_*` | `apis/rcu.md` |
| Linux Kernel API Guide: `refcount_t` and `kref` (v7.3-rc5) | `refcount_t`, `refcount_set`, `refcount_read`, `refcount_inc`, `refcount_inc_not_zero`, `refcount_dec`, `refcount_dec_and_test`, `refcount_dec_and_mutex_lock`, `refcount_*`, `kref_*` | `apis/refcount.md` |
| Task State, Wakeup, and Migration API (`include/linux/sched.h`) | `set_current_state`, `__set_current_state`, `schedule`, `wake_up_process`, `try_to_wake_up`, `migrate_disable`, `migrate_enable`, `is_migration_disabled`, `wake_up_*` | `apis/scheduler.md` |
| Syscall Parameter Validation & User Copy API Guide (Linux v7.3-rc5) | `copy_from_user`, `copy_to_user`, `copy_struct_from_user`, `get_user`, `put_user`, `SYSCALL_DEFINE*` | `apis/syscall.md` |
| Sysfs Group & Kobject Lifecycle API Guide (Linux v7.3-rc5) | `sysfs_create_group`, `sysfs_update_group`, `attribute_group`, `is_visible`, `is_bin_visible`, `SYSFS_GROUP_INVISIBLE`, `kobject_create_and_add`, `kobject_put`, `sysfs_*` | `apis/sysfs.md` |
| Kernel Timers (`timer_list`, `hrtimer`, `delayed_work`) (`include/linux/timer.h`, `include/linux/hrtimer.h`) | `timer_list`, `timer_setup`, `mod_timer`, `add_timer`, `timer_delete`, `timer_delete_sync`, `timer_shutdown_sync`, `timer_pending`, `timer_*`, `hrtimer_*` | `apis/timers.md` |
| Kernel Tracepoints & `TRACE_EVENT` API Guide (Linux v7.3-rc5) | `TRACE_EVENT`, `TRACE_EVENT_CONDITION`, `TRACE_EVENT_SYSCALL`, `DECLARE_TRACE_SYSCALL`, `TP_STRUCT__entry`, `TP_fast_assign`, `__string`, `__assign_str`, `trace_*`, `tracepoint_*` | `apis/tracing.md` |
| Serial Core & UART API Guide (Linux v7.3-rc5) | `uart_add_one_port`, `uart_remove_one_port`, `uart_ops`, `uart_configure_port`, `uart_change_pm`, `uart_*` | `apis/tty.md` |
| VFS Inode, Dentry, Permission, and `file_operations` API Guide (Linux v7.3-rc5) | `inode_lock`, `inode_lock_nested`, `I_MUTEX_NORMAL`, `I_MUTEX_PARENT`, `I_MUTEX_CHILD`, `I_MUTEX_XATTR`, `I_MUTEX_NONDIR2`, `I_MUTEX_PARENT2`, `vfs_*` | `apis/vfs.md` |
| Workqueue Lifecycle & Teardown API (`include/linux/workqueue.h`, `kernel/workqueue.c`) | `work_struct`, `queue_work`, `schedule_work`, `flush_work`, `cancel_work_sync`, `disable_work_sync`, `flush_workqueue`, `destroy_workqueue` | `apis/workqueue.md` |
| XArray and Maple Tree API Guide (Linux v7.3-rc5) | `xa_init`, `xa_init_flags`, `xa_destroy`, `xa_load`, `xa_store`, `xa_erase`, `xa_cmpxchg`, `xa_insert`, `xa_*`, `xas_*` | `apis/xarray-maple.md` |
