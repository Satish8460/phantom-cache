# Multi-host validation: runbook

**Status:** Draft for owner review. **Audience:** operator. **Last reviewed:** 2026-10-30 18:18 (+05:30, machine clock).

**Nothing in this file has been run.** Commands are derived from the real configuration keys (`src/config.rs`), the packaging scripts, the cluster test harness (`tests/support/cluster_harness.rs`) and the user documentation. Each command carries a tag: **[FACT]** = the key, flag or format was read in the cited file; **[UNVERIFIED]** = the command line or the combination was not run or could not be fully confirmed from source. Treat every [UNVERIFIED] line as a hypothesis until the loopback rehearsal (section S) passes.

Placeholders: `<N1_IP>` etc. are the static IPs from `01_topologies.md`; `<CID>` is the cluster id (a label of `[a-z0-9-]`, 1 to 32 characters, FACT `config.rs:139`); `$CP` cluster port; `$HP` ingress HTTP port; `$OP` origin port.

**Missing tools.** The runbook calls three tools that do not exist yet: `mh-origin` (standalone versioned origin), `mh-probe` (propagation prober and load generator), `mh-token` (JWT minting) and `mh-pki` is optional (openssl commands are given). Their contracts are in `04`, section 6. Until they exist the multi-host steps cannot be executed; the loopback rehearsal can use the existing cargo tests (section S.1).

---

## S. Smoke-test sequence (rehearse on ONE machine, loopback, first)

Tier label: `LOOPBACK`. Purpose: validate scripts, certificates, config and the evidence pipeline, and calibrate thresholds. **Not evidence for multi-host claims.**

### S.1 Use what already exists (needs owner OK to run cargo; I ran nothing)

```sh
git rev-parse HEAD ; git branch --show-current ; git status --short | wc -l     # record before
cargo build --release --locked -j 1 --bin phantom-cache-demo                     # [FACT] packaging/build-release.sh
cargo test --test cluster_propagation_latency_tests -j 1 -- --nocapture          # [FACT] file header; debug profile unless --release is added
cargo test --test cluster_harness_controls_tests -j 1                            # [FACT] file exists; contents not read by me
```

On Windows with the repository convention (FACT docs/demo/scenario_b_runbook.md): `set TEMP=D:\Temp & set TMP=D:\Temp & set CARGO_TARGET_DIR=D:\Personal\_m7_target`. Always `-j 1` (CLAUDE.md section 3).

The tests start 3 gateway processes, mTLS, fault proxies on every link, a mock origin (`VersionedMockOrigin`, `cache-control: max-age=60`, FACT harness line 135) and print timings. Record the output as the **loopback baseline** for E1.

### S.2 Hand-rolled 3-process loopback cluster (same shape as the physical runs)

1. Create a work dir `W` outside the repository: `W=$HOME/mh_smoke_$(date +%Y%m%dT%H%M%S)`; `mkdir -p $W/{pki,logs,out}`.
2. Generate the PKI and `members.json` (section 3), with all addresses `127.0.0.1` and ports: ingress 18081 to 18083, cluster 19081 to 19083.
3. Start an origin on 127.0.0.1:19000 (the `mh-origin` contract, or, until it exists, skip S.2 and stay with S.1).
4. Start 3 nodes (section 4) with `<Nk_IP>=127.0.0.1`.
5. Verify (section 5). Run one probe trial (section 6.1). Inject one fault with the **Rust fault proxy** (loopback only) or an OS tool (`02`), verify, remove.
6. Stop and clean (section 9). Compare the results with S.1.
7. Pass criteria for the rehearsal (PROPOSAL): all nodes ready within 30 s; every node reports the other two as Active; one DELETE issued on N1 makes N2 and N3 miss within 1 s; the fault is visible in node states and heals; the evidence bundle is complete and its hashes verify.

---

## 1. Pre-flight (every session)

On the controller:

```sh
date '+%Y-%m-%d %H:%M:%S %z'                     # record in the run log
git rev-parse HEAD ; git branch --show-current ; git status --short   # CLAUDE.md section 1; record, report unexpected differences
```

