# Netlink & Generic Netlink API (`include/net/netlink.h`, `include/net/genetlink.h`)

## Notifications, Dumps, and Extended ACK

- **`genl_notify()` and `NLM_F_ECHO`:** When emitting a notification triggered by a user request, pass the request's `struct genl_info *` to `genl_notify()` so `NLM_F_ECHO` is honored.
- **Dump consistency (`NLM_F_DUMP_INTR`):** If iteration across `dumpit` calls can skip or duplicate objects (for example, over lockless or concurrently modified collections), maintain a generation sequence in `cb->seq` (`struct netlink_callback`) and set `NLM_F_DUMP_INTR` via `nl_dump_check_consistent()` when a concurrent modification occurs.
- **Extended ACK (`struct netlink_ext_ack`):** Prefer structured attribute error macros (`NL_SET_BAD_ATTR(extack, attr)`, `NL_SET_ERR_ATTR_MISS(extack, type)`) over plain-text messages; omit the text string when the bad/missing attribute pointer plus errno fully identifies the error. Extended ACK warnings can also be set on the success path.

## Attribute Accessors and Validation Policies (`nla_get_*`, `nla_put_*`, `nla_policy`)

- **Type agreement:** Every `nla_get_*()` and `nla_put_*()` call must match the width and signedness declared in the `struct nla_policy` table and the `Documentation/netlink/specs/` YAML specification.
- **String termination (`NLA_NUL_STRING`):** Strings should use `NLA_NUL_STRING` (`unterminated-ok` is only for legacy families). In both `struct nla_policy.len` and YAML `max-len`, the length bound excludes the trailing NUL byte (e.g., `IFNAMSIZ - 1`).
- **64-bit alignment padding:** When putting 64-bit integers (`nla_put_u64_64bit()` / `nla_put_s64()`) in families with unaligned legacy headers, specify the `pad` attribute (at most one `pad` attribute per attribute set).
- **`netlink-raw` sub-messages:** The selector attribute must appear in the message stream before the sub-message attribute; missing selectors at the expected nesting level must be rejected as an error.
