# M3 invalidation durability: refined design

Status: INVESTIGATION + PROPOSED DESIGN. Prototype validated only in a scratch copy (`D:\Temp\m3_scratch`). Nothing here is integrated. INTEGRATION REQUIRES OWNER APPROVAL (application-code change, CLAUDE.md section 2) AND AN INDEPENDENT REVIEW PASS (invalidation path, CLAUDE.md section 5).

Evidence tags used below: [code] = verified by reading `file:line` in the repo snapshot; [scratch-run] = reproduced in the scratch copy (see `reproduction_report.md`, `test_plan.md`, `evidence/`); [design] = proposal, not tested; [assumption].

Baseline for line numbers: repo HEAD `456bb25` plus the uncommitted working tree as of 2026-10-30.

## 1. Scope

In scope (all opt-in or bounded):
1. An env-gated durable owed-invalidation log (`PHANTOM_INVALIDATION_LOG_PATH`, unset = off = today's behaviour byte for byte).
2. A graceful-shutdown drain with a deadline (`PHANTOM_SHUTDOWN_DRAIN_MS`, default 2000 in the prototype, owner decision; 0 = today's behaviour).
3. Harness support (kill / restart / graceful stop) and the multi-process tests.

Out of scope (rejected in the investigation, section 12 of the accountability report): consensus or replicated logs, persisting the replay ring or the stream sequence counters, flush-all-peers on every boot, persisting the cache, a wire-format change (boot id), delayed-204 strict mode, a TTL ceiling.

## 2. Failure model

Covered: gateway process crash (kill -9 / TerminateProcess / panic abort / OOM kill), SIGTERM / Ctrl-C / Ctrl-Break, restart, host reboot when `fsync=always`, torn last write.
With `fsync=interval|never`: process crash only; an OS crash or power loss can lose the not-yet-synced tail (at most 100 ms of intents for `interval`).
Not covered: loss or corruption of the log medium (disk loss), an operator deleting the log AND its sentinel, a second failure of the issuer before the replayed debts are delivered AND loss of the log, peers that are permanently gone (their debt ages out, section 9), clock steps (age-out only).
The guarantee given when the log is on: *a DELETE answered 204 whose fan-out has not been acknowledged by every non-primary member is re-delivered after any restart of the issuer, until acknowledged or aged out.* It does not make the primary's synchronous leg or the local purge durable (they are not owed: they already happened).

## 3. What the code does today (all [code])

- `delete_cache_handler` purges locally (`server.rs:1952`) then awaits `invalidate_key_across_cluster` (`server.rs:1957`).
- On primary success, `invalidate_key_across_cluster` computes `fanout_targets` and `dispatch_fanout` (`router.rs` ~1822-1823 original numbering): deferred peers go straight to `RepairLedger::record`, live peers go into the bounded broadcaster channel, whose per-peer worker retries up to 4 times (25..200 ms backoff, 150 ms per attempt) and records a debt only after exhausting them. Until then the invalidation exists only in channel / task memory.
- `RepairLedger` (`router.rs:137`) is in-memory only; bounds `REPAIR_MAX_ENTRIES_PER_PEER=1024`, `REPAIR_MAX_KEY_BYTES_PER_PEER=256 KiB`, tenant collapse (<= 16 tenant flushes), then a `full_flush` marker.
- `main.rs` has no drain: on a signal it prints "cache is in-memory, nothing to flush" and returns; `rt.shutdown_timeout(5 s)` follows. `WorkerGuard::drop` moves queued jobs into the ledger, which dies with the process.

## 4. Design

### 4.1 Where durability hooks in (the 204 contract)

An accepted DELETE is "accepted" at the moment the handler decides to answer 204. The durable intent is therefore appended **inside `invalidate_key_across_cluster`, after the primary acknowledged and before `dispatch_fanout`**, and the call awaits the group-commit. The owed peer set is `live + deferred` (the primary acknowledged synchronously and the local node is purged by the caller). In the acting-primary failover path the suspect primary is added to the set.

Error paths (primary unreachable / rejected -> 5xx): debts are already recorded in the ledger (`record_owed_to_all`); the ledger mirror (4.2) makes them durable best-effort (non-awaited), consistent with the client having received an error rather than a 204.

### 4.2 Mirror of the ledger, ack watermarks

Two hook families, both no-ops when the log is off:

- **Owed events.** `RepairLedger::record` and the `mark_full_flush` transition call `DurableLog::note_owed(peer, scope, key)` (non-blocking enqueue). This makes every existing debt source durable without touching each call site: deferred peers, worker retry exhaustion, `WorkerGuard::drop`, queue overflow, `record_owed_to_all`, escalations in `repair_peer`.
- **Ack events.** A delivery attempt captures `w = durable high-water stamp` BEFORE its first send. On a success ack it appends `Ack(peer, scope, key, upto=w)`. Sites: the broadcaster worker (`delivered`), `RepairLedger::complete`, `drop_permanent`, `flush_finished` (uses `flush_w` captured in `take_full_flush`). `RepairEntry` gains `durable_w`, `PeerRepair` gains `flush_w`.

**Why a watermark and not "ack = remove"**: the ledger dedups by `(scope, key)` and an in-flight delivery can predate a newer DELETE of the same key. Each intent gets a monotone stamp assigned by the single writer thread. An ack removes only owed items with `stamp <= w`; any intent accepted after the delivery started has a larger stamp and survives. Correct because a send that *starts* after stamp `s` was assigned is processed by the peer after the origin changed (the origin change precedes the DELETE, the DELETE precedes the stamp). Tenant-scope acks clear tenant and exact-key items of that tenant with `stamp <= w`; a FullCache ack clears everything `<= w` for the peer. A lost or dropped ack is always safe: it only causes one redundant, idempotent re-delivery after a restart. An ack that races ahead of its intent in the queue is impossible by construction for awaited intents (the intent is applied by the writer before the awaiting caller dispatches the job that will later ack it) and safe for non-awaited ones (stamp larger than `w`).

### 4.3 File format (version 1)

All integers little endian. One file, append-only, path from `PHANTOM_INVALIDATION_LOG_PATH`; sentinel `<path>.sentinel`; compaction temp `<path>.tmp`; quarantined bad log `<path>.corrupt` (one generation kept).

```
header (20 B): "PCIL" | version u16 (=1) | node_id u8 | reserved u8 | created_unix_ms u64 | crc32 u32 (of the first 16 B)
record       : body_len u32 | body | crc32 u32 (of the 4 length bytes and the body)
body types   :
  1 Intent      stamp u64 | unix_ms u64 | scope u8 | key_len u16 | key | npeers u8 | peers[npeers]
  2 Ack         peer u8 | scope u8 | key_len u16 | key | upto_stamp u64
  3 FlushMarker peer u8 | stamp u64 | unix_ms u64        peer owes a FullCache flush (key empty)
  4 Checkpoint  stamp_counter u64                         first record of a compacted file
  5 Clean       unix_ms u64                               informational: orderly shutdown
```
CRC-32/IEEE (poly 0xEDB88320), implemented in-module (no dependency; self-test vector `123456789 -> 0xCBF43926`).

Bounds: `body_len` in `1..=8192` (key <= `MAX_KEY_LENGTH_BYTES` 4096 + fixed fields + 255 peers). Keys are the already-normalised wire keys (`normalize_wire_invalidation`: an exact key over 4096 B is owed as its tenant, <= 128 B), so a record can never exceed the frame cap. `scope` must be 0..=2. A record failing any of these is corruption (see 4.5).

### 4.4 In-memory image, collapse, size bound

The writer thread owns a `State` identical in bounds to the ledger: per peer <= 1024 entries and <= 256 KiB of key bytes, largest-tenant collapse (<= 16 tenant entries), then a single FullCache marker. Peers are `u8`, so worst case 255 x ~256 KiB of keys ~ 65 MB in memory, the same ceiling the ledger documents; a real cluster has a handful of peers.

Disk size: appends are unbounded between compactions, so compaction is the bound.
- Trigger: file >= max(1 MiB, 2 x size after the previous compaction), or > 64 MiB hard cap (forces it), or on orderly close with an empty image.
- Method: write `<path>.tmp` (header + Checkpoint + FlushMarkers + one single-peer Intent per live item, sorted by stamp), `fsync`, atomic `rename` over the log, fsync the directory (Unix), reopen for append. A crash at any point leaves either the old or the new complete file. A failed compaction keeps appending to the old file and counts a write error.
- Steady state: the file is bounded by roughly `2 x max(1 MiB, live image)`; live image is bounded by the ledger bounds above.
- Age-out safety net: the ledger may collapse differently from the log image (for example the log has an exact-key entry while the live ledger already escalated to a tenant flush), in which case the matching ack may never arrive and the debt would be replayed forever. Items older than `max_debt_age_ms` (default 7 days, 0 disables) are dropped at replay and compaction. This is a deliberate, documented trade-off; a follow-up (drive the image from ledger transitions instead of from events) removes the divergence.

### 4.5 Boot sequence

1. Config: if the path is unset, do nothing else (log off).
2. `DurableLog::open(path, node_id, non_local_members, cfg)`:
   - file absent and sentinel absent: first start, create (header + Checkpoint), fsync, write sentinel.
   - file absent but sentinel present: the log was lost -> **LostFallbackFlush**.
   - file present: read it whole; validate header (magic, version, header CRC, node id) then each record.
     - zero bytes or a header prefix shorter than 20 B whose bytes are a prefix of the magic: torn creation, treated as fresh.
     - a record that is incomplete, has a bad CRC but ends exactly at EOF, or a zero-filled tail: **torn tail** (a crash during append; nothing in it was acknowledged under `fsync=always`) -> truncate to the last good record, continue.
     - any other failure (bad header, bad length, CRC mismatch not at EOF, unknown type, malformed body, node id mismatch): **CorruptFallbackFlush**.
   - fallback: rename the bad file to `<path>.corrupt` (replacing the previous one), start an empty image, mark every non-local member as owing a FullCache flush, write that image as the new file (so the fallback debts are themselves durable until acknowledged).
   - else apply age-out.
3. Replay: for each debt, `RepairLedger::record(peer, None, scope, key)` (the ledger applies its own bounds; seq `None` = fresh stream sequence; the receiver applies every InvalidateRequest regardless of sequence). This happens BEFORE `attach_durable`, otherwise the replay would be appended to the log again.
4. Attach the log to the ledger, then start the anti-entropy / repair worker. Peers are `Active` at boot, so the first repair tick (250 ms) delivers.
5. A failure to open the log with the flag set is **fatal at start-up** (`EXIT_CONFIG`): an operator who opted in must not silently run without durability.

Coarse-but-safe: the fallback flushes the peers' whole caches (cluster-visible cold cache on those peers). That is intentionally the same trade-off the ledger already makes on overflow; it happens only after real corruption or a lost log.

### 4.6 Group commit, fsync policies, hot-path cost

A dedicated OS thread owns the file. Producers use a bounded `std::sync::mpsc::sync_channel(8192)` with `try_send` (never block a tokio worker). The thread drains up to 512 commands, encodes, applies to the image, does one `write_all` and (policy permitting) one `sync_data`, then releases the awaiting callers through `tokio::sync::oneshot`. Concurrent DELETEs share an fsync.
- `always` (default when enabled): callers are released after the fsync. DELETE latency += one group-commit fsync.
- `interval` (100 ms): callers released after the write reaches the OS; fsync at most every 100 ms.
- `never`: write only.
For a process-crash failure model `never` and `interval` already preserve the data (the bytes are in the OS page cache once `write_all` returns); fsync only matters for OS crash / power loss.
Acks and `note_owed` events never wait.
Disk error / full: see 4.7.

### 4.7 Disk full, I/O errors, permissions

- `write_all` or `sync_data` failure: the batch's waiters get `Err`; the log sets `degraded`, increments `write_errors` and prints one line; DELETEs proceed **fail-open** (the 204 contract degrades to today's in-memory behaviour and the degradation is visible in `stats`). Fail-closed (DELETE answers 5xx when the intent cannot be persisted) is the alternative; it needs a new `DistError` variant and a handler mapping, and is left as an owner decision (the report recommends fail-open for a first, opt-in iteration because a 5xx after the local purge and the primary ack does not undo them).
- Compaction failure: keep appending to the old file, count the error.
- Channel full (8192 queued commands): `append_intent` returns `Err` (fail-open, counted); acks are dropped and counted (safe).
- Unix: file created `0600`. Windows: inherits the directory ACL; the directory should be private to the service account (document in `operate.md`). The log contains cache keys (paths, queries and tenant ids): treat as sensitive, same class as the cache contents.

### 4.8 Interaction with stream sequences and the replay ring

Not persisted. After a restart the gateway's per-destination sequence counters restart at 1 and its ring is empty. [code] the receiver applies every `InvalidateRequest` regardless of sequence, so replayed (new-sequence) frames work; the receiver's tracker may log a "duplicate or obsolete" gap note, which is cosmetic. If a peer later pulls from the restarted issuer it gets `HistoryOverwritten` and flushes (safe, coarse): that existing behaviour is unchanged. The log carries *what is owed*, not *what was sent*.

### 4.9 Graceful shutdown drain

`RendezvousRouter::drain(deadline)`: loop every ~25 ms: finish when `pending_jobs == 0` (new counter: fan-out jobs accepted into a per-peer queue and not yet settled) and no deliverable peer (active member, not Failed/Offline) has ledger debts; otherwise run `repair_all_peers()` bounded by the remaining time. `main.rs` sequence on SIGINT / SIGTERM / Ctrl-Break: `begin_draining()` (readiness false), stop polling the server future (no new connections; in-flight handler tasks keep running), `drain(PHANTOM_SHUTDOWN_DRAIN_MS)`, `router.stop()`, `DurableLog::close(clean = drain complete)` (final fsync; compacts an empty image; writes the Clean marker). On an incomplete drain it prints which peers still owe acknowledgement and whether the debts are kept in the log or lost. Windows: `shutdown_signal` additionally listens for CTRL_BREAK (the only console event that can be aimed at one process).
The drain is useful without the log (planned restarts with a reachable peer) and complementary with it (deadline expiry keeps the debt).

## 5. Invariants that must hold

| # | Invariant | How it is enforced / tested |
|---|---|---|
| D1 | A DELETE answered 204 never loses its owed set (non-primary members) across an issuer crash/restart, barring disk loss, while the log is on and healthy | intent awaited before `dispatch_fanout`; `m3_kill9_with_log_peer_converges_after_issuer_restart` |
| D2 | Replay is idempotent | ledger dedup + receiver idempotence; `replay_is_idempotent_and_bounded` |
| D3 | Log size bounded | ledger-equal image bounds + compaction + 64 MiB cap; `file_size_stays_bounded_under_churn_via_compaction` |
| D4 | The DELETE hot path is never blocked beyond the chosen fsync policy; no blocking call on a tokio worker | `try_send` + oneshot; overhead measurement |
| D5 | Fencing semantics unchanged: no change to `finalize_fetch`, generations, epochs, lock order | diff touches only `src/dist/router.rs`, new `durable_log.rs`, `main.rs`, `config.rs`, `lib.rs`; no `core::cache` change |
| D6 | An ack never clears an intent newer than the delivery's start | stamps; `newer_intent_survives_an_ack_that_started_before_it` |
| D7 | Corruption never panics and never silently drops debts: it falls back to a full flush of every peer | `mid_file_corruption_falls_back_to_full_flush_for_every_peer`, `bad_header_wrong_node_and_lost_log_all_fall_back` |
| D8 | A torn last record is tolerated, earlier records survive, later appends are readable | `truncated_last_record_...`, `appends_after_torn_tail_recovery_are_readable` |
| D9 | Log off = no behavioural change (no allocations on the hot path beyond one `OnceLock::get`) | every hook is behind `durable.get()` |
| D10 | `std::sync::Mutex` never held across `.await`; ledger lock order unchanged (ledger mutex is a leaf; `try_send` under it is non-blocking) | code review item |

## 6. Interaction with existing invariants (CLAUDE.md section 5)

Generation fencing, epochs, eviction, byte accounting, the staleness cap and telemetry are not touched. Lock order: `RepairLedger.peers` mutex now calls `DurableLog::note_owed` / `ack` while held (non-blocking `try_send`, no further locks): a leaf. `ReplicaBroadcaster` gains one `AtomicUsize`. `RepairEntry` gains a field (struct has no construction sites outside `router.rs`).

## 7. Known limitations of the design (not hidden)

1. Fail-open on log write errors (4.7).
2. Image / ledger divergence can leave a redundant debt until age-out (4.4).
3. `fsync=interval|never` do not survive an OS crash.
4. The primary's leg and the local purge are not durable (not owed).
5. Peers that stay unreachable keep their debts (bounded); a tenant or FullCache escalation is the bounded steady state.
6. A DELETE handler future cancelled by a client disconnect between the local purge and the intent (hypothesis in the investigation, axum behaviour unverified) is NOT fixed by this design; fix = run the cluster leg detached (separate item).
7. Single-node insecure/partial cluster mode has no usable router; the log is wired only in the mTLS cluster path.
8. Documentation overclaims (ADR-003 4.1 / G2 / G3, `limitations.md:8`, `operate.md:33`) remain mandatory regardless of this patch.
9. No metrics endpoint yet exposes `LogStats` (counters exist in the struct; wiring them to `/api/v1/metrics` is a follow-up).

## 8. Open decisions for the owner

1. Fail-open (prototype) vs fail-closed on log write failure.
2. Default of `PHANTOM_SHUTDOWN_DRAIN_MS` (prototype 2000; 0 preserves today's exit timing).
3. Default `PHANTOM_INVALIDATION_LOG_FSYNC` (`always`).
4. Whether to keep the 7-day age-out default.
5. Whether `PHANTOM_INVALIDATION_LOG_PATH` should require `PHANTOM_MODE=production` hardening (private directory check).