For every host: OS version, CPU model, core count, RAM, NIC and link speed, MTU, IP, clock offset (chrony `chronyc tracking`; Windows `w32tm /stripchart /computer:<ref> /samples:5 /dataonly`), free disk, and confirmation that the host is idle (no browsers, no builds). Isolation check: `ping -c 2 8.8.8.8` must fail (Linux; Windows `ping -n 2 8.8.8.8`).

## 2. Build the release binary (once, on the build host)

Reported numbers require a RELEASE gateway (FACT docs/demo/scenario_b_runbook.md). A debug binary makes the run EXPLORATORY.

```sh
sh packaging/build-release.sh                    # [FACT] file header: produces dist/phantom-cache-<version>-<target>/ and SHA256SUMS; stamps commit; "-dirty" suffix if the tree is dirty
# Windows: powershell -File packaging\build-release.ps1      [FACT] file exists; its flags were not read
```

The working tree is dirty by design (CLAUDE.md), so the stamped commit will end in `-dirty`; record `git status --short` output and the sha256 of the binary. Do not `git add`/`commit`/`stash` anything.

```sh
BIN=dist/phantom-cache-<version>-<target>/phantom-cache-demo        # name from packaging/build-release.sh  [FACT]
sha256sum $BIN                                                       # Windows: Get-FileHash -Algorithm SHA256
$BIN --version                                                       # [FACT] config.rs:257, smoke-test.sh
```

Copy the **same** binary to every node host (`scp`), and check the hash there. Cross-OS builds: a Windows binary does not run on Linux; build once per OS and record both hashes (this breaks "same binary"; see `05`, D2). The M3 build (if included) is built the same way from the sandbox patch set under a different CARGO_TARGET_DIR, with its own hash and a recorded patch hash; **never** replace the G binary with it.

## 3. Certificates and the membership file

The cluster uses mutual TLS 1.3 with a private CA. A node's identity is the DNS SAN `node-<id>.<CID>.phantom.internal` and the SHA-256 fingerprint of its leaf certificate DER listed in `members.json` (FACT `src/security/cluster.rs:70` to `:115`, `:287`, `:352` to `:398`; harness `generate_cluster_pki`, `cluster_harness.rs:198` to `:266`). The test PKI is generated in Rust with `rcgen` and is not available as a command; the following uses `openssl`.

```sh
CID=mh-lab ; NODES="1 2 3" ; P=$W/pki
# CA (ECDSA P-256; the harness uses rcgen's default key type, which is ECDSA P-256 — UNVERIFIED but compatible with rustls)
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out $P/cluster_ca.key
openssl req -x509 -new -key $P/cluster_ca.key -sha256 -days 30 -subj "/CN=mh-cluster-ca" \
    -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign" -out $P/cluster_ca.pem

for id in $NODES; do
  openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out $P/node_$id.key            # PKCS#8 PEM (rustls_pemfile::private_key accepts it, FACT security/cluster.rs:619)
  openssl req -new -key $P/node_$id.key -subj "/CN=node-$id" -out $P/node_$id.csr
  printf "subjectAltName=DNS:node-$id.$CID.phantom.internal\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth,clientAuth\n" > $P/node_$id.ext
  openssl x509 -req -in $P/node_$id.csr -CA $P/cluster_ca.pem -CAkey $P/cluster_ca.key -CAcreateserial \
      -days 30 -sha256 -extfile $P/node_$id.ext -out $P/node_$id.crt
  # fingerprint = SHA-256 of the DER, lower-case hex, 64 chars (FACT security/cluster.rs:287, :453)
  openssl x509 -in $P/node_$id.crt -outform DER | sha256sum | cut -d' ' -f1 > $P/node_$id.fp
done
```

All the openssl lines above are **[UNVERIFIED]**: the SAN/EKU/key-usage extension set is chosen to satisfy rustls's webpki client verifier (FACT `cluster.rs:624`: `WebPkiClientVerifier` with the CA as root) but whether the gateway accepts these certificates is exactly what the loopback rehearsal (S.2) must show; if it fails, inspect the node log tail for the `ClusterSecurityError` text (variants at `cluster.rs:30` to `:50`). Do not change the gateway to accommodate openssl.

