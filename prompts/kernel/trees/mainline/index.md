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
| HID Subsystem | `drivers/hid/`, `include/linux/hid.h`, `hid_device`, `hid_driver`, `hid_ll_driver`, `report_fixup` | `contexts/hid.md` |
| ARM64 Hyp (EL2 / pKVM / nVHE) | `arch/arm64/kvm/hyp/`, `pkvm_`, `__pkvm_`, `kern_hyp_va`, `hyp_pool`, `hyp_fixmap_`, `hyp_vmemmap`, `flush_hyp_vcpu` | `contexts/hyp-arm64.md` |
| I2C Subsystem | `drivers/i2c/`, `include/linux/i2c.h`, `i2c_client`, `i2c_adapter` | `contexts/i2c.md` |
| Input Subsystem | `drivers/input/`, `include/linux/input.h`, `input_dev`, `input_handler`, `evdev` | `contexts/input.md` |
| io_uring | `io_uring/`, `io_uring_`, `io_ring_`, `io_sq_`, `io_cq_`, `io_wq_`, `io_kiocb`, `IORING_` | `contexts/io_uring.md` |
| Kconfig | `Kconfig`, `config `, `select `, `depends on `, `COMPILE_TEST` | `contexts/kconfig.md` |
| Kexec Handover (KHO) | `kernel/liveupdate/`, `kexec_handover`, `kho_` | `contexts/kho.md` |
| ARM64 KVM (Host / EL1) | `arch/arm64/kvm/`, `vcpu_has_run_once`, `kvm_vcpu_initialized`, `vcpu_sysreg`, `user_mem_abort`, `ICH_HCR_EL2` | `contexts/kvm-arm64.md` |
| KVM Core & x86 MMU | `virt/kvm/`, `arch/x86/kvm/`, `kvm_mmu_`, `is_page_fault_stale`, `mmu_invalidate_seq`, `gfn_to_pfn_cache`, `slots_lock` | `contexts/kvm.md` |
| libbpf Public API | `tools/lib/bpf/`, `LIBBPF_API`, `libbpf_err`, `libbpf_err_ptr`, `libbpf_ptr` | `contexts/libbpf.md` |
| Media / V4L2 Subdevice Core | `drivers/media/`, `include/media/`, `v4l2_subdev_`, `V4L2_SUBDEV_`, `MEDIA_BUS_FMT_` | `contexts/media.md` |
| MIPS Architecture | `arch/mips/`, `tlb_probe`, `tlb_read`, `tlb_write_indexed`, `write_c0_entryhi`, `TLBP`, `TLBR`, `TLBWI` | `contexts/mips.md` |
| MM Page Allocator, SLUB & vmalloc | `mm/page_alloc.c`, `mm/slub.c`, `mm/vmalloc.c`, `get_page_from_freelist`, `lowmem_reserve`, `allocate_slab`, `pcp_spin_` | `contexts/mm-alloc.md` |
| MM Folio Lifecycle & Page Cache | `mm/filemap.c`, `mm/swap.c`, `mm/truncate.c`, `page_folio`, `filemap_`, `mapping_set_update`, `folio_set_swapbacked`, `pfn_to_online_page` | `contexts/mm-folio.md` |
| MM Large Folios, THP, Hugetlb & HWPoison | `mm/huge_memory.c`, `mm/hugetlb.c`, `mm/memory-failure.c`, `hugetlb`, `hstate`, `huge_pmd_`, `memory_failure`, `deferred_split_folio` | `contexts/mm-largepage.md` |
| MM Page Table Internals | `mm/memory.c`, `mm/mprotect.c`, `mm/pagewalk.c`, `mm/pt_reclaim.c`, `zap_pte_range`, `change_pte_range`, `try_to_free_pte`, `flush_tlb_batched_pending` | `contexts/mm-pagetable.md` |
| MM Reclaim, Swap & Migration | `mm/vmscan.c`, `mm/swap_state.c`, `mm/migrate.c`, `mm/memcontrol.c`, `shrink_folio_list`, `evict_folios`, `LRU_GEN_MASK`, `dtc_dom`, `TTU_RMAP_LOCKED` | `contexts/mm-reclaim.md` |
| MM VMA Internals | `mm/vma.c`, `mm/mmap.c`, `mm/mmap_lock.c`, `lock_vma_under_rcu`, `vma_prepare`, `vma_complete`, `vma_modify`, `vma_merge_`, `call_mmap_prepare` | `contexts/mm-vma.md` |
| Networking Core Packet Flow & XFRM | `net/core/`, `net/xfrm/`, `skb->cb`, `destructor_arg`, `xfrm_state`, `inner_mode`, `virtio_net_hdr`, `PACKET_*` | `contexts/networking-core.md` |
| NFS Server (NFSD) Subsystem | `fs/nfsd/`, `fs/lockd/`, `fh_verify`, `nfsd4_`, `nfs4_stid`, `nfs4_preprocess_stateid_op`, `nfsd_user_namespace`, `nfserrno` | `contexts/nfsd.md` |
| Objtool Control-Flow Validation | `tools/objtool/`, `INSN_BUG`, `INSN_TRAP`, `decode.c`, `validate_sls`, `ignore_unreachable_insn` | `contexts/objtool.md` |
| Open Firmware (DT) Core & MSI | `drivers/of/`, `of_msi_xlate`, `of_msi_get_domain`, `msi-map`, `msi-parent` | `contexts/of.md` |
| Perf Tools (`tools/perf/`) | `tools/perf/`, `perf_tool`, `perf_env`, `DECLARE_RC_STRUCT`, `thread__get`, `maps__get`, `fdopendir` | `contexts/perf.md` |
| Generic Power Domains (`genpd`) | `drivers/pmdomain/`, `pm_genpd_`, `of_genpd_`, `GENPD_FLAG_NO_STAY_ON`, `exynos_pd_` | `contexts/pmdomain.md` |
| Kernel Rust Toolchain & Pin-Init | `rust/`, `*.rs`, `__rust_helper`, `rust_helper_`, `RUST_CONST_HELPER_`, `try_pin_init`, `pin_init`, `build_assert!` | `contexts/rust.md` |
| Scheduler Internals (EEVDF, RT, DL, `sched_ext`) | `kernel/sched/`, `double_rq_lock`, `__pick_eevdf`, `update_load_avg`, `sched_cpu_dying`, `dl_server_` | `contexts/scheduler.md` |
| Kernel Selftests (`tools/testing/selftests/`) | `tools/testing/selftests/`, `TEST_PROGS`, `TEST_FILES`, `TEST_GEN_FILES`, `vm_create_with_one_vcpu`, `devconf_inherit_init_net` | `contexts/selftests.md` |
| SMBDirect (RDMA) & ksmbd | `fs/smb/smbdirect/`, `fs/smb/server/`, `smbdirect_`, `smb_direct_`, `ksmbd_`, `refill_work`, `immediate_work` | `contexts/smb-ksmbd.md` |
| SunRPC Transport & GSS (`net/sunrpc/`) | `net/sunrpc/`, `svc_xprt_`, `svc_rqst`, `rpc_task`, `xprtrdma`, `rpcrdma_`, `svc_rdma_`, `gss_svc_` | `contexts/sunrpc.md` |
| Sysfs Attribute Groups (`fs/sysfs/`) | `fs/sysfs/`, `sysfs_group_attrs_change_owner`, `kernfs_find_and_get`, `SYSFS_GROUP_INVISIBLE` | `contexts/sysfs.md` |
| TTY & 8250 Serial Architecture | `drivers/tty/`, `8250_base`, `8250_port`, `8250_core`, `serial_core` | `contexts/tty.md` |
| USB Mass Storage (`unusual_devs.h`) | `drivers/usb/storage/`, `unusual_devs.h`, `UNUSUAL_DEV`, `USB_SC_`, `USB_PR_` | `contexts/usb-storage.md` |
| VFS Path Walk, Open & Superblock | `fs/namei.c`, `fs/open.c`, `fs/dcache.c`, `LOOKUP_RCU`, `try_to_unlazy`, `do_dentry_open`, `s_umount` | `contexts/vfs.md` |
| Wireless (`mac80211`) MLO Callbacks | `drivers/net/wireless/`, `net/mac80211/`, `BSS_CHANGED_*`, `vif_cfg_changed`, `link_info_changed`, `bss_info_changed` | `contexts/wireless.md` |

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
| GPIO Consumer & Driver API | `linux/gpio.h`, `gpio_get_value`, `gpio_set_value`, `gpiod_get_raw_value`, `gpiod_set_raw_value`, `gpio_chip`, `GPIO_GENERIC`, `GPIO_REGMAP` | `apis/gpio.md` |
| HID Transport & Driver API | `hid_allocate_device`, `hid_add_device`, `hid_destroy_device`, `hid_parse`, `hid_hw_start`, `hid_hw_stop`, `hid_device_io_start`, `hid_safe_input_report`, `hid_is_usb` | `apis/hid.md` |
| Hardware Monitoring API | `hwmon_device_register_with_info`, `devm_hwmon_device_register_with_info`, `hwmon_lock`, `hwmon_unlock` | `apis/hwmon.md` |
| I2C Transfer & DMA API | `i2c_transfer`, `i2c_master_send`, `i2c_master_recv`, `i2c_master_send_dmasafe`, `i2c_master_recv_dmasafe`, `I2C_M_DMA_SAFE`, `i2c_smbus_*`, `i2c_get_dma_safe_msg_buf`, `i2c_put_dma_safe_msg_buf` | `apis/i2c.md` |
| Input Device & FF API | `input_allocate_device`, `devm_input_allocate_device`, `input_register_device`, `input_free_device`, `input_device_enabled`, `input_report_*`, `input_sync`, `input_ff_create_memless` | `apis/input.md` |
| MMIO Stream & Register Accessors | `writesl`, `readsl`, `writesw`, `readsw`, `writesb`, `readsb`, `__raw_writel`, `__raw_readl` | `apis/io-accessors.md` |
| Kexec Handover (KHO) API | `kho_is_enabled`, `is_kho_boot`, `kho_add_subtree`, `kho_remove_subtree`, `kho_retrieve_subtree`, `kho_preserve_folio`, `kho_preserve_pages`, `kho_alloc_preserve` | `apis/kho.md` |
| KVM Core Helpers | `gfn_to_hva`, `gfn_to_hva_memslot`, `gfn_to_hva_memslot_prot`, `kvm_vcpu_memslots`, `vcpu_load`, `vcpu_put`, `kvm_make_request`, `kvm_check_request` | `apis/kvm.md` |
| LED Class & Trigger API | `led_classdev_register`, `led_classdev_unregister`, `devm_led_classdev_register`, `devm_led_classdev_register_ext`, `led_trigger_register`, `devm_led_trigger_register` | `apis/leds.md` |
| Kernel Locking & Memory Ordering | `spin_lock*`, `raw_spin_lock*`, `local_lock`, `mutex_*`, `rwsem*`, `seqlock*`, `seqcount*`, `lockdep_pin_lock`, `cpus_read_lock`, `smp_store_release`, `smp_load_acquire` | `apis/locking.md` |
| Multi-Function Device (MFD) API | `mfd_add_devices`, `devm_mfd_add_devices`, `mfd_remove_devices`, `mfd_cell`, `PLATFORM_DEVID_AUTO` | `apis/mfd.md` |
| Memory Allocation, GFP & Slab API | `alloc_pages`, `__GFP_*`, `kmalloc`, `kzalloc`, `kmem_cache_*`, `vmalloc`, `mempool_alloc`, `memblock_*`, `gfpflags_allow_blocking`, `ARCH_KMALLOC_MINALIGN` | `apis/mm-alloc.md` |
| Folio, Page Cache & XArray API | `folio_*`, `page_folio`, `folio_expected_ref_count`, `folio_end_read`, `kmap_local_folio`, `find_get_entries`, `xas_next`, `xas_advance`, `xa_get_order` | `apis/mm-folio.md` |
| Large Folio, THP & Hugetlb API | `split_folio`, `split_huge_page`, `try_folio_split_to_order`, `min_order_for_split`, `folio_put_refs`, `folio_hstate`, `is_vm_hugetlb_page`, `pmd_trans_huge`, `PageHWPoison` | `apis/mm-largepage.md` |
| Page Table Accessors, Batching & Walkers | `pte_*`, `pmd_*`, `pud_*`, `set_ptes`, `folio_pte_batch`, `folio_pte_batch_flags`, `folio_mk_pmd`, `softleaf_*`, `page_vma_mapped_walk`, `walk_page_range`, `pte_offset_map_lock`, `pages_identical` | `apis/mm-pagetable.md` |
| Writeback, Swap, Memcg & Migration API | `folio_start_writeback`, `wbc_to_tag`, `PAGECACHE_TAG_*`, `folio_test_swapbacked`, `folio_test_swapcache`, `get_swap_device`, `mem_cgroup_*`, `folio_memcg`, `migrate_folio`, `filemap_migrate_folio` | `apis/mm-reclaim.md` |
| VMA Classification, Flags & Locking API | `vma_is_anonymous`, `vma_set_anonymous`, `vm_flags_*`, `vma_start_write`, `mmap_read_lock_killable`, `vma_assert_locked`, `vma_interval_tree_foreach`, `find_vm_area`, `memfd_alloc_file`, `security_vm_enough_memory_mm` | `apis/mm-vma.md` |
| Netlink & Generic Netlink API | `genl_notify`, `nla_*`, `NLA_*`, `NLM_F_*`, `nlmsg_*`, `netlink_callback`, `NL_SET_BAD_ATTR`, `NL_SET_ERR_ATTR_MISS` | `apis/netlink.md` |
| SKB, Socket, Netfilter, Routing & SNMP API | `skb_put`, `skb_push`, `skb_pull`, `pskb_may_pull`, `skb_unshare`, `release_sock`, `sock_release`, `sock_hold`, `sock_put`, `NF_HOOK`, `skb_dst_force`, `SNMP_*` | `apis/networking-core.md` |
| 64-Bit Network Driver Statistics (`u64_stats_sync`) | `u64_stats_sync`, `u64_stats_init`, `u64_stats_update_begin`, `u64_stats_update_begin_irqsave`, `u64_stats_fetch_begin`, `u64_stats_fetch_retry` | `apis/networking-drivers.md` |
| Open Firmware (DT) Node & Iterator API | `for_each_child_of_node*`, `for_each_available_child_of_node*`, `of_node_put`, `of_node_get`, `of_parse_phandle`, `of_get_parent`, `of_get_next_parent`, `of_find_*` | `apis/of.md` |
| PCI Endpoint & IRQ Vector API | `pci_epc_get`, `pci_epf_create`, `pci_epf_destroy`, `pci_alloc_irq_vectors`, `pci_free_irq_vectors`, `pci_enable_msi` | `apis/pci.md` |
| Power Management & Runtime PM API | `pm_runtime_*`, `__pm_runtime_*`, `rpm_idle`, `rpm_suspend`, `rpm_resume`, `pm_hibernate_is_recovering`, `pm_hibernation_mode_is_suspend`, `pm_sleep_ptr`, `pm_ptr` | `apis/pm.md` |
| Read-Copy-Update (RCU) API | `rcu_read_lock`, `rcu_assign_pointer`, `rcu_dereference`, `call_rcu`, `synchronize_rcu`, `kfree_rcu`, `kvfree_call_rcu`, `list_del_rcu`, `hlist_del_rcu` | `apis/rcu.md` |
| Task State, Wakeup & Migration API | `set_current_state`, `schedule`, `wake_up_process`, `try_to_wake_up`, `migrate_disable`, `set_cpus_allowed_ptr`, `kthread_bind` | `apis/scheduler.md` |
| Syscall Parameter Validation & User Copy API | `SYSCALL_DEFINE*`, `copy_from_user`, `copy_to_user`, `copy_struct_from_user`, `get_user`, `put_user` | `apis/syscall.md` |
| Sysfs Group & Kobject Lifecycle API | `sysfs_create_group`, `sysfs_update_group`, `attribute_group`, `is_visible`, `is_bin_visible`, `SYSFS_GROUP_INVISIBLE`, `kobject_create_and_add`, `kobject_put` | `apis/sysfs.md` |
| Kernel Timers (`timer_list`, `hrtimer`, `delayed_work`) | `timer_setup`, `mod_timer`, `timer_delete_sync`, `timer_shutdown_sync`, `hrtimer_setup`, `hrtimer_cancel`, `delayed_work`, `cancel_delayed_work_sync` | `apis/timers.md` |
| Kernel Tracepoints & `TRACE_EVENT` API | `TRACE_EVENT*`, `TP_fast_assign`, `__string`, `__assign_str`, `CREATE_TRACE_POINTS`, `tracepoint_probe_register`, `tracepoint_synchronize_unregister`, `rv_attach_trace_probe` | `apis/tracing.md` |
| Serial Core & UART Registration API | `uart_add_one_port`, `uart_remove_one_port`, `uart_ops`, `uart_configure_port`, `uart_change_pm` | `apis/tty.md` |
| VFS Inode, Dentry & `file_operations` API | `inode_lock_nested`, `I_MUTEX_PARENT`, `lock_rename`, `d_instantiate`, `d_instantiate_new`, `d_add`, `d_unhashed`, `inode_permission`, `replace_fops`, `vfs_*` | `apis/vfs.md` |
