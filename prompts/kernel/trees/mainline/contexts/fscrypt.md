# Filesystem Encryption (`fscrypt`) Context (`fs/crypto/`, `include/linux/fscrypt.h`)

## `struct fscrypt_operations` & Data Unit Constraints (`include/linux/fscrypt.h`)

- `FSCRYPT_CONTENTS_ALIGNMENT` (`16` bytes): All encrypted content lengths must be divisible by 16 bytes. Filesystems supporting variable-length encrypted blocks (e.g., compressed extents) must pad to `FSCRYPT_CONTENTS_ALIGNMENT`.
- `supports_subblock_data_units`: Allows policies with `log2_data_unit_size` smaller than the filesystem block size; incompatible with variable-length blocks and with `fscrypt_encrypt_block_inplace()` / `fscrypt_decrypt_block_inplace()`.
- `needs_bounce_pages`: Must be set when the filesystem calls `fscrypt_encrypt_pagecache_blocks()` so the bounce page pool is allocated.
- `legacy_key_prefix`: Deprecated v1 logon key prefix; filesystems newly integrating `fscrypt` must leave `legacy_key_prefix == NULL`.
- `is_block_based`: Set to 1 if the filesystem is block-based; enables `blk_crypto_key` setup and `fscrypt_set_bio_crypt_ctx()`.
- `has_32bit_inodes`: Allows `FSCRYPT_POLICY_FLAG_IV_INO_LBLK_{32,64}` for inline crypto hardware with limited data unit number widths.

## Inode Creation & Key Lifecycle (`fs/crypto/`)

- **New inode initialization:** Filesystems must call `fscrypt_prepare_new_inode(dir, inode, &encrypt)` before the new inode can become visible to other threads, and persist the inherited context via `fscrypt_set_context(inode, fs_data)` inside the inode-creation transaction before unlocking the new inode.
- **File open & DIO:** Regular file `->open()` must call `fscrypt_file_open(inode, filp)` to set up `i_crypt_info` (or reject opens without the key on encrypted files); direct I/O paths must verify `fscrypt_dio_supported(iocb, iter)`.
- **Eviction:** When an inode is evicted, `fscrypt_put_encryption_info()` must be called to release associated key references and crypto transforms.

## Synchronization & Concurrency Rules

- **Locking Invariants:** `fscrypt_inode_info` initialization is protected by the inode's `i_rwsem` (or equivalent) to prevent race conditions during concurrent `open()` calls.
- **RCU Usage:** `fscrypt_inode_info` is accessed via RCU in hot paths. Modifications to encryption policy or key state must use `rcu_assign_pointer()`; readers must use `rcu_read_lock()` / `rcu_dereference()`.
- **Atomic Contexts:** Do not perform memory allocations with `GFP_KERNEL` inside `fscrypt` callbacks invoked from atomic contexts (e.g., bio submission). Use `GFP_NOFS` or `GFP_ATOMIC`.
- **Serialization:** Filesystem-specific `fscrypt_operations` must ensure that `set_context` and `get_context` are atomic with respect to inode creation/lookup to prevent TOCTOU races.

## Error Handling & Integrity

- **Pointer Validation:** Always use `IS_ERR()` before checking for `NULL` on `fscrypt` helpers returning `ERR_PTR`.
- **Transaction Integrity:** Metadata operations (setting encryption context) must be part of the same atomic transaction as inode creation. If the transaction fails, the `fscrypt` state must be rolled back.
- **Transient Errors:** Propagate `EBUSY` to allow for retries; treat invalid policies as permanent failures.
- **Integer Overflows:** Use `check_add_overflow()` or `check_mul_overflow()` when calculating buffer sizes for `fscrypt_str` or key blobs.
- **Uninitialized Memory:** Zero-initialize `fscrypt_context_v2` structures (e.g., via `memset()`) before writing to disk to prevent information leaks.
- **Crypto API Masking:** Use `FSCRYPT_CRYPTOAPI_MASK` when allocating transforms to ensure only inline-compatible crypto drivers are used, avoiding slow/problematic non-inline crypto API drivers.

## Untrusted Input Boundaries

- **Policy Validation:** Validate that `log2_data_unit_size` is within bounds supported by the filesystem's block size.
- **Name Lengths:** Validate `disk_name.len` against the filesystem's maximum filename length before VFS lookups.
- **Key Sizes:** Use `FSCRYPT_MIN_KEY_SIZE` (16) and `FSCRYPT_MAX_RAW_KEY_SIZE` (64) for validation; never use `FSCRYPT_MAX_KEY_SIZE` (deprecated/misleading).
- **Buffer Safety:** Ensure `fscrypt_str` buffers are correctly sized and null-terminated where required by the VFS layer.

## Subsystem Contracts & Callbacks

- **Cross-Layer Contracts:** Filesystems must ensure that `fscrypt_operations` are fully populated before the filesystem is registered with the VFS.
- **Error Propagation:** When `fscrypt_file_open` returns an error, the filesystem must abort the open operation and propagate the error code to userspace.
- **Hardware Constraints:** Inline crypto engines must be queried for supported algorithms; `fscrypt` will automatically fallback to CPU-based crypto if the hardware does not support the requested policy.
- **Cleanup:** Ensure that all `fscrypt` resources are released during `evict_inode` to prevent memory leaks in long-running filesystems.
- **Context Preservation:** Ensure that `fscrypt_context_v2` structures are properly versioned and that the `nonce` field is unique per file to prevent IV reuse attacks.
- **Memory Scrubbing:** Sensitive key material in `fscrypt_master_key` structures must be zeroed using `memzero_explicit()` before the memory is freed or returned to the slab allocator.
- **Reentrancy:** Ensure that `fscrypt` callbacks do not trigger recursive filesystem operations that could lead to stack overflows or deadlocks on the same inode's `i_rwsem`.
- **Atomic Completion:** Crypto request handlers must return zero if they have already performed the finalization callback to prevent double-completion bugs.
- **Resource Reservation:** Ensure that metadata transactions reserve sufficient space for `fscrypt` context updates to avoid ENOSPC during cleanup or inode creation.