`members.json` (FACT format: `ClusterMembersConfigFile`, `cluster.rs:445` to `:451`; `MemberNode`, `:302` to `:306`; the harness writes exactly these fields). Node ids are numbers; address map keys are node ids as JSON strings:

```json
{
  "cluster_id": "mh-lab",
  "version": 1,
  "members": [
    { "node_id": 1, "fingerprints": ["<64 hex of node 1>"], "active": true },
    { "node_id": 2, "fingerprints": ["<64 hex of node 2>"], "active": true },
    { "node_id": 3, "fingerprints": ["<64 hex of node 3>"], "active": true }
  ],
  "addresses": { "1": "<N1_IP>:<CP>", "2": "<N2_IP>:<CP>", "3": "<N3_IP>:<CP>" }
}
```

Every member must have an address from either this map or `PHANTOM_CLUSTER_PEER_ADDRS`, else startup aborts (FACT `src/main.rs`, error text around line 349). Validation rules (strict 64-char lowercase hex, non-empty lists) are at `cluster.rs:453` and `:478` to `:540`.

Distribution: copy `cluster_ca.pem`, `members.json`, and each node's own `node_<id>.crt` and `node_<id>.key` to its host (key readable only by the service user). The CA **key** stays on the controller (or is deleted after issuing; certificate rotation tests need it, E10).

Alternatives, in order of preference: (1) a small Rust or script tool that mirrors `generate_cluster_pki` (listed in `04`); (2) the commands above.

## 4. Start N nodes

Environment per node (all [FACT] keys from `src/config.rs`; the harness sets the same set, `cluster_harness.rs:495` to `:506`). Example for node 1 on a Linux host in development mode on the isolated segment:

```sh
export PHANTOM_NODE_ID=1
export PHANTOM_CLUSTER_ID=mh-lab
export PHANTOM_CLUSTER_LISTEN_ADDR=<N1_IP>:$CP
export PHANTOM_CLUSTER_PEER_ADDRS="2=<N2_IP>:$CP,3=<N3_IP>:$CP"      # id=ip:port,... (FACT config.rs:153)
export PHANTOM_CLUSTER_MEMBERS_FILE=/opt/mh/pki/members.json
export PHANTOM_CLUSTER_CA=/opt/mh/pki/cluster_ca.pem
export PHANTOM_NODE_CERT=/opt/mh/pki/node_1.crt
export PHANTOM_NODE_KEY=/opt/mh/pki/node_1.key
export PHANTOM_STRICT_CLUSTER=true                                    # require full mTLS material (FACT config.rs:157)

export ORIGIN_URL=http://<O_IP>:$OP                                   # FACT main.rs:253
export PHANTOM_ORIGIN_ALLOWLIST=http://<O_IP>:$OP                     # FACT config.rs:127; comma-separated URLs
export PHANTOM_ALLOW_INSECURE_UPSTREAM=true                           # plaintext http origin (FACT config.rs:131; harness sets it)
export PHANTOM_ALLOW_LAN_ORIGIN=true                                  # private-range origin (FACT config.rs:133)

export PHANTOM_BIND_ADDR=<N1_IP>:$HP                                  # network bind => JWT required (FACT config.rs:99, config.rs:739 to :760)
export PHANTOM_REQUIRE_AUTH=true
export PHANTOM_JWT_SECRET="$(openssl rand -base64 48)"                # >= 32 bytes; identical on all nodes and the token minter (FACT config.rs:108, :741 to :755)
export PHANTOM_ALLOW_INSECURE_HTTP=true                               # plaintext ingress, isolated lab segment only (FACT config.rs:104; warning at :926)
export PHANTOM_MAX_CACHE_BYTES=67108864                               # 64 MB as in the harness (FACT harness :506)
export PHANTOM_CLUSTER_TIMEOUT_MS=150                                 # default; state it (FACT config.rs:159)

nohup $BIN > /opt/mh/logs/node1.log 2>&1 &
echo $! > /opt/mh/node1.pid
```

