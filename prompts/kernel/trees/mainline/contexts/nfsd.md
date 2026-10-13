# NFS Server (NFSD) Subsystem Internals (`fs/nfsd/`, `fs/lockd/`)

## Trust Boundaries and XDR Codec (`nfs4xdr.c`, `nfs3xdr.c`)

`XDR decode (untrusted) -> fh_verify() -> nfs4_preprocess_stateid_op() -> VFS (trusted)`
- **Decode validation:** Always check the return value of `xdr_stream_decode_*()` before reading the decoded variable. Bounds-check client-controlled counts and lengths before allocation or array indexing (use `check_mul_overflow()` for `count * sizeof(...)`).
- **Encode and replay safety:**
  - Always NULL-check `xdr_reserve_space()` and check `xdr_stream_encode_*()` return values.
  - Perform irreversible state mutations (close, revoke, rename) and populate `so_replay` / `rp_buf` only *after* encoding succeeds so retries do not observe corrupted state or cached garbage.
  - Copy stateids to the response before calling `nfs4_put_stid()` (which may free the stateid).

## Reference Counting and Stateowner Lifecycle (`nfs4state.c`, `filecache.c`)

| Counter | Prevents | Scope |
| :--- | :--- | :--- |
| `sc_count` | Freeing `nfs4_stid` | Stateid lifetime (`nfs4_get_stid` / `nfs4_put_stid`) |
| `cl_nfsdfs.cl_ref` | Freeing `nfs4_client` | nfsdfs client object memory lifetime |
| `cl_rpc_users` | Client unhashing | Active incoming RPC compounds and async copy workers (`put_client_renew`) |
| `cl_cb_inflight` | Client destruction | Outgoing callbacks (`nfsd4_run_cb` increments; `nfsd4_shutdown_callback` drains) |

- **Do not confuse `cl_rpc_users` with `cl_cb_inflight`:** `nfsd4_run_cb()` increments `cl_cb_inflight` internally; callers dispatching callbacks do NOT need to increment `cl_rpc_users` (though delegation callbacks must increment `dp->dl_stid.sc_count` before `nfsd4_run_cb()`).
- **`so_count` and `release_openowner()`:** Hash/list membership on `cl_ownerstr_hashtbl` and `cl_openowners` is not a counted reference. `release_openowner()` consumes the caller's single reference (unhashes, drains stateids, and performs the final `nfs4_put_stateowner()`); never call an extra `nfs4_put_stateowner()` after `release_openowner()`.
- **Validation before publication:** Assign resources to struct fields only after validation succeeds (e.g., `nfsd_set_fh_dentry()`).

## File Handle and NFSv4 Stateid Invariants (`nfsfh.c`, `nfs4state.c`, `nfs4layouts.c`)

- **File handle verification (`fh_verify()`):** `fh_dentry` and `fh_export` are `NULL` until `fh_verify()` succeeds. Pass `NFSD_MAY_*` flags matching the operation (`NFSD_MAY_READ`, `NFSD_MAY_WRITE`, `NFSD_MAY_EXEC`, `NFSD_MAY_SATTR`) and pass `S_IFREG` / `S_IFDIR` when a specific file type is required. Cross-export operations (`RENAME`, `LINK`) must verify both source and target handles.
- **Stateid locking and `SC_STATUS_CLOSED`:**
  - Open/lock stateids: `cl_lock` (plus `st_mutex` for open/lock stateid serialization). After dropping `cl_lock` to acquire `mutex_lock(&stp->st_mutex)`, re-verify with `nfsd4_verify_open_stid()` to detect concurrent unhashing.
  - Delegations: `deleg_lock`.
  - Layouts: `ls_lock` (spinlock) for non-sleeping state checks; `ls_mutex` for sleeping operations or allocations.
  - Check and modify `sc_status` (`SC_STATUS_CLOSED`) atomically under the stateid's lock, and bump stateid generation via `nfs4_inc_and_copy_stateid()`.
