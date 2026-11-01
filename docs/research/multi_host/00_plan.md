# Multi-host validation plan

**Status:** Draft for owner review. **Audience:** product owner (decisions), whoever runs the study, independent reviewers. **Last reviewed:** 2026-10-30 18:18 (+05:30, machine clock). **Author:** agent MH (documents only; nothing in this folder has been executed).

Companion files: `01_topologies.md`, `02_fault_injection.md`, `03_runbook.md`, `04_data_collection_and_analysis.md`, `05_open_decisions.md`.

**Tags used.** FACT (file:line) = read in the working tree on 2026-10-30. FACT (ledger) = taken from a repository document, not re-verified in code. PROPOSAL = my suggestion, not a decision. UNVERIFIED = I could not check it without running something. Nothing here is a measurement.

---

## 1. Why this study exists

Every cross-node statement PhantomCache makes today rests on one machine. The evidence is:

- **FACT (ledger):** claim C6 (reliable invalidation fan-out) is "EVALUATED (weak)", qualified to single-host TCP loopback AND to the case where the issuing gateway stays alive (`docs/research/claims_ledger.md`, C6 amendment and C6 qualifier).
- **FACT (ledger):** the only propagation measurement is loopback, debug build, Windows 11, 3 processes: warm P99 about 10 to 20 ms, first invalidation to a destination (cold connection plus mTLS handshake) 35 to 60 ms (`claims_ledger.md`, C6 amendment).
- **FACT (docs/user/limitations.md):** only Windows x86-64 has been exercised; cluster mode is experimental; no release-grade numbers exist.
- **FACT (ledger, M3 report):** an accepted DELETE whose fan-out is not yet acknowledged lives only in the issuer's memory; a kill left the peer stale in every one of 92 polls over 10 s (sandbox, debug, loopback). The durable log that fixes it is NOT integrated (DEC-027, blocked on rework and re-review).
- **FACT (code):** no automated test or process uses more than one physical host. The cluster harness binds ephemeral `127.0.0.1` ports (`tests/support/cluster_harness.rs`, `start_once`, around lines 417 to 450).

The study asks: **do the loopback observations survive contact with a real network, real clocks and real process failures, and where do they break?** It is a falsification study, not a demonstration. A result that says "the claim fails" is a successful outcome.

## 2. Scope: two builds, never mixed

The plan covers two builds. Their results are reported in separate tables and never pooled.

| Build | Definition | Status |
|---|---|---|
| **G (current gateway)** | The release binary built from the committed or working-tree source as it stands, with no M3 patch. Command, config keys and behaviour as in `docs/user/`. | Testable now once devices exist. |
| **M3 (durable-invalidation build)** | G plus the M3 durable log, detached DELETE cluster leg and shutdown drain (`docs/research/m3_durability/`). Opt-in via `PHANTOM_INVALIDATION_LOG_PATH` (FACT, `m3_durability/design.md` line 12). | **NOT integrated.** DEC-027 is "blocked on rework and re-review". Include only if the owner decides to (`05_open_decisions.md`, D6) and only from a build whose provenance (patch hash, source hash) is recorded. Tests marked "M3 only" are skipped for G. |

"Unpatched build" (the baseline in section 7) means G with no M3 content. Any M3 result must state that the build was a sandbox build, and whether the independent re-review had completed by the run date.

## 3. Claims under test and their falsifiers

Each row is a claim the project makes or implies, the experiment, and the observation that would **falsify** it. Experiments are numbered E1 to E10. Thresholds are in section 6 and are PROPOSALS until pre-registered.

