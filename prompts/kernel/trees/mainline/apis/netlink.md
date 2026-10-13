# Netlink & Generic Netlink API (`include/net/netlink.h`, `include/net/genetlink.h`)

## Notifications, Dumps, and Extended ACK

- **`genl_notify()` and `NLM_F_ECHO`:** When emitting a notification triggered by a user request, pass the request's `struct genl_info *` to `genl_notify()` so `NLM_F_ECHO` is honored.
- **Dump consistency (`NLM_F_DUMP_INTR`):** If iteration across `dumpit` calls can skip or duplicate objects (for example, over lockless or concurrently modified collections), maintain a generation sequence in `cb->seq` (`struct netlink_callback`) and set `NLM_F_DUMP_INTR` via `nl_dump_check_consistent()` when a concurrent modification occurs.
- **Extended ACK (`struct netlink_ext_ack`):** Prefer structured attribute error macros (`NL_SET_BAD_ATTR(extack, attr)`, `NL_SET_ERR_ATTR_MISS(extack, type)`) over plain-text messages; omit the text string when the bad/missing attribute pointer plus errno fully identifies the error. Extended ACK warnings can also be set on the success path. Always initialize `struct netlink_ext_ack extack = {};` before use to prevent kernel stack information leaks or invalid pointer dereferences.

## Attribute Accessors and Validation Policies (`nla_get_*`, `nla_put_*`, `nla_policy`)

- **Type agreement:** Every `nla_get_*()` and `nla_put_*()` call must match the width and signedness declared in the `struct nla_policy` table and the `Documentation/netlink/specs/` YAML specification.
- **String termination (`NLA_NUL_STRING`):** Strings should use `NLA_NUL_STRING` (`unterminated-ok` is only for legacy families). In both `struct nla_policy.len` and YAML `max-len`, the length bound excludes the trailing NUL byte (e.g., `IFNAMSIZ - 1`).
- **64-bit alignment padding:** When putting 64-bit integers (`nla_put_u64_64bit()` / `nla_put_s64()`) in families with unaligned legacy headers, specify the `pad` attribute (at most one `pad` attribute per attribute set).
- **`netlink-raw` sub-messages:** The selector attribute must appear in the message stream before the sub-message attribute; missing selectors at the expected nesting level must be rejected as an error.

## Lifecycle, Concurrency, and Safety Invariants

- **Ownership:** `nlmsg_new()` allocates an `skb` owned by the caller. Transmission functions (`genl_notify()`, `nlmsg_unicast()`) transfer ownership to the netlink subsystem; do not call `kfree_skb()` after successful transmission. If a transmission function returns an error, the caller is responsible for freeing the `skb`.
- **Module Safety:** If performing asynchronous work or long-running `dumpit` operations, hold a module reference via `try_module_get(family->module)` to prevent unloading while the operation is in progress.
- **Locking:** If `parallel_ops` is set in `genl_family`, the core does not hold the global `genl_lock`. You must provide custom synchronization for shared state. Never perform blocking operations (e.g., `msleep`, `mutex_lock`) inside `doit` or `dumpit` callbacks if the family is not marked for parallel execution, as this may deadlock the netlink subsystem.
- **Validation:** Never trust `nla_data()` without prior `nla_parse()` or `nlmsg_parse()` validation against a policy. Use `nla_validate()` for standalone attribute validation.
- **Buffer Sizing:** Use `nlmsg_total_size()` and `nla_total_size()` for all buffer calculations to prevent overflows. Always account for `NLMSG_HDRLEN` and potential padding.
- **Teardown:** `genl_unregister_family()` must be the first step in module exit to prevent new requests during teardown.
- **Notification Errors:** Do not propagate errors from multicast notifications as the return value of a `doit` command; a command may succeed even if the notification fails.

## Advanced API Contract & Error Recovery

- **Return Conventions:** `doit` callbacks must return `0` on success or a negative error code (e.g., `-EINVAL`, `-EOPNOTSUPP`). Do not return positive values.
- **Partial Failure:** If a multi-step configuration fails, ensure all previously allocated resources (e.g., `nla_nest_start` buffers) are explicitly cancelled via `nlmsg_cancel()` if the message is not yet sent.
- **Memory Ordering:** When publishing state to shared structures, ensure appropriate memory barriers are used if the state is accessed by lockless readers.
- **Context Awareness:** Use `genl_info_net(info)` to retrieve the network namespace context. Ensure that family operations marked with `netnsok` correctly handle cross-namespace object references.
- **Input Sanitization:** Always validate user-provided lengths against `maxattr` and the actual buffer size to prevent out-of-bounds reads. Use `nla_len(attr)` to verify the payload size matches the expected type size.
- **Resource Cleanup:** Always ensure that `skb` buffers are freed on all error paths if the ownership has not been transferred to the netlink core. Use `kfree_skb()` for manual cleanup when `nlmsg_new()` fails or when a message is not sent.
- **Dump Termination:** Ensure `dumpit` callbacks return `0` when the dump is complete and a positive value (e.g., `len`) if the buffer is full and the dump must continue. Returning a negative error code terminates the dump and propagates the error to userspace.
- **Policy Enforcement:** For families with `resv_start_op`, ensure that all operations defined at or after this index provide a valid `nla_policy` to prevent the core from rejecting messages as unvalidated.
- **Attribute Nesting:** When using `nla_nest_start()`, always pair it with `nla_nest_end()` on success or `nla_nest_cancel()` on failure to maintain correct message structure and prevent malformed attribute chains.
- **Header Alignment:** Use `NLMSG_ALIGN()` and `NLA_ALIGN()` macros for all manual buffer calculations to ensure compliance with the 4-byte alignment requirement of the netlink protocol.
- **State Idempotency:** Ensure that `doit` handlers are idempotent where possible; if a configuration update fails, the system should remain in the previous valid state rather than an intermediate, partially-configured state.
- **Permission Checks:** Always verify `CAP_NET_ADMIN` or `CAP_SYS_ADMIN` for sensitive operations or multicast group binding using the `GENL_MCAST_CAP_*` flags in the `genl_multicast_group` definition.
