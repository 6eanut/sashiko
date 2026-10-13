# Subsystem Guide Index

Load subsystem guides from the prompt directory based on what the code
touches. Each guide contains subsystem-specific invariants, API
contracts, and common bug patterns. Each subsystem guide may reference
additional pattern files to load conditionally.

A change can match multiple rows. Load **every** matching guide, not
just the deepest or most specific. For example, code touching
`arch/arm64/kvm/hyp/` matches the ARM64, KVM, ARM64 KVM (EL1/Host),
and ARM64 Hyp (EL2) rows — all four guides apply.

The triggers column below includes both path names, function calls,
and symbols regexes.

## Subsystem Guides

| Subsystem | Triggers | File |
|-----------|----------|------|
| VFS | inode, dentry, vfs_, fs/*.c | vfs.md |
| Workqueue | kernel/workqueue.c, work_struct | workqueue.md |
| Wireless/mac80211 | drivers/net/wireless/, net/mac80211/, BSS_CHANGED_, vif_cfg_changed, link_info_changed, bss_info_changed | wireless.md |


