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
| Scheduler | kernel/sched/, sched_, schedule, *wakeup* | scheduler.md |
| Timers | timer_list, timer_setup, mod_timer, del_timer, hrtimer, delayed_work | timers.md |
| RCU | rcu*, call_rcu, synchronize_rcu, kfree_rcu, kvfree_call_rcu | rcu.md |
| Tracing | trace_, tracepoints | tracing.md |
| Workqueue | kernel/workqueue.c, work_struct | workqueue.md |
| Syscalls | `SYSCALL_DEFINE`, `copy_from_user`, `copy_to_user`, `get_user`, `put_user`, any change to syscall parameter validation | syscall.md |
| SunRPC | net/sunrpc/* | sunrpc.md |
| RCU lifecycle | `call_rcu(`, `kfree_rcu(`, `synchronize_rcu(`, `rhashtable_*` + `call_rcu`, `hlist_del_rcu` + `call_rcu`, `list_del_rcu` + `call_rcu` | rcu.md |
| Power Domains | drivers/pmdomain/, pm_genpd_, of_genpd_, exynos_pd_ | pmdomain.md |
| PM Runtime | include/linux/pm_runtime.h, pm_runtime_, __pm_runtime_, rpm_idle, rpm_suspend, rpm_resume | pm.md |
| Sysfs | fs/sysfs/, sysfs_create_group, sysfs_update_group, attribute_group, is_visible | sysfs.md |
| TTY/Serial | drivers/tty/, uart_add_one_port, uart_ops, serial_core | tty.md |
| SMB/ksmbd | fs/smb/server/, ksmbd_, smb_direct_ | smb-ksmbd.md |
| Perf Tools | tools/perf/, openat, fdopendir, closedir | perf.md |
| Wireless/mac80211 | drivers/net/wireless/, net/mac80211/, BSS_CHANGED_, vif_cfg_changed, link_info_changed, bss_info_changed | wireless.md |
| Selftests | tools/testing/selftests/, TEST_PROGS, TEST_FILES, TEST_GEN_FILES | selftests.md |
| USB Storage | drivers/usb/storage/, unusual_devs.h, UNUSUAL_DEV, USB_SC_, USB_PR_ | usb-storage.md |
| Rust | any Rust code | rust.md |

## Optional Patterns

Load only when explicitly requested in the prompt:

- **Subjective Review** (subjective-review.md): Subjective general assessment