Notes.
- Whether the combination of `PHANTOM_ALLOW_LAN_ORIGIN`, the allowlist and `PHANTOM_ALLOW_INSECURE_UPSTREAM` is the minimum set for a private-range http origin is **[UNVERIFIED]**; first run `$BIN --check-config` (FACT `config.rs:255`, smoke-test.sh) and read its warnings and errors before starting.
- `PHANTOM_MODE=production` forbids insecure HTTP and insecure cluster (FACT `config.rs:942` to `:951`); the lab uses the development profile. If the owner requires production mode, ingress TLS (`PHANTOM_TLS_CERT`/`PHANTOM_TLS_KEY`) replaces insecure HTTP (`scripts/gen-demo-certs.sh` produces an ingress certificate with an IP SAN, FACT file) and `PHANTOM_CORS_ORIGINS` must be set. That mode adds TLS cost to latency; decide in `05`, D5.
- Alternative to avoid JWT in the study: bind ingress to `127.0.0.1:$HP` on each node (no JWT needed on loopback, FACT config.rs:739) and run a local prober on each node host. This removes the single-clock method (P1) and the client-to-node network; it is the fallback if token minting is not available.
- Windows (PowerShell) equivalent: `$env:PHANTOM_NODE_ID='1'` ... then `Start-Process -FilePath $Bin -RedirectStandardOutput node1.log -RedirectStandardError node1.err -PassThru`; record the pid.
- Start the nodes within a few seconds of each other; the order does not matter for correctness but is recorded. Start the origin first.
- Intelligence: leave `PHANTOM_INTELLIGENCE` at its default `off` (FACT config.rs:161) except in E11's capped arm.
- M3 build only: additionally set `PHANTOM_INVALIDATION_LOG_PATH=/opt/mh/log/inv.log` on the nodes under test and note `PHANTOM_INVALIDATION_LOG_FSYNC` (default `always`) and `PHANTOM_SHUTDOWN_DRAIN_MS` (FACT m3_durability/design.md lines 12 and 156; the other M3 keys were not re-read). Use a dedicated directory on a local disk, not a network share.

## 5. Verify health and readiness

```sh
curl -fsS http://<N1_IP>:$HP/health                  # plain OK (FACT server.rs:199, :330)
curl -fsS http://<N1_IP>:$HP/health/live             # FACT health.rs:7
curl -fsS -o /dev/null -w '%{http_code}\n' http://<N1_IP>:$HP/health/ready   # 200 only when ready (FACT health.rs:8, :64)
grep -n "pool pre-warm complete" /opt/mh/logs/node1.log     # FACT marker "[cluster] pool pre-warm complete" (harness :312)
```

Cluster view from each node (needs a token with Observer role when JWT is on (FACT `server.rs:210`, the metrics route; the Observer check in `server.rs` around lines 789 and 2140 was read for other handlers, so the exact role on `/api/v1/metrics` is UNVERIFIED); token from `mh-token`):

```sh
curl -fsS -H "Authorization: Bearer $TOK_OBS" http://<N1_IP>:$HP/api/v1/metrics | jq '.nodes[] | {node_id,state}'
# [FACT] fields node_id and state: harness node_states(), cluster_harness.rs:705 to :727. Expect every peer "Active" (exact spelling of the state strings: UNVERIFIED, read from the harness' want values in the tests)
```

Session is ready only when: every node is ready, pre-warmed, sees all peers Active, and a warm-up DELETE per node (the harness `warm_invalidation_paths`, `:750`, issues one throwaway DELETE per source with a unique key) has returned 204. The warm-up exists because the first invalidation to a destination costs a connection and a handshake (35 to 60 ms on loopback, FACT ledger).

## 6. Run the workload and the experiments

Tools are the `04` section 6 contracts. Token roles: `cache:read`, `cache:invalidate` for probes; Observer (`telemetry:read`) for metrics (FACT docs/user/security.md; JWT claims `sub, iss, aud, exp, tenant_id, roles`, FACT `src/security/jwt.rs:31` to `:42`; issuer default `phantom-cache-auth`, audience `phantom-cache-api`, FACT config.rs:110 to :113). GET/DELETE require a non-empty `tenant_id` (FACT security.md line 39). All probes use one tenant.

