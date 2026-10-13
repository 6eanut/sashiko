# Wireless (`mac80211`) Subsystem Code Context Guide (v7.3-rc5)

This guide provides authoritative technical invariants for the `mac80211` subsystem. It is intended for automated patch review and architectural validation.

## 1. MLO Callback Architecture (`vif_cfg_changed` vs `link_info_changed`)

In `mac80211`, Multi-Link Operation (MLO) mandates a strict partition of configuration callbacks in `struct ieee80211_ops`.

- **`vif_cfg_changed(hw, vif, changed)`**: Handles `BSS_CHANGED_VIF_CFG_FLAGS`.
- **`link_info_changed(hw, vif, info, changed)`**: Handles per-link configuration changes (`struct ieee80211_bss_conf`).
- **`ieee80211_vif_cfg_change_notify`**: Helper to trigger VIF-global configuration updates. Passing any flag outside `BSS_CHANGED_VIF_CFG_FLAGS` triggers `WARN_ON_ONCE`.
- **`ieee80211_link_info_change_notify`**: Helper to trigger per-link configuration updates. Passing any `BSS_CHANGED_VIF_CFG_FLAGS` bit triggers `WARN_ON_ONCE`.

| Callback | `BSS_CHANGED_*` Flags | State Location |
| :--- | :--- | :--- |
| `vif_cfg_changed()` | `BSS_CHANGED_ASSOC`, `BSS_CHANGED_IDLE`, `BSS_CHANGED_PS`, `BSS_CHANGED_IBSS`, `BSS_CHANGED_ARP_FILTER`, `BSS_CHANGED_SSID`, `BSS_CHANGED_MLD_VALID_LINKS`, `BSS_CHANGED_MLD_TTLM` | `vif->cfg.*`, `vif->valid_links` |
| `link_info_changed()` | `BSS_CHANGED_BSSID`, `BSS_CHANGED_BEACON_INFO`, `BSS_CHANGED_BEACON`, `BSS_CHANGED_BEACON_INT`, `BSS_CHANGED_ERP_CTS_PROT`, `BSS_CHANGED_ERP_PREAMBLE`, `BSS_CHANGED_ERP_SLOT`, `BSS_CHANGED_HT`, `BSS_CHANGED_BASIC_RATES`, `BSS_CHANGED_TXPOWER`, `BSS_CHANGED_BANDWIDTH`, `BSS_CHANGED_HE_BSS_COLOR`, `BSS_CHANGED_TPE` | `struct ieee80211_bss_conf` (`link_conf`) |

## 2. Synchronization, Concurrency, and Lifetimes

- **Teardown**: Drivers must ensure all pending work items and timers are synchronously cancelled in the `stop()` callback. Use `cancel_work_sync()` and `timer_shutdown_sync()`.
- **Interrupt Context**: Only `ieee80211_tx_status_irqsafe()` and `ieee80211_rx_irqsafe()` are permitted in hardware interrupt context.
- **Workqueue**: Work items on the `mac80211` workqueue must **never** acquire the `RTNL` lock to avoid circular dependencies.
- **RCU**: Use `rcu_read_lock()`/`rcu_read_unlock()` for station list traversal. Do not use `rcu_dereference()` if the pointer is already locally available and protected by a higher-level lock.

## 3. Initialization and Registration

- **Premature Publication**: A device must be fully initialized (registers, DMA buffers, internal state) before calling `ieee80211_register_hw()`.
- **Error Unwind**: Use a symmetric cleanup pattern. If `probe()` fails, every resource allocated (e.g., `aead_key_setup_encrypt` via `aead_key_free`) must be released in reverse order.
- **State Flags**: Ensure internal state flags (e.g., `vif->cfg`) are initialized to safe defaults before any potential failure point.

## 4. Error Handling and Boundary Validation

- **Pointer Sentinel Validation**: Many `mac80211` helpers return `ERR_PTR` on failure. Always use `IS_ERR()` before dereferencing. Checking only for `NULL` is insufficient.
- **SKB Safety**: Before accessing packet headers, always validate the buffer length using `pskb_may_pull()`. Accessing fields (e.g., MIC, IV) without this check leads to out-of-bounds reads.
- **Arithmetic & Bounds**: When calculating buffer sizes for protocol elements, use `check_add_overflow()` or similar helpers for size calculations derived from external inputs.

## 5. High-Frequency Bug Patterns

| Bug Pattern | Prevention Strategy |
| :--- | :--- |
| **Use-after-free** | Use `timer_shutdown_sync()` and `cancel_work_sync()` in `stop()`. |
| **SKB OOB Access** | Always call `pskb_may_pull()` before header parsing. |
| **Unbalanced Refcount** | Ensure every `get` has a corresponding `put` on all error paths. |
| **Sleeping in Atomic** | Use `GFP_ATOMIC` if in an atomic context; otherwise, ensure no locks are held. |
| **IS_ERR Mismatch** | Explicitly check `IS_ERR(ptr)` before `if (ptr)`. |

## 6. Subsystem-Specific Invariants

- **Crypto API**: When using `aead_key_setup_encrypt`, the caller is responsible for calling `aead_key_free` on the returned `struct crypto_aead *` pointer.
- **Hardware Registers**: Always mask register reads to ignore reserved bits before performing logic or arithmetic.
- **Interface State**: Before invoking driver-specific operations, verify the interface is still present and active in the driver's internal tracking.
- **Driver Ops**: Ensure `wake_tx_queue` is implemented or linked to `ieee80211_handle_wake_tx_queue()`.
