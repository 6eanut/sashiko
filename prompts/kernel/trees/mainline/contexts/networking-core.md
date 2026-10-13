# Networking Core Packet Flow, SKB Control Block, and XFRM (`net/`)

## `skb->cb` Scratch Lifetime vs `destructor_arg`

- `skb->cb` (`char cb[48]` in `include/linux/skbuff.h`) is a layer-local scratch buffer overwritten as an SKB moves across IP, netfilter, qdisc, and device driver layers.
- Never store data in `skb->cb` during packet construction and read it later from an SKB destructor (`skb->destructor`) or asynchronous completion callback. Use `skb_shinfo(skb)->destructor_arg` (and helpers such as `skb_uarg()`) for metadata that must survive until SKB destruction.

## Packet Type Constants (`skb->pkt_type`, `include/uapi/linux/if_packet.h`)

| Constant | Value | Meaning |
| :--- | :--- | :--- |
| `PACKET_HOST` | `0` | Destined for this host |
| `PACKET_BROADCAST` | `1` | Link-layer broadcast |
| `PACKET_MULTICAST` | `2` | Link-layer multicast |
| `PACKET_OTHERHOST` | `3` | Destined for another host (promiscuous mode) |
| `PACKET_OUTGOING` | `4` | Outgoing packet of any type |
| `PACKET_LOOPBACK` | `5` | Multicast/broadcast frame looped back |

## UAPI Embedded Header Alignment Inheritance

When a UAPI header struct embeds an existing UAPI struct (for example, `virtio_net_hdr_v1` -> `virtio_net_hdr_v1_hash` -> `virtio_net_hdr_v1_hash_tunnel` in `include/uapi/linux/virtio_net.h`), the outer struct inherits the alignment of its members:
- `virtio_net_hdr_v1` is 12 bytes with 2-byte alignment (`__u8` and `__virtio16` fields). Placing a 4-byte aligned `__le32` / `__u32` field after the 12-byte header would either sit at a 2-byte aligned offset or introduce compiler padding holes.
- When extending embedded UAPI network headers, ensure new fields do not require wider alignment than the embedded struct provides (or split wider values into matching fields such as two `__le16` members) and guard casts with `BUILD_BUG_ON(__alignof__(outer) != __alignof__(inner))`.

## XFRM / IPsec Packet Family Determination (`include/net/xfrm.h`)

`struct xfrm_state` carries multiple address-family fields (`x->props.family` for outer tunnel family, `x->inner_mode.family` for primary inner family, `x->inner_mode_iaf.family` for dual-stack alternative inner family, and `x->outer_mode.family`). In cross-family tunnels (e.g., IPv6-over-IPv4) and dual-stack SA states, `x->props.family` or `x->inner_mode.family` may not match the actual packet being processed:
- Determine the actual packet family from `skb_dst(skb)->ops->family` (or `skb->protocol`) before invoking family-specific header accessors such as `ip_hdr(skb)` or `ipv6_hdr(skb)`.
