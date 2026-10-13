# Networking Drivers Maintainer Notes (`drivers/net/`)

## Ethtool Driver-Private Statistics (`ethtool -S`) vs Standard uAPIs

- **No duplicate standard counters in `ethtool -S`:** The `ethtool -S` string table (`get_ethtool_stats()`, `get_sset_count()`, `get_strings()`) is strictly for driver- or hardware-specific counters that have no standard kernel representation.
- Counters covered by a standard kernel uAPI must be exposed through that standard interface rather than added to `ethtool -S` (see `Documentation/networking/statistics.rst`), including:
  - Device-level statistics (`struct rtnl_link_stats64` via `ndo_get_stats64`)
  - Per-queue RX/TX statistics (`struct netdev_queue_stats_rx`, `struct netdev_queue_stats_tx` via netlink)
  - Page pool statistics (via netlink)
  - Standard IEEE / MAC / pause / PHY / FEC / RMON statistics (dedicated callbacks in `struct ethtool_ops`)
- Only flag patches that add *new* `ethtool -S` strings duplicating a standard uAPI counter; pre-existing legacy `ethtool -S` entries in untouched code are not bugs in new patches.

## No Ad-Hoc Synchronization with Flags, Atomics, or Trylock Loops

- **Do not use flags or atomics as locks:** Do not use `atomic_t`, bitops (`set_bit()` / `test_bit()` / `clear_bit()`), or boolean flags in a set-on-entry / clear-on-exit pattern to guard code sections or coordinate readers against device teardown. Such homebrew schemes lack mutual exclusion and memory ordering and are invisible to lockdep. Use proper kernel synchronization primitives instead:
  - RCU for reader-heavy paths (such as `ndo_get_stats64`)
  - `struct mutex` for sleeping mutual exclusion
  - `spinlock_t` for atomic-context mutual exclusion
  - `del_timer_sync()` / `timer_delete_sync()` / `cancel_work_sync()` to drain asynchronous timers and work items
- **No open-coded trylock retry loops:** Do not use `mutex_trylock()` or `spin_trylock()` in open-coded spin/retry loops or to silently skip required work to work around lock-ordering bugs. (`mutex_trylock()` is acceptable when a work item reschedules itself via `schedule_work()` / `schedule_delayed_work()` on contention.)