### 6.1 E1 to E3: propagation probe (one trial)

1. Choose probe key `k = /mh/<seed>/<trial>`; `ISS` = issuer node, `PEERS` = the other nodes.
2. `GET /api/v1/cache/<k>` on every node (expect `X-Cache-Status: MISS` then cached; response body carries the version). Confirm all return version v.
3. Mutate the origin to version v+1 (`mh-origin` control endpoint). Record t0 on the observer clock.
4. `DELETE /api/v1/cache/<k>` on `ISS`; expect HTTP 204 (FACT: harness `http_delete`, `:821`). Record t_204.
5. Poll every peer at 1 ms intervals (open-loop schedule; the harness polls with `sleep(1 ms)` between polls, `wait_until_invalidated`, `:664`): the first response that is not a HIT of version v (a miss that fetches v+1, or a hit of v+1) is `t_conv[peer]`. Non-200 responses are NOT convergence evidence (FACT, the harness continues polling on non-200).
6. Continue polling for `T_hold` and flag any response with version v after `t_conv` as an E2 violation.
7. Write one CSV row per (trial, peer): `trial, issuer, peer, t204, t_conv, stale_polls, version_after, status_after, violation`.

Call the observer-clock method **P1**. Method **P2** (local pollers on 127.0.0.1 of each node, clocks synchronised) is run as a cross-check in a separate set of trials, never mixed.

### 6.2 Background workload (E4 to E8, E11)

`mh-probe load --rate 300 --keys 5000 --alpha 0.9 --seed <S0+r> --duration 31s --targets N1,N2,N3` (the flag names are a proposal; the parameters are reused from Scenario B, FACT docs/demo/scenario_b_runbook.md). Requests are scheduled open-loop and are distributed across live nodes by a fixed rule recorded in the manifest.

### 6.3 Experiments

