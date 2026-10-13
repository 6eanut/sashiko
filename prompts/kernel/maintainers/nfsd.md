# NFSD Maintainer Notes (`fs/nfsd/`, `fs/lockd/`)

## Code Style & XDR Code Generation (`Documentation/filesystems/nfs/nfsd-maintainer-entry-profile.rst`)

- **Variable declaration ordering:** Order local variable declarations in reverse-christmas-tree style (longest line to shortest line).
- **Status and endianness conventions:** Use `nfs_ok` and `nfserr_*` (`__be32`) constants for NFS protocol status values and `cpu_to_be32()` / `be32_to_cpu()` for endianness conversions.
- **Generated XDR (`xdrgen`):** New NFSv4 XDR encoding and decoding code should use `xdrgen`-generated definitions (`fs/nfsd/nfs4xdr_gen.c`) rather than adding new hand-written XDR marshalling helpers.
