# Bluetooth Subsystem Context (`net/bluetooth/`)

## Advertisement Instance `0x00` State Tracking (`hci_core.c`, `hci_event.c`)

Bluetooth HCI supports multiple advertisement instances, but legacy instance `0x00` is tracked differently from dynamic instances (`>= 1`):

- Instance `0x00` is **never** added to `hdev->adv_instances` (`hci_add_adv_instance()` rejects `instance < 1`).
- `hdev->cur_adv_instance == 0` only indicates which instance is currently selected, **not** whether instance `0x00` is actively advertising.
- `hci_dev_test_flag(hdev, HCI_LE_ADV)` indicates whether *any* LE advertising is active across the controller, not specifically instance `0x00`.
- Query instance `0x00`'s enabled state exclusively via `hci_dev_test_flag(hdev, HCI_LE_ADV_0)` (set in `hci_cc_le_set_ext_adv_enable()` when `set->handle == 0` and no `adv_info` exists, and cleared on disable).

## MGMT Pending Command Lifecycle (`net/bluetooth/mgmt_util.c`)

`mgmt_pending_valid(hdev, cmd)` atomically checks whether `cmd` is still on the pending list **and** unlinks it via `list_del()`.

- Once `mgmt_pending_valid(hdev, cmd)` returns `true`, the completion callback owns `cmd` and **must** call `mgmt_pending_free(cmd)` on **every** exit path (both error and success paths).
- **Never** call `mgmt_pending_remove(cmd)` after `mgmt_pending_valid(hdev, cmd)` succeeds: `mgmt_pending_remove()` performs another `list_del()` followed by `mgmt_pending_free()`, causing a double `list_del()` and double free.
- If `err == -ECANCELED` or `!mgmt_pending_valid(hdev, cmd)`, the callback does not own `cmd` and must return without freeing it.

## Variable-Length MGMT Command Payloads (`include/net/bluetooth/mgmt.h`)

Several MGMT command structs end with flexible array members (e.g., `struct mgmt_cp_set_mesh` (`ad_types[]`), `struct mgmt_cp_load_irks` (`irks[]`), `struct mgmt_cp_load_long_term_keys` (`keys[]`)).

- Never copy `cmd->param` into a stack-allocated struct instance with `memcpy(&cp, cmd->param, sizeof(cp))` and then index the flexible array member (`sizeof(cp)` excludes the flexible array, causing stack out-of-bounds access).
- Either read directly from `cmd->param` or allocate stack storage using `DEFINE_FLEX()` (`include/linux/overflow.h`) and bound the copy with `min(__struct_size(cp), len)`.
