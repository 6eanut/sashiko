# SunRPC Transport & GSS Internals (`net/sunrpc/`)

## Core Client & Server Infrastructure (`svc.c`, `svc_xprt.c`, `clnt.c`, `sched.c`, `xprt.c`)

- **Transport lifetime and state flags:**
  - Acquire `svc_xprt_get()` (server) or `xprt_get()` (client) before queuing async work and release in the handler. When reassigning a client transport, release the old `xprt` reference first.
  - Check `XPT_CLOSE` / `XPT_DEAD` (server) or `XPRT_CONNECTED` / `XPRT_CLOSING` (client) before operating on a transport.
  - Every exit path from `svc_handle_xprt()` (including reservation failure) must call `svc_xprt_received()` to clear `XPT_BUSY`.
  - Use atomic `set_bit()` / `clear_bit()` (never `__set_bit()` / `__clear_bit()`) on `rqstp->rq_flags` to avoid racing with `svc_xprt_enqueue()`.
- **Server thread pool, pages, and deferral:**
  - Every server thread exit path must call `svc_exit_thread()` to clear `SP_VICTIM_REMAINS` (if `kthread_stop()` wins the race before the thread runs, the controller must clear it).
  - Guard `rqstp->rq_next_page < rqstp->rq_page_end` before calling `svc_rqst_replace_page()`.
  - When deferring a request, transfer `dr->xprt_ctxt = rqstp->rq_xprt_ctxt` and set `rqstp->rq_xprt_ctxt = NULL` (and restore on revisit) to prevent double-free.
- **Client RPC task FSM, slots, congestion, and timeouts:**
  - Every `call_*` FSM action must set `task->tk_action` for the next state and must not overwrite `tk_status` once `tk_action == NULL`.
  - Release transport slots via `xprt_release()` on all exit paths after `xprt_reserve()`.
  - Hold `xprt->transport_lock` when updating `xprt->cong`, and clamp `rq_timeout` against `to_maxval` after backoff bit-shifts.

## Socket Transport (`svcsock.c`, `xprtsock.c`)

- **Framing and socket state:** Handle short TCP reads/writes (`MSG_WAITALL` for fixed headers like the 4-byte record marker) and bounds-check incoming record lengths before allocation. Always balance `tcp_sock_set_cork(sk, false)` on error paths, and ensure `svc_write_space()` calls `svc_xprt_enqueue()` when write space becomes available.
- **Socket callback lifetime:**
  - Server child sockets inherit `sk_user_data` from the listener socket; callbacks must check `sk->sk_state == TCP_LISTEN` before dereferencing `sk_user_data` as a listener.
  - Client socket teardown must serialize under `lock_sock(sk)`: `xs_restore_old_callbacks()`, `sk->sk_user_data = NULL`, `release_sock(sk)`, then `sock_release()`.
- **Client reconnect, NOFS, and state flags:** Use `xprt_reconnect_delay()` with `queue_delayed_work()`, wrap socket operations in `memalloc_nofs_save()` / `memalloc_nofs_restore()`, place `smp_mb__after_atomic()` between ordered state-flag updates, and clear any new `XPRT_SOCK_*` flags in `xs_sock_reset_state_flags()`.

## RDMA Transport (`net/sunrpc/xprtrdma/`)

- **DMA and FRWR/MR lifecycle:** Check `ib_dma_map_sg()` return (`mr_nents == 0` on failure) and keep mappings valid until completion or synchronous wait. Do not reuse a Memory Region after `frwr_unmap_async()` until invalidation completes (`frwr_unmap_sync()` blocks). On `IB_WC_WR_FLUSH_ERR`, the RDMA device may already be gone—do not invoke `ib_dma_*` helpers in flush error paths.
- **Work completion and post-send lifetime:** Check `wc->status == IB_WC_SUCCESS` before reading `wc->byte_len` or other completion fields (only `wr_cqe` and `status` are valid on error). Because completion can fire immediately after `ib_post_send()`, copy any fields needed after `ib_post_send()` to local variables beforehand, and set `XPT_CLOSE` *before* `svc_rdma_*_ctxt_put()` on teardown.
- **CM events, reconnect, and credits:** Track whether `ESTABLISHED` (`rpcrdma_ep_get()`) actually occurred before dropping the reference on `DISCONNECTED` / `DEVICE_REMOVAL`. Remap `regbuf` DMA buffers on reconnect if teardown called `rpcrdma_regbuf_dma_unmap()`, and always call `rpcrdma_post_recvs()` *before* `rpcrdma_update_cwnd()` to avoid RNR errors.

## RPCSEC_GSS and Network Namespaces (`auth_gss/`, `svcauth_gss.c`)

- **GSS security and lifetimes:** Hold `sd_lock` across sequence-window checks and updates (handling `MAXSEQ` overflow and `sd_max - GSS_SEQ_WIN` underflow); check `gss_verify_mic()`, `gss_wrap()`, and `gss_unwrap()` against `GSS_S_COMPLETE`; pair `gss_svc_searchbyctx()` with `cache_put()`; account for RFC 4121 Kerberos v2 trailing checksum slack (`au_ralign != au_rslack`); match `uid`, `service`, and in-flight state in `__gss_find_upcall()`; and skip re-verification when `rqstp->rq_deferred` is set.
- **Network namespaces:** Always use `xprt->xpt_net` / `serv->sv_net` (server) or `xprt->xprt_net` / `clnt->cl_xprt->xprt_net` (client) with `sock_create_kern(net, ...)` and `rdma_create_id(net, ...)`—never `&init_net` or `current->nsproxy->net_ns`.
