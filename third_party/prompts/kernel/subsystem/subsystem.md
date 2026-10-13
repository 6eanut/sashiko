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
| Timers | timer_list, timer_setup, mod_timer, del_timer, hrtimer, delayed_work | timers.md |
| Tracing | trace_, tracepoints | tracing.md |
| Workqueue | kernel/workqueue.c, work_struct | workqueue.md |
| TTY/Serial | drivers/tty/, uart_add_one_port, uart_ops, serial_core | tty.md |
| Wireless/mac80211 | drivers/net/wireless/, net/mac80211/, BSS_CHANGED_, vif_cfg_changed, link_info_changed, bss_info_changed | wireless.md |
| USB Storage | drivers/usb/storage/, unusual_devs.h, UNUSUAL_DEV, USB_SC_, USB_PR_ | usb-storage.md |