- **Stateid-to-file matching:** Verify stateids against the current filehandle (`fh_match()` against `sc_file->fi_fhandle` for `CLAIM_DELEG_CUR`; lock stateids against their open stateid's file).
- **Lease and delegation specifics:**
  - Check `fl->fl_lmops == &nfsd_lease_mng_ops` before casting `fl->fl_owner` to `struct nfs4_delegation *`.
  - `FMODE_NOCMTIME` and `dl_atime` / `dl_mtime` are only valid on `OPEN_DELEGATE_WRITE_ATTRS_DELEG` delegations.
  - Same-client delegation short-circuits must still break conflicting delegations held by other clients.
  - Pass `ATTR_DELEG` in `ia.ia_valid` when calling `notify_change()` during `nfs4_unlock_deleg_lease()` so the delegation being released is not re-broken.
  - In VFS lease-break callbacks (which hold `flc_lock`), use `refcount_dec(&stid->sc_count)` when the caller guarantees a remaining reference so `nfs4_put_stid()` does not acquire `cl_lock` via `refcount_dec_and_lock()`.

## Locking Hierarchy, Client State Machine, and Grace Period

- **Lock hierarchy (outer to inner):**
  `nn->client_lock -> nn->deleg_lock -> nn->s2s_cp_lock -> fp->fi_lock -> clp->cl_lock -> stp->st_mutex`
  (All except `st_mutex` are spinlocks; `nn->nfsd_ssc_lock` is mutually exclusive with `s2s_cp_lock`.)
- **Client state transitions (`cl_state`):** `NFSD4_ACTIVE -> NFSD4_COURTESY -> NFSD4_EXPIRABLE -> destroyed`. Only `NFSD4_COURTESY` can transition back to `NFSD4_ACTIVE` on reconnect; `NFSD4_EXPIRABLE` is terminal. Parent openowner destruction must free copynotify stateids via `nfs4_free_cpntf_statelist()`.
- **Admin interfaces:** Procfs/sysfs/netlink handlers inspecting `nn->nfsd_serv` or server running state must hold `nfsd_mutex` across the check and operation.
- **Grace period and lease clock:** Non-reclaim state-creating ops during grace must return `nfserr_grace`. Use `nn->nfsd4_lease` / `nn->nfsd4_grace` and `ktime_get_boottime_seconds()` (not `ktime_get()`) for `cl_time` so leases survive suspend/resume.

## Error Mapping, User Namespaces, Sessions, Pages, Copy Offload, and Re-Export

- **Error mapping:** NFSv3 procedures must return `nfsd3_map_status(resp->status)`; internal negative errnos must be mapped once via `nfserrno()` (never double-mapped on an existing `__be32` status); NFSv4-only errors (`nfserr_delay`) must not be returned to v2/v3 callers; do not map `EOPENSTALE` directly to `nfserr_stale`.
- **User and net namespaces:** Use `nfsd_user_namespace(rqstp)` (not `init_user_ns` or `current_user_ns()`) with `from_kuid()` / `from_kgid()` / `from_kuid_munged()` in request paths, validate `make_kuid()` / `make_kgid()` with `uid_valid()` / `gid_valid()`, and use `nn->net` or `genl_info_net(info)` with `ns_capable()`.
- **Callbacks and session slots:** Check `cl_cb_state == NFSD4_CB_UP` under `cl_lock`, guard `cl_cb_session` with `cl_minorversion > 0`, bounds-check `slotid < se_fchannel.maxreqs` before `xa_load(&session->se_slots, slotid)`, check `same_creds()` on slot replay, and clear `NFSD4_SLOT_INUSE` on all exit paths.
- **Page array management (`rq_pages`):** Save `resp->pages = rqstp->rq_next_page` *before* read operations advance `rq_next_page`; guard `rq_next_page < rq_page_end` and `rq_bvec` against `rq_maxpages`; let `nfsd4_encode_operation()` synchronize `page_ptr` / `rq_next_page` rather than doing it manually per op; in `nfsd_splice_actor()`, do not re-append `page` when `page == *(rq_next_page - 1)` at a non-page-aligned offset, and check `svc_rqst_replace_page()`.
- **Copy offload:** Remove async copy entries from the IDR under `s2s_cp_lock` before the final put; make `OFFLOAD_CANCEL` state transitions atomic under lock; populate `wr_bytes_written` / `wr_stable_how` before calling `nfsd4_run_cb()`; validate `cnr_stateid` ownership and expiry in `nfsd4_setup_inter_ssc()`.
- **NFS re-export (`s_magic == NFS_SUPER_MAGIC`):** Embed upstream `NFS_FH()` rather than unstable `i_ino`, do not retry `-ESTALE`, acquire upstream `vfs_lock_file()` before committing local NFSD lock state, and require explicit `ex_fsid` / `ex_uuid`.
- **Resource limits:** Check per-client and compound `args->opcnt` limits *before* allocating (`nfserr_resource`), and balance `atomic_inc()` on resource counters (`num_delegations`, etc.) with `atomic_dec()` on all error paths.
