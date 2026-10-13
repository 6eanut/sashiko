# Bluetooth Subsystem Context (`net/bluetooth/`)

This guide provides authoritative technical invariants for the Linux Bluetooth subsystem (`net/bluetooth/`, `drivers/bluetooth/`) as of v7.3-rc5.

## 1. Advertisement Instance Management (`hci_core.c`, `hci_event.c`)

Bluetooth HCI supports multiple advertisement instances, but legacy instance `0x00` is tracked differently from dynamic instances (`>= 1`):

- Instance `0x00` is **never** added to `hdev->adv_instances` (`hci_add_adv_instance()` rejects `instance < 1`).
- `hdev->cur_adv_instance == 0` only indicates which instance is currently selected, **not** whether instance `0x00` is actively advertising.
- `hci_dev_test_flag(hdev, HCI_LE_ADV)` indicates whether *any* LE advertising is active across the controller, not specifically instance `0x00`.
- Query instance `0x00`'s enabled state exclusively via `hci_dev_test_flag(hdev, HCI_LE_ADV_0)` (set in `hci_cc_le_set_ext_adv_enable()` when `set->handle == 0` and no `adv_info` exists, and cleared on disable).

## 2. MGMT Pending Command Lifecycle (`net/bluetooth/mgmt_util.c`)

`mgmt_pending_valid(hdev, cmd)` atomically checks whether `cmd` is still on the pending list **and** unlinks it via `list_del()`.

- Once `mgmt_pending_valid(hdev, cmd)` returns `true`, the completion callback owns `cmd` and **must** call `mgmt_pending_free(cmd)` on **every** exit path (both error and success paths).
- **Never** call `mgmt_pending_remove(cmd)` after `mgmt_pending_valid(hdev, cmd)` succeeds: `mgmt_pending_remove()` performs another `list_del()` followed by `mgmt_pending_free()`, causing a double `list_del()` and double free.
- If `err == -ECANCELED` or `!mgmt_pending_valid(hdev, cmd)`, the callback does not own `cmd` and must return without freeing it.

## 3. Variable-Length MGMT Command Payloads (`include/net/bluetooth/mgmt.h`)

Several MGMT command structs end with flexible array members (e.g., `struct mgmt_cp_set_mesh` (`ad_types[]`), `struct mgmt_cp_load_irks` (`irks[]`), `struct mgmt_cp_load_long_term_keys` (`keys[]`)).

- Never copy `cmd->param` into a stack-allocated struct instance with `memcpy(&cp, cmd->param, sizeof(cp))` and then index the flexible array member (`sizeof(cp)` excludes the flexible array, causing stack out-of-bounds access).
- Either read directly from `cmd->param` or allocate stack storage using `DEFINE_FLEX()` (`include/linux/overflow.h`) and bound the copy with `min(__struct_size(cp), len)`.

## 4. Synchronization and Concurrency

- **Lock Hierarchy:** Always acquire `hdev->lock` before nested subsystem locks. Never perform blocking operations (e.g., `msleep()`, `mutex_lock()`) while holding `hdev->lock` or within `spin_lock_bh()` contexts.
- **Atomic Context:** Use `GFP_ATOMIC` for all memory allocations within `hci_dev` callbacks or interrupt handlers.
- **RCU Usage:** When traversing `hdev->adv_instances` or connection lists, ensure `rcu_read_lock()` is held. Use `rcu_dereference()` to access pointers and `synchronize_rcu()` or `call_rcu()` for safe reclamation.

## 5. Memory Safety and Packet Handling

- **SKB Validation:** Before accessing or trimming headers (e.g., RFCOMM frames, L2CAP PDUs), **always** call `pskb_may_pull(skb, len)`. Failure to do so leads to kernel panics on non-linear buffers.
- **Bounds Checking:** Every length field received from hardware or userspace must be validated against `HCI_MAX_FRAME_SIZE` or the destination buffer size.
- **EIR Parsing:** Use `eir_get_data()` for parsing EIR/AD structures; it handles length validation and field iteration safely.

## 6. Teardown and Resource Unwinding

- **Quiescing:** Before freeing an object (e.g., `hci_dev`), synchronously cancel all associated work items (`cancel_work_sync()`) and timers.
- **Refcount Balance:** Every `hci_dev_hold()` or `mgmt_pending_add()` must have a corresponding `hci_dev_put()` or `mgmt_pending_free()` on all exit paths.
- **Devcoredump:** Use `hci_devcd_init()` and `hci_devcd_append()` correctly; ensure `hci_devcd_complete()` is called on success and `hci_devcd_abort()` on failure.

## 7. High-Frequency Bug Patterns

| Bug Pattern | Prevention Strategy |
| :--- | :--- |
| **UAF on Teardown** | Ensure all `delayed_work` and `work_struct` are cancelled before `kfree()`. |
| **SKB Header Access** | Always use `pskb_may_pull()` before accessing fields beyond the linear buffer. |
| **Double-Free** | Check `mgmt_pending_valid()` before calling `mgmt_pending_free()`. |
| **Integer Overflow** | Validate all user-provided lengths against `HCI_MAX_FRAME_SIZE`. |
| **Atomic Context** | Use `GFP_ATOMIC` for all allocations in `hci_dev` callbacks. |
| **AOSP Extensions** | Use `aosp_has_quality_report()` to guard calls to `aosp_set_quality_report()`. |
