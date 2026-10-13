# Filesystem Encryption (`fscrypt`) Context (`fs/crypto/`, `include/linux/fscrypt.h`)

## `struct fscrypt_operations` & Data Unit Constraints (`include/linux/fscrypt.h`)

- `FSCRYPT_CONTENTS_ALIGNMENT` (`16` bytes): all encrypted content lengths must be divisible by 16 bytes. Filesystems supporting variable-length encrypted blocks (e.g., compressed extents) must pad to `FSCRYPT_CONTENTS_ALIGNMENT`.
- `supports_subblock_data_units`: allows policies with `log2_data_unit_size` smaller than the filesystem block size; incompatible with variable-length blocks and with `fscrypt_encrypt_block_inplace()` / `fscrypt_decrypt_block_inplace()`.
- `needs_bounce_pages`: must be set when the filesystem calls `fscrypt_encrypt_pagecache_blocks()` so the bounce page pool is allocated.
- `legacy_key_prefix`: deprecated v1 logon key prefix; filesystems newly integrating `fscrypt` must leave `legacy_key_prefix == NULL`.

## Inode Creation & Key Lifecycle (`fs/crypto/`)

- **New inode initialization:** Filesystems must call `fscrypt_prepare_new_inode(dir, inode, &encrypt)` before the new inode can become visible to other threads, and persist the inherited context via `fscrypt_set_context(inode, fs_data)` inside the inode-creation transaction before unlocking the new inode.
- **File open & DIO:** Regular file `->open()` must call `fscrypt_file_open(inode, filp)` to set up `i_crypt_info` (or reject opens without the key on encrypted files); direct I/O paths must verify `fscrypt_dio_supported(iocb, iter)`.
