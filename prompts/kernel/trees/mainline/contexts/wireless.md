# Wireless (`mac80211`) MLO Callback Architecture (`net/mac80211/`, `drivers/net/wireless/`)

## `vif_cfg_changed()` vs `link_info_changed()` (`BSS_CHANGED_VIF_CFG_FLAGS`)

In `struct ieee80211_ops` (`include/net/mac80211.h`), Multi-Link Operation (MLO) splits the legacy `.bss_info_changed()` callback into two callbacks partitioned by `BSS_CHANGED_VIF_CFG_FLAGS` (`net/mac80211/main.c`):
- **`vif_cfg_changed(hw, vif, changed)`:** Handles VIF-global configuration changes (`struct ieee80211_vif_cfg` on `vif->cfg`). Passing any flag outside `BSS_CHANGED_VIF_CFG_FLAGS` to `ieee80211_vif_cfg_change_notify()` triggers `WARN_ON_ONCE(changed & ~BSS_CHANGED_VIF_CFG_FLAGS)`.
- **`link_info_changed(hw, vif, info, changed)`:** Handles per-link configuration changes (`struct ieee80211_bss_conf`). Passing any `BSS_CHANGED_VIF_CFG_FLAGS` bit to `ieee80211_link_info_change_notify()` triggers `WARN_ON_ONCE(changed & BSS_CHANGED_VIF_CFG_FLAGS)`.

| Callback | `BSS_CHANGED_*` Flags | State Location |
| :--- | :--- | :--- |
| `vif_cfg_changed()` (`BSS_CHANGED_VIF_CFG_FLAGS`) | `BSS_CHANGED_ASSOC`, `BSS_CHANGED_IDLE`, `BSS_CHANGED_PS`, `BSS_CHANGED_IBSS`, `BSS_CHANGED_ARP_FILTER`, `BSS_CHANGED_SSID`, `BSS_CHANGED_MLD_VALID_LINKS`, `BSS_CHANGED_MLD_TTLM` | `vif->cfg.*`, `vif->valid_links` |
| `link_info_changed()` (`~BSS_CHANGED_VIF_CFG_FLAGS`) | `BSS_CHANGED_BSSID`, `BSS_CHANGED_BEACON_INFO`, `BSS_CHANGED_BEACON`, `BSS_CHANGED_BEACON_INT`, `BSS_CHANGED_ERP_CTS_PROT`, `BSS_CHANGED_ERP_PREAMBLE`, `BSS_CHANGED_ERP_SLOT`, `BSS_CHANGED_HT`, `BSS_CHANGED_BASIC_RATES`, `BSS_CHANGED_TXPOWER`, `BSS_CHANGED_BANDWIDTH`, `BSS_CHANGED_HE_BSS_COLOR`, `BSS_CHANGED_TPE`, etc. | `struct ieee80211_bss_conf` (`link_conf`) |