| Exp | Procedure (summary; fault commands in `02`) |
|---|---|
| E4 | Start background load. At t=10 s `kill -9`/`taskkill /F` the chosen non-issuer node. Run 20 probe trials on the live nodes. Record node states every 100 ms. At t=25 s restart the node with the same identity (same ports, certificate, node id). Run probe trials on all three nodes until all converge. Measure `T_detect`, `T_heal`, and any key still stale on the returned node. |
| E5 | Partition {N1} from {N2,N3} (both directions, both hosts). DELETE on each side, 20 trials each. Heal. Poll until all converge; record `T_heal`. Also record every non-204 status and its latency. |
| E6 | Drop N1-to-N2 only (then N2-to-N1 only). Same procedure. |
| E7 | For each setting (delay 5/20/100/500 ms; loss 1/5/10/20 %), apply to all inter-node egress, run the E1 series (>= 200 trials per pair per setting), remove, run the no-fault control. |
| E8 | Blackhole N1-to-N2; DELETE via N1 for a key whose canonical primary is N3 (the primary is chosen by rendezvous hashing, FACT `src/dist/router.rs`; the sandbox found a suitable key by scanning with the production router, FACT reproduction_report.md section 2; for the physical study the key choice must be made by a tool that links the gateway's router code, or by searching for keys until the 204 occurs while N2 stays stale and N3 is fresh — a procedure rather than a prediction). Kill N1 within 1 s of the 204; heal; restart N1; poll N2 for 10 s. Repeat >= 20 times. |
| E8m | As E8 on the M3 build with the log path set. Add: kill during replay, torn-tail, deadline-expired graceful stop, per the crash-point matrix (`m3_durability/remediation/crash_point_test_matrix.md`; points K1 to K13 need the `m3-failpoints` feature and cannot be driven externally; only the external-kill subset applies on a release build). Power-cut subset if a PDU exists. |
| E9 | On a sacrificial node, stop time sync, step the clock by +5 min, then -5 min, then +1 h, run the E1/E2 series and a freshness check (an entry with `max-age=3` served beyond expiry on that node's clock), restore, run the control. Check cluster handshake logs for certificate-validity errors. |
| E10 | See 6.4. |
| E11 | Arms BASE (`PHANTOM_INTELLIGENCE=off`, no cap) and STATIC-CAP (`PHANTOM_INTELLIGENCE=canary`, `PHANTOM_ORIGIN_CAP_MIN=PHANTOM_ORIGIN_CAP_MAX=24`, FACT scenario_b.rs:155 to :170 for the arm definitions and config.rs:163 to :170 for the keys) with the Scenario B phases and origin degradation (service time x6 in the degraded phase). Verify `origin_cap.enforcing` per node via `/api/v1/metrics` before measuring (FACT the Scenario B runbook performs this check). Origin host records concurrency at each arrival. |

### 6.4 E10: identity cases

For each case, write the changed material, restart only the named node(s), and record whether peers admit it (log text and `nodes[].state`):
1. Wrong cluster id in the node's `PHANTOM_CLUSTER_ID` and certificate SAN.
2. Node id not in `members.json` (unknown node).
3. A certificate whose fingerprint is not in the member list (re-issue the leaf, do not update the file).
4. Expired certificate (issue with `-days 0`/a past `notAfter`; UNVERIFIED openssl flags) and a not-yet-valid one.
5. Malformed SAN (underscore, missing `node-` prefix; FACT parse rules `cluster.rs:70` to `:115`).
6. Rotation: add the new fingerprint to the member's list (`version` incremented), restart peers, then restart the node with the new certificate; remove the old fingerprint in a later `version`.
7. Revocation: set `active:false` and a higher `version`; restart the others; the node must be refused (FACT `RevokedNode` error, `cluster.rs:41`, `:352`). Then case 7b: revoke **without** restarting peers and observe (expected no effect; there is no config reload, FACT limitations.md; the runtime `update` method has no caller I could find).

## 7. Inject faults

Use `02`. Sequence per trial: start watchdog, apply, verify, run the trial, remove, verify removal, record. Never apply to the management interface.

## 8. Collect logs and metrics

Per node, at the end of every run (and on any abnormal exit): the stdout/stderr log; `GET /api/v1/metrics` snapshot every 250 ms to a JSONL file (FACT: the Scenario B driver polls `/api/v1/metrics` into `gateway_snapshots.jsonl`); `GET /api/v1/diagnostics/summary` and `/diagnostics/events` if useful (FACT routes, `server.rs:211` to `:212`; contents not read); the binary hash; the effective configuration printed at startup (FACT `config.rs` `report()`; secrets are redacted: confirm before archiving); OS network counters (`ip -s link`, `netstat -s`, or `Get-NetAdapterStatistics`) before and after; fault-tool state; clock offset samples; origin request log. The controller writes the manifest. Layout in `04`.

```sh
for id in 1 2 3; do scp node$id:/opt/mh/logs/node$id.log $W/out/run_<id>/; done
( cd $W/out/run_<id> && sha256sum * > SHA256SUMS )
```

## 9. Stop and clean

1. Stop probes. Stop each node: `kill -TERM $(cat node1.pid)` for a graceful stop (build G does not drain, FACT limitations.md) and note that a crash-class stop (`kill -9`) is used in E4/E8 only on purpose.
2. Remove all fault rules and verify: `tc qdisc show`, `iptables -L -n`, `Get-NetFirewallRule -DisplayName 'MH-*'`, clumsy closed; confirm the watchdogs are gone.
3. Restore clocks (`chronyc makestep`, `w32tm /resync /force`) and record the offset.
4. Stop the origin. Delete private keys and the CA key from the hosts (`shred -u` or secure delete); keep certificates' fingerprints in the evidence.
5. Copy evidence to the controller; compute hashes; set the manifest status COMPLETE.
6. Run `git rev-parse HEAD`, `git branch --show-current`, `git status --short` again and report any difference from step 1 (CLAUDE.md section 1).
7. Record in the run log that cleanup was verified and by whom.

## 10. Abort rules

Abort and mark the run CANCELLED/FAILED if: a node cannot be reached out-of-band after a fault; the isolation check fails; a host shows sustained CPU above the pre-registered bound; any process crashes outside the plan; the operator is unsure whether a rule is active. Cleanup (section 9) is mandatory after an abort.