| ID | Claim (source) | Experiment | Falsifier (any one is enough) |
|---|---|---|---|
| **E1** | **Cross-host propagation is fast.** After a DELETE returns 204 on node A, peers stop serving the old entry within a small bound (C6; loopback P99 about 10 to 20 ms, FACT ledger). | Warm cluster, N trials per ordered node pair: GET key on all nodes, change the origin version, DELETE via A, poll B and C until a miss or the new version. Two measurement methods (single-observer clock, local pollers) per `04`. | The upper confidence bound of the P99 propagation time exceeds the pre-registered bound `T_prop`; or any single trial exceeds `T_prop_max`; or the cold-path (first invalidation to a destination) distribution exceeds `T_cold`. |
| **E2** | **No stale read after propagation.** Once a peer has served the new version or a miss, it never again serves the old version (C6 "P(stale)=0" was NOT supported as worded, FACT ledger). | In every E1 trial keep polling for `T_hold` after first convergence. | Any response carrying the old version after that peer's first non-stale response. |
| **E3** | **Stale window after DELETE is bounded by propagation, not by TTL.** The 204 means "local purge plus primary acknowledged; others asynchronous" (FACT ledger C6 qualifier; `docs/user/limitations.md`). | Measure the stale window per peer (time from the 204 until that peer's first non-stale answer) across all peers, including the peer that is neither issuer nor primary. | The stale window of any trial exceeds `T_prop_max`, or the window is bounded only by the origin `max-age` (the entry stays until it expires). |
| **E4** | **Node failure is detected and healed.** Peers mark a dead node Suspect/Failed; DELETE continues via an acting primary; a rejoining node converges (repair worker, anti-entropy every 250 ms; FACT main.rs around lines 370 to 380; effectiveness "not established", FACT limitations.md). | Kill -9 (or power-cut a VM) of a non-issuer node during a DELETE stream; later restart it. Record states via `GET /api/v1/metrics` `nodes[].state` (as in `cluster_harness.rs` `node_states`). | A DELETE issued while the node is down is never applied on that node after it rejoins within `T_heal`; or any live node serves a stale entry for longer than `T_prop_max` while the dead node is down; or a live node never leaves the Active state for the dead node within `T_detect`. |
| **E5** | **Symmetric partition behaviour is fail-safe.** | Partition {A} from {B,C} both directions, DELETE on each side, then heal. | After healing, any node still serves a pre-DELETE entry beyond `T_heal`; or a DELETE returned 204 while the primary was unreachable (a 204 must imply the primary acknowledged, FACT ledger); or the gateway hangs instead of answering within its leg timeout (150 ms per attempt by default, FACT config.rs:159 to 160). |
| **E6** | **Asymmetric partition** (A can send to B, B cannot send to A, or one direction drops): the same properties as E5. | One-way drop on one link. | Same as E5, and additionally any node permanently marked Failed or Offline while it is in fact reachable in at least one direction and the cluster does not recover after healing. |
| **E7** | **Delayed and lost messages do not break correctness**, only latency (C6 failure model F: loss up to 20 percent). | Inject fixed delay (for example 5, 20, 100, 500 ms) and random loss (1, 5, 10, 20 percent) on inter-node links; run E1/E2/E3 under each. | Any E2 violation under loss or delay; or propagation not monotone-bounded by retry budget plus repair-worker interval; or loss of 20 percent leads to a stale entry that outlives `T_heal`. |
| **E8** | **Issuer crash, build G.** An accepted DELETE with unacknowledged fan-out is lost if the issuer dies (FACT ledger; reproduced in the sandbox). This is a known defect; the study measures it on real hardware and sets the stale duration. | Blackhole A to B, DELETE via A, kill A within 1 s, restart A, poll B. | This experiment is expected to show the defect. It is **falsified (the defect does not occur)** only if B converges after the restart in every trial. Report the proportion of trials with B stale and the stale duration (bounded by origin `max-age`). |
| **E8m** | **Issuer crash, M3 build only.** With the log on, B converges after the issuer restarts (sandbox: 1.8 ms after readiness, debug, loopback). | The same scenario plus the crash-point list from `m3_durability/remediation/crash_point_test_matrix.md` that can be driven externally (kill after 204, kill during replay, torn tail). Power loss is added here (see `02`). | Any trial where a DELETE answered 204 is not applied on every live peer within `T_replay` after the issuer is back; or a duplicated replay regresses a peer to an older version. Power-loss falsifier: after a hard power cut, a 204-acknowledged DELETE is missing from the log on restart (K18 in the matrix is NOT RUN anywhere). |
| **E9** | **Clock skew does not affect invalidation correctness** (C6: "zero clock dependence"; PROPOSAL to test, because certificates and freshness use wall clocks). | Step one node's clock by about +/- 5 min and +/- 1 h (VM or sandbox host only; see `02`); run E1/E2 and a freshness check; also test cert validity windows (`notBefore`/`notAfter`). | Any E2 violation under skew; or propagation time depends on skew; or an entry is served stale past its origin lifetime measured on the *serving* node's clock beyond the stale cap; or the cluster mTLS handshake fails for a reason that is not certificate validity (record the error text). |
| **E10** | **mTLS identity, rotation and revocation behave as designed.** Identity = SAN `node-<id>.<cluster>.phantom.internal` + SHA-256 fingerprint in the membership file + cluster id + `active` flag (FACT `src/security/cluster.rs:70`, `:287`, `:352` to `:398`). No hot reload of certificates or configuration exists (FACT docs/user/limitations.md line 14); `ClusterMembershipRegistry::update` exists (FACT `cluster.rs`, around line 410) but I found no caller in `src/main.rs`, `src/dist/router.rs` or `src/dist/node.rs` (UNVERIFIED: a grep for `.update(` and reload terms returned nothing; not exhaustive). | Cases: (a) wrong cluster id, unknown node, wrong fingerprint, expired cert, malformed SAN: peer connection must be refused; (b) rotation by restart with the new fingerprint added to `fingerprints` then removed; (c) revocation by setting `active:false` and restarting the other nodes; (d) revocation **without** restart (expected to have no effect because there is no reload; this documents a gap). | A node with an invalid identity is admitted (security failure, highest severity); a rotation with an overlapped fingerprint list causes an outage longer than one restart; a revoked node is still accepted after every other node restarted with the new file. (d) is not a falsifier of the design; it is a measurement of the documented gap. |
| **E11** | **The origin gate holds under cluster load.** The origin concurrency cap (`PHANTOM_ORIGIN_CAP_*`, config.rs:163 to 170) is per gateway; GET is served from the local cache only (FACT limitations.md), so N nodes miss independently (HYPOTHESIS: aggregate origin concurrency up to N times the cap). | Reuse the Scenario B shape (section 8) across N nodes, arms BASE (cap off) and STATIC-CAP, with the origin on its own host. Count origin concurrency on the origin host. | Peak aggregate origin concurrency in the capped arm exceeds `N x cap` (the cap is not enforced per node), or the gateway reports `origin_cap.enforcing=false` in the capped arm, or the cap shows no reduction of origin peak versus BASE (then say so; do not claim a benefit, since adaptive benefit is NOT established, FACT limitations.md). |

(Numbering note: E8m is the M3 variant of E8; E11 is listed after E10 although it is the "origin gate" item.)

## 4. What this study cannot show

State these in every report.

1. It cannot prove linearizability, bounded staleness in general, or safety. It can only fail to falsify specific claims under the tested conditions, on the tested hardware, at the sample sizes used.
2. It cannot establish behaviour on WAN links, across data centres, over the public internet, or on links with sustained packet reordering beyond what is injected.
3. It cannot exercise more nodes than the topology has (3 minimum; 5 extended). Scaling claims beyond that are out of scope.
4. Fault injection emulates failures; it is not the same as a real switch fault or cable pull (a physical cable pull can be added by hand and recorded).
5. Power-loss durability (M3) needs a real power cut or a fault-injecting block device. A VM "power off" tests the hypervisor's write cache behaviour, not a physical disk (`02`).
6. Operating-system effects are confounded with hardware effects unless the OS mix is held constant (`05`, D2).
7. It does not measure the adaptive policy engine (Phantom Intelligence); no benefit is established (FACT limitations.md) and this plan does not test one.
8. A pass on build G says nothing about the M3 build and vice versa.
9. Results from loopback, a VM, WSL2 and physical hosts are different tiers and must not be merged (CLAUDE.md section 4).

## 5. Evidence tiers

Every dataset carries exactly one tier label in its manifest:

| Tier label | Meaning |
|---|---|
| `LOOPBACK` | All processes on one OS instance, 127.0.0.1. |
| `WSL2` | Linux guest under WSL2 on one Windows machine (virtual NIC, NAT or mirrored). Non-physical. |
| `VM_SINGLE_HOST` | Several VMs (Hyper-V or similar) on one machine, virtual switch. Non-physical. |
| `PHYSICAL_LAN` | Separate physical machines, real NICs, one switch, isolated segment. |
| `PHYSICAL_MIXED` | As above with different OS or hardware classes (record each). |

Only `PHYSICAL_LAN` and `PHYSICAL_MIXED` data can support a "multi-host" statement. A `SIMULATED` tier (the project's Tier 1) is not used here at all.

## 6. Acceptance criteria and pre-registration

### 6.1 Rule

All numeric thresholds, sample sizes, seeds and exclusion rules **must be written, dated, hashed and committed to the study folder (or an owner-approved equivalent) before the first measured run**. After the first measured run, a threshold may change only by a dated amendment that states why, and results obtained before the amendment are reported under the old threshold as well. A rehearsal on loopback (`03`, section S) is not a measured run and may be used to calibrate thresholds, provided the calibration is recorded as such. The existing M6 preregistration practice (`docs/research/preregistration_m6_v2*.md`, freeze JSON) is the precedent; the owner decides whether the same freeze mechanism is used (`05`, D8). **I did not read the sealed M6 directory.**

### 6.2 Template (copy into `preregistration_multi_host_<date>.md`)

```
Study id:                      MH-<yyyymmdd>
Registered by / date (machine clock, UTC offset):
Build(s) under test:           G | M3   (binary sha256, git head, dirty entries, source hash)
Topology and tier label:       (from 01_topologies.md)
Hardware and OS table:         (filled from the device list)
Hypotheses and falsifiers:     E1..E11 selected: ______
Primary endpoint per hypothesis, estimator, interval method, alpha:
Thresholds:                    T_prop=__ ms  T_prop_max=__ ms  T_cold=__ ms  T_hold=__ s
                               T_detect=__ s  T_heal=__ s  T_replay=__ s  N_cap_factor=__
Sample sizes:                  trials per pair=__  repetitions=__  seeds=__
Workload:                      (section 8 parameters, rate, keys, alpha, duration)
Warm-up and exclusion rules:   (what is dropped, decided now)
Stopping rule:                 fixed N; no optional stopping
Invalid-run rules:             (section 6.4)
What counts as a negative result and where it is reported:
Amendments:                    (dated list)
Hash of this file:             sha256 = ______  recorded in: ______
```

### 6.3 Recommended thresholds (all PROPOSALS; flagged, to be set by the owner)

The reference points are the loopback debug observations (P99 about 10 to 20 ms warm; cold 35 to 60 ms, up to 117 ms under contention; the 150 ms per-attempt connect/handshake budget; the 250 ms anti-entropy tick). A switched LAN adds well under 1 ms of transit; most of the time is software. A release build should be faster than debug, but that is a hypothesis, not a premise.

| Symbol | Meaning | Proposed value | Rationale |
|---|---|---|---|
| `T_prop` | Upper 95 percent confidence bound of the P99 warm-path propagation time (DELETE 204 to peer's first non-stale answer), per peer pair | 50 ms | Round number above the loopback debug P99 of 20 ms; falsifiable. |
| `T_prop_max` | Largest single warm-path trial | 250 ms | One anti-entropy tick plus one 150 ms attempt, rounded. |
| `T_cold` | Cold path (first invalidation to a destination after start) maximum | 500 ms | Loopback up to 117 ms under contention; WAN-free LAN. |
| `T_hold` | Observation window after first convergence for E2 | 2 s | Longer than any retry or repair tick. |
| `T_detect` | Time for a live node to leave Active for a killed peer | to be calibrated in the rehearsal; no basis yet | I found no documented detection bound; do not invent one. |
| `T_heal` | After a fault is removed, time to converge all pending invalidations | 5 s | PROPOSAL; to be calibrated. |
| `T_replay` | M3 only: time from issuer readiness to convergence | 2 s | Sandbox observed 1.8 ms after readiness (debug, loopback, n small). |
| stale-after-converge | E2 violations | 0 in all trials (hard) | A single violation falsifies. |
| `stale_cap_violations` | gateway counter | 0 (hard, CLAUDE.md section 5) | Existing invariant. |
| Identity admission | E10 invalid identities admitted | 0 (hard) | Security. |
| Origin cap | E11 aggregate peak concurrency | at most `N x cap` | Hypothesis from per-node cap. |

### 6.4 Invalid runs

A run is **invalid**, not failed, and is reported with the reason, if: the NTP/PTP or chrony offset between the observer and any node exceeds the declared bound during a method-P2 run; the observer's send-lag (open-loop lateness) P99 exceeds 100 ms (the existing Lab `contention_flag` rule, FACT docs/demo/scenario_b_artifacts.md); a node process exited unplanned; the fault was not confirmed active by the verification step in `02`; the network was not isolated (`00` section 10); the binary hash differs from the registered one. An invalid run is never silently discarded and never counted toward the sample size.

## 7. Baselines

| Baseline | Purpose |
|---|---|
| **Single node** (one gateway, same host class, same workload) | Separates cluster cost from gateway cost: hit latency, throughput, and origin behaviour without peers. |
| **Unpatched build G** | Reference for every M3 comparison. |
| **Loopback rehearsal of the same scripts** | Shows how much the physical network changes the numbers (labelled `LOOPBACK`, never merged). |
| **No-fault control** inside each fault experiment | Every fault run is interleaved with a no-fault run of the same trial count. |
| **Network floor** | `ping`/TCP-connect RTT and an iperf-class throughput check between every host pair before each session, recorded (tool is an install request, `01`). |

## 8. Workload and seeds

**Reuse (labelled as reuse):** the Scenario B workload parameters, which are the existing real-gateway experiment: Poisson arrivals at 300 req/s, Zipf alpha 0.9 over 5,000 keys, 1 KiB bodies, 31 s per run (warm-up 8 s excluded, healthy 5 s, degraded 10 s, recovery 8 s), origin base service 20 ms with `Cache-Control: max-age=3`, seed 20261030 plus repetition index (FACT docs/demo/scenario_b_runbook.md). The Lab load generator is open-loop and coordinated-omission-safe (latency measured from the scheduled time, FACT docs/demo/scenario_b_artifacts.md).

**Important limit (FACT, `src/lab_live/cli.rs` flags list, lines 31 to 34, and `scenario_b.rs:155`):** `phantom-lab scenario-b` spawns its own gateway and origin on the local machine and has no flag to target remote gateways. It cannot be pointed at a cluster as it stands. Therefore a remote-capable load generator and a standalone origin are needed (listed in `04`, section 6). They should reuse the same sampler, schedule and CSV schema so that the data is comparable; a change to that code is a code change and needs owner approval (CLAUDE.md section 2). This plan does not write them.

| Experiment | Workload |
|---|---|
| E1 to E3 propagation probes | Not the Poisson workload. Dedicated probe keys, one per trial (so each trial starts cold in the sense of one key, warm in the sense of connections), with a **background load** of 0 (idle), and a second series at 300 req/s Zipf 0.9 background to see the effect of load. Probe keys are distinct from background keys. |
| E4 to E8 | Background Scenario B load (300 req/s, Zipf 0.9, 5,000 keys) split across the live nodes, plus the probe sequence. |
| E11 | Scenario B shape across N nodes. The aggregate rate is a design choice (`05`, D9): either 300 req/s total or 300 req/s per node; state which. |

**Seeds.** Seed S0 = 20261030 (the existing value, reuse) for the workload schedule; repetition r uses S0 + r for arrivals, keys and the origin RNG, identical across the arms of repetition r (the Scenario B rule). Fault timing uses a separate seed S1 = 20261031 + r, so that a change in fault timing cannot shift the arrival schedule. Seeds go in the manifest. Probe key names are derived from the seed and a trial counter.

## 9. Statistics (summary; detail in `04`)

- Propagation times are skewed with a long tail. Report the empirical distribution (all raw values), nearest-rank P50/P90/P99/max, and a **bootstrap 95 percent interval for P99** (percentile or BCa, stated), per ordered node pair and pooled with the pair kept visible. Do not report a mean alone.
- Count outcomes (E2 violations, convergence failures) with an exact (Clopper-Pearson) 95 percent interval. "0 of n" is reported as "upper bound 3/n" (rule of three) and never as "zero".
- Repetitions: at least 10 independent repetitions (new processes, new certificates optional) of at least 100 trials per ordered pair each for E1; at least 1,000 trials per pair in total for a P99 claim (the loopback N=1000 run gave P99 19.8 ms). PROPOSAL; the owner fixes the numbers in the pre-registration.
- Warm-up: the first invalidation per destination is the cold path and is analysed separately (it was 35 to 60 ms on loopback).
- Multiple comparisons: with 11 experiments, declare which are confirmatory (pre-registered primary endpoints) and which exploratory; do not mix.
- No optional stopping; no post-hoc exclusion except by the rules in 6.4.

## 10. Risk and safety

- **No production networks.** Use an isolated switch or VLAN with no route to the internet or any production host (`01`, section 5). The nodes are given a JWT secret and, if insecure ingress HTTP is used, serve unencrypted traffic; keep that off any shared network.
- Fault-injection tools need administrator rights and can sever the machine's own connectivity. Always keep an out-of-band path (console or a separate management NIC), set a self-expiring rule (for example a `sleep N; tc qdisc del` watchdog), and never apply rules to the management interface.
- Do not install or change firewall/system settings without owner approval (CLAUDE.md section 2). Every install is a request in `02`, section 8.
- Secrets: the test CA, node keys and JWT secret are lab-only. Generate them fresh per session, keep them out of the repository and out of the evidence bundle (the bundle contains hashes of certificates, never private keys).
- Clock changes (E9) only on VMs or hosts dedicated to the study; restore the clock and verify.
- Cleanup: stop processes, remove firewall/qdisc rules, restore clocks, delete keys, record the cleanup check in the run log (`03`, section 9).
- Do not run the study from a machine holding the sealed M6 material or other protected records.
- AI agents execute nothing here without owner approval; physical devices are an owner blocker (BLOCKED in `docs/design/OWNER_DECISION_REGISTER.md`, as stated in the assignment; I did not find a dedicated entry in the register text I read, so the ID is UNVERIFIED).

## 11. Evidence retention

Reuse the Lab manifest approach (FACT docs/demo/scenario_b_artifacts.md): a `manifest.json` written at the start with status RUNNING and rewritten at the end (COMPLETE, CANCELLED, FAILED), containing the command line, seeds, tier label, per-host hardware/OS, binary path and sha256, git head and dirty entry count, source hash, environment keys set (values for secrets redacted), timer probe, and clock-sync report. Raw data are CSV/JSONL and are hashed (sha256) into a `SHA256SUMS` file that is itself hashed into the manifest. Evidence is write-once: no run directory is edited after COMPLETE. Layout and field lists are in `04`.

## 12. Roles (to be filled by the owner)

| Role | Responsibility | Name |
|---|---|---|
| Study owner | Approves pre-registration, installs, devices, deviation from plan | ________ |
| Operator | Runs the runbook, signs the run log | ________ |
| Second operator / witness | Checks fault verification and cleanup (recommended for E8m power-loss and E9) | ________ |
| Analyst | Runs the analysis scripts on the frozen data; not the person who chose the thresholds (PROPOSAL) | ________ |
| Independent reviewer | Reviews the report against this plan | ________ |

AI agents may draft scripts and analysis; they do not run physical experiments or approve installs.

## 13. Schedule (placeholders)

| Phase | Content | Dates |
|---|---|---|
| P0 | Owner decisions (`05`), device list returned | ____ |
| P1 | Write and review the missing tools (`04`, section 6); loopback rehearsal (`03`, S) | ____ |
| P2 | Non-physical step: WSL2/VM (`01`, option C), clearly labelled | ____ |
| P3 | Pre-registration frozen and hashed | ____ |
| P4 | Physical session 1: E1 to E3, E7 | ____ |
| P5 | Physical session 2: E4 to E6, E10, E11 | ____ |
| P6 | Physical session 3: E8, E9 (and E8m if included) | ____ |
| P7 | Analysis, report, independent review | ____ |

## 14. Reporting rules

The report states for every result: tier label, build, binary hash, hardware and OS, workload, seeds, baseline, repetitions, intervals, limitations (CLAUDE.md section 4). Negative and null results are reported in the same table as positive ones, with the same prominence (`04`, section 5). No sentence in the report may claim linearizability, a staleness bound, or a guarantee without the failure model, the bound and the test that could falsify it.
