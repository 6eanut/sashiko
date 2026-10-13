# Netlink & Generic Netlink Maintainer Notes

## New Generic Netlink Family Design Rules

The following rules from `Documentation/userspace-api/netlink/` apply to **new Generic Netlink families** (do not enforce them on extensions to existing legacy families that already follow older conventions):
- **Numbering and IDs:** Start the first attribute ID and first command ID at `1` (do not define an `unspec = 0` attribute or command). Use the `unified` message-ID model (never `directional`). Use the same command ID for a request and its reply, and use a separate command ID for each notification.
- **No multi-message `do` replies:** A `do` operation must never reply with multiple messages (`NLM_F_MULTI`); use a filtered `dump` operation instead.
- **Full replies on creation (`NEW`/`ADD`):** Operations that create objects must reply with a full message carrying the allocated identifier rather than only returning an ACK or relying on `NLM_F_ECHO`.
- **Policy scope:** Use `per-op` (default) or `split` `kernel-policy`, never `global`, and reject unknown attributes by default. Prefer declarative `NLA_POLICY_*` macros over open-coded attribute validation.
- **Deprecated flags:** Do not introduce new uses of legacy request-type flags (`NLM_F_REPLACE`, `NLM_F_EXCL`, `NLM_F_CREATE`, `NLM_F_APPEND`, `NLM_F_NONREC`, `NLM_F_BULK`, `NLM_F_ATOMIC`, `NLM_F_ROOT`, `NLM_F_MATCH`) outside legacy families.

## Attribute Design Preferences

- **Arrays:** Prefer repeated (`multi-attr`) attributes for arrays over extra wrapping nests, `indexed-array`, or `type-value` encodings.
- **No binary C structs in attributes:** Decompose each field into its own netlink attribute instead of embedding binary structures inside attributes.
- **Integer widths:** Prefer variable-width `uint` / `sint` or 32/64-bit integer types; avoid `u8` / `u16` attributes (which save no wire space due to 4-byte netlink alignment) unless the value is a fixed-width protocol header field.

## YAML Specification Conventions (`Documentation/netlink/specs/`)

- **License and self-containment:** Use license `((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`. Keep specs self-contained (use `header` only for shared UAPI constants such as `IFNAMSIZ`), include `doc` strings on major properties, and use dashes (`-`) rather than underscores (`_`) in YAML identifiers.
- **Attribute sets and notifications:** Define an attribute's `value` only in its main attribute set (never in a `subset-of` fractional set). Prefer `notify:` (sharing structure with a `GET` op) over `event:` unless the notification carries data that can never be queried via `GET`.
- **Fixed C structs in YAML:** YAML struct definitions are implicitly packed; include explicit padding members required for natural alignment.
