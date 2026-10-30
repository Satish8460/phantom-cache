# PhantomCache: project status, pending work and accountability review

**Date:** 2026-10-30, about 11:50 (+05:30). **Status:** snapshot report (historical once written; later status lives in the ledger and the accountability report).
**Audience:** the product owner.
**Basis:** the accountability report (`phantomcache_milestone_accountability_report.md`, last edited 08:26 and therefore partly stale), the master plan, the progress ledger (267 lines), the gate verdicts (`g1_verdict.md`, `g2_verdict.md`), the browser reports (rounds 1 to 3), the vision documents (`vision/`), the storage record, and checks run today (listed in section 11). Evidence tags: **[L]** run by the lead this session, **[A]** reported by an agent and not independently re-run, **[R]** read-only review, **[D]** documentation only.
**Not stated here:** no percentage complete. The accountability report explains why a number would be invented.

## 0. Answers to the five questions

1. **Where does PhantomCache stand?** The single-node engine is strong and heavily tested but its original acceptance criteria were never independently met for any of M1 to M8. Two of the five delivery gates are passed (G1 Scenario B, G2 Lab API, both with dispositions), G3 (UI plus browser) is in its last cycle, and G4 (two whole-product reviews) and G5 (commits and push) have not started. Nothing is committed. The headline research hypothesis (adaptive origin control beats a static cap) was tested and not supported.
2. **What is genuinely completed and verified?** See section 1.2. In short: the sealed M6 evaluation and its integrity repairs; Scenario B on a real gateway with independently recomputed numbers; the Lab HTTP API after two security-review rounds; a real-browser verification of the Lab UI in three rounds; ten core defects fixed with failing-before tests; the storage cleanup.
3. **What is pending and why?** Section 6. The main reasons are sequencing (reviews need a stable tree), owner decisions (anything in frozen code, quotas, M3 durability, SEC dispositions) and research that cannot start without something only you can supply (a v3 salt, real traces).
4. **Plan and timeline?** Sections 8 and 9. The critical path to a first push, if no owner decision blocks it, is about 4 to 6 working days of agent time. This is an estimate, not a commitment.
5. **What from your instructions is not fulfilled?** Section 10.

## 1. Milestones M1 to M8

### 1.1 Table

Status vocabulary follows the accountability report: IMPLEMENTED, TESTED, INDEPENDENTLY VERIFIED, UNVERIFIED. "Independently verified" means someone other than the author checked retained evidence or code. Reading-only reviews are labelled as such.

| M | Original objective | Done and evidence | Outstanding | Blockers | Gate / verdict |
|---|---|---|---|---|---|
| **M1** Truth and baseline | Trustworthy, measurable single-node path | Telemetry v2 with provenance (15 honesty tests), byte accounting I1 to I8 (16 tests), known-gap tests, CORRECTIONS.md [L, in sweeps] | Benchmarks never re-run on the current tree; approvals A-1 to A-14 never recorded; M1 "PASS" was agent-reported | R-16 needs a quiet machine and the release profile | **Partially implemented. AC not demonstrated** |
| **M2** Core correctness | Verified freshness, stale, invalidation | Freshness engine, stale cap, fenced finalize, 304 merge, canonical keys, idle-CPU fix, R-01 origin cap; five read-only reviews found no Critical defect [R] | `PHANTOM_DEFAULT_TTL_MS` is unwired (decision made: keep the 60 s default, patch not applied); 2 h soak; release-profile suite; S-4 percentiles; L3 coalescing window | Applying the TTL patch touches frozen `Cargo.toml` and needs the addendum-2 batch | **Complete-ish in code, tested. AC not demonstrated** |
| **M3** Distributed | Explicit invalidation contract with evidence | All-peer fan-out, repair ledger, mTLS identity binding, fault proxy; loopback only, debug only [L suites] | **Durability defect reproduced:** invalidations owed by an issuing gateway are lost if it dies. Fix prototyped in a scratch copy only. No release-profile or multi-host data; C6 ("zero stale after 50 ms") unsupported; membership static; the ADR-003 "ACCEPTED" was agent-authored | M3 fix touches frozen router/main/config files: needs your approval plus an independent review | **Partially implemented; weakly verified. AC not met** |
| **M4** Lab and baselines | Reproducible experiment platform | Deterministic T1 runner and `phantom-lab` binary, 12,000-cell dry run, Scenario B runner (G1 PASS), Lab API (G2 PASS) | T1/T2 agreement study; second-machine reproduction; several baseline arms; cluster scenarios | Second machine unavailable | **Partially complete** |
| **M5** Intelligence | Auditable intelligence path | `src/intelligence/` machinery (signals, shadow, engine, executor, candidates) tested; nothing outside that directory uses it (V1 grep) | Integration, overhead, any live decision trace; observation wiring | Needs your approval for application-code changes | **Machinery implemented, not integrated; hypothesis not supported; disposition: superseded by MVP-B for the serving path** |
| **M6** Research evaluation | Integrated evaluation and MVP decision | v2/v2.1/v2.2 preregistrations, freeze record (56 files) and addendum 1; sealed run executed once: **H1 NOT_SUPPORTED** | RT-0 prior-art sweep; claims C3, C4, C7; H3/H5/H6; clean-machine reproduction | The sealed set is spent; a new protocol needs your offline-generated salt | **Executed; negative. Partly complete** |
| **M7** Productization | Release-ready, secure | Config validation, health endpoints, SEC-M7-01 to 08 fixed, Lab API hardening | **SEC-M7-09/10/11 open**, no advisory scan, no graceful drain, no perf gate, no clean-machine reproduction, no release decisions | Advisory-scan tool needs your approval; release variant, version and licence are yours | **Partially implemented. G-RELEASE not attempted** |
| **M8** UI/UX | Apple-inspired, honest, accessible UI | Lab UI built; three browser rounds; 212/212 UI tests [L, 11:4x] | See section 3: N-6/N-7 low findings, residual target sizes, N-2 unresolved, no assistive-technology pass, no usability study | None technical | **Implemented; browser-verified for Lab; AC not fully demonstrated** |

### 1.2 What is genuinely verified (higher confidence)
- **Sealed M6 result and its integrity chain** [L]: immutable v2 record, v2.1 and v2.2 amendments, run-once marker, fail-closed verifier with addendum 1.
- **Scenario B (G1)** [L]: numbers recomputed by an independent Node path from the raw CSVs. Both capped arms are TRADE_OFF_FAILURE. The binding interpretation notice governs wording.
- **Lab API (G2)** [L plus two security-review rounds, R]: 841 then 851 tests passing across 75 targets, 0 failed, 8 ignored, clippy exit 0 (sweep 10:11 to 10:23). Remaining dispositions G2-D1 to D7 are listed in `g2_verdict.md`.
- **Browser verification** [A, with lead spot checks of the report text]: three rounds, one real Chrome 156. Round 3 report: layout fixes confirmed at 320 to 1280 px, N-5 flow confirmed. Only about 12 of about 330 round-2 screenshots and 8 of round 3 were opened by the agents.
- **Storage cleanup** [L]: described in section 5.

### 1.3 Not verified, and often confused with verified
Passing tests do not establish correctness. A gate "PASS" does not mean a milestone passed. No milestone has passed its original acceptance criteria in an independently verified way. The earlier "Phase 4 accepted / zero P0" handoff claim is not supported.

## 2. Intelligence and research

**Implemented:** the machinery listed in M5 (library code, tested, not in the serving path). The only intelligence in the serving path is the R-01 origin concurrency cap, active only in canary or active mode when configured.

**Evaluated:** AIMD/PIE-style adaptive origin control. Sealed M6 (simulation, Tier 1): H1 NOT_SUPPORTED. Scenario B (real gateway, loopback, 10 reps): both capped arms trade client success for lower origin concurrency; the AIMD arm behaved like a static cap near 12. Investigated causes [A, code-checked by the lead for the main ones]: AIMD starts at its maximum, uses an absolute p99 ceiling, the window p99 equals the max for 99 samples or fewer, a hard-coded 0.05 error trigger, and an objective that rewards the smallest cap. **This does not show that intelligent caching is not viable.** It shows this controller, on these plants, gave no benefit over a static cap.

**Independently validated:** nothing in the intelligence area has been independently validated beyond the code-reading reviews and the lead's recomputation of Scenario B.

**Alternatives under investigation (research documents, not experiments):** V1 (13 candidates plus 9 extra ideas, ranked), V2 (forecasting architecture and evaluation), and the consolidated report. V1's top ranks: Observe-mode wiring for truthful hot keys and change alarms; a Demand Calendar with deterministic pre-warming; a "Phantom Advisor" (what-if replay plus cap sizing); uncertainty-aware decisions with an outcome ledger; early flash-crowd detection. V2's central design: run an oracle-warm headroom test (E9) before building any forecaster, and make a plain "warm the declared set on a schedule" baseline a kill gate.

**Predictive demand beyond concept?** No. It exists as a design with a schema, a baseline list and kill criteria. Nothing is coded, no experiment has been run, no real traffic trace exists, and no warm or prefetch actuator exists.

**What remains before an implementation is justified:**
1. A pre-registered protocol v3 (E2 headroom draft exists, unfrozen) with a salt that **you** generate offline.
2. E1: a plant where the right adaptation visibly pays off (a load-dependent origin), so a result cannot be an artifact of a plant that rewards nothing.
3. E9: the oracle-warm headroom test.
4. Your approval for the first application-code step (Observe-mode wiring and event tap).
5. A prior-art sweep (RT-0) before any novelty or positioning claim. V1 ran 20 searches, which is not RT-0 and found no source either way.

**Next concrete experiment:** the E2 exploratory dry run on development seeds once its tooling exists, followed by E1. Neither can start as a pre-registered confirmatory run until the salt exists. Dry runs on development seeds are allowed.

## 3. Flagship demonstration and product experience

| Capability | Real today | Simulated / logical | Proposed | Unsupported |
|---|---|---|---|---|
| Phantom Lab (T1 runs, Scenario B runs, evidence browser, run history, cancel, SSE) | Yes, anonymous on loopback or JWT; verified in a real browser (3 rounds) | T1 results are SIMULATED by label | | Live T1 progress not observed (runs finished in under 2 s) |
| Basic cache lifecycle (MISS then HIT) | Yes in the engine and the old dashboard | | A per-request event view | Per-request event stream does not exist |
| Predictive intelligence demo | | | Calendar plus warmer; replay act | Live prediction (no predictor) |
| Multi-tenant | Namespace isolation by JWT `tenant_id` | | Quotas, tenant telemetry | Capacity isolation (a tenant flood can evict others) |
| Multi-device / pairing | LAN works with JWT, CORS and `PHANTOM_ALLOW_LAN_ORIGIN` | | Pairing tool, scoped short-lived tokens | Revocation, token minting |
| Security demos | Several attack classes covered by existing tests (V4 list) | | Evidence runner, `phantom-verify` | Signed or hash-chained bundles |
| Node failure / recovery | Peer-offline repair and failover for DELETE, loopback | **`NODE_FAILURE` is a logical flag** | | Anything that relies on durable invalidation (M3 defect) |
| Evidence and reproducibility | Hash manifests; T1 deterministic | | Evidence bundle format | `manifest.rs:103` hard-codes `git_tree_hash` (not evidence of the real tree) |

**Browser review, rounds 1 to 3.** Round 1 found F-01 to F-11 (reflow, nav, targets, focus, favicon and others). Round 2 (against the rebuilt gateway) confirmed F-02, F-04 to F-08, F-09, F-10 and the VALID wording as fixed and F-01/F-03 as partly fixed, and found N-1a to N-5. Round 3 (after the UI fix round) reports N-1a, N-1b, N-3 (summary rows and checkbox hit areas), N-4 and N-5 as fixed and N-2 (the stall on repeated cold loads) as **not reproduced in 28 loads, cause still unknown**. It found two low items: **N-6** (Overview page 14 px sideways scroll at 320 px, pre-existing) and **N-7** (Simulation demo buttons 21 px high at 1280). Residuals still below the stated target: skip link 39 px, two text inputs 36 px, 24 px radio and checkbox controls. Not verified: mobile:true, real zoom, Safari/Firefox, screen readers, non-text contrast.

**Pending G3 verdict:** a formal G3 PASS or PASS-with-dispositions has not been recorded. Recommendation: record it after fixing N-6/N-7 and re-checking only those pages.

## 4. Security and distributed-system gaps

Severity is my assessment from the cited documents. "Required action" lists what a real fix needs, not that I have done it.

| Issue | Severity | Current disposition | Required action | Dependencies |
|---|---|---|---|---|
| Tenant resource fairness: no per-tenant quotas, shared eviction and origin cap | High for multi-tenant claims; Medium otherwise | Open; documented as "namespace isolation only" | Per-tenant byte budget, fair eviction, tenant telemetry (V3/V4 D-04) | Touches frozen `cache.rs`; your approval plus independent review |
| Origin protection per tenant | Medium | Global cap only; a bypass flood returns 503 for everyone | Per-tenant share of the cap; bypass rate limit | Same as above |
| `x-cache-bypass` authorization | Medium | Any `cache:read` principal can bypass to origin | Dedicated role or rate limit | `server.rs` change; approval |
| Peer invalidation authorization | Medium | Any authenticated cluster member can send tenant-scope or full-cache invalidations (`node.rs:547-557`) | Per-scope authorization on peer frames | Frozen dist files; approval |
| JWT lifecycle: no revocation, no `jti`, no max lifetime, `kid` not wired, `/api/events` outlives token | Medium | SEC-M7-11 OPEN | Revocation list or short lifetimes, `nbf`, stream expiry | `main.rs`/`server.rs`; fix or formal acceptance |
| SEC-M7-09 (bare token / `requester` alias), SEC-M7-10 (SSRF ranges) | Low to Medium (labels) | OPEN | Fix with tests, or your written acceptance | Small; can start now, no frozen file needed per the register (verify) |
| Device pairing and scoped credentials | Medium | Not built; no minter | External minter script first (V3 D1) | Your decision on trust bootstrap |
| Multi-node topology | Medium | Static membership; at most 3 to 5 nodes, loopback, debug | Dynamic membership; multi-host rehearsal | Large; deferred |
| M3 invalidation durability | **High for any "delete everywhere" claim** | Reproduced; opt-in durable log prototyped in scratch only | Integrate after review (design recommends opt-in, fsync always, fail-open, drain default 0, 7-day age-out) | Your approval; addendum-2 batch |
| Node failure and recovery claims | Medium | `NODE_FAILURE` is logical; GET is local-only; no read failover | Label all demos; never present as real | None for labelling |
| Evidence integrity / build provenance | Medium | `git_tree_hash` hard-coded; no signed bundles | Replace with a real tree hash; evidence runner | `src/lab` is frozen: addendum-2 batch |
| Dev mode anonymous, trusts `x-tenant-id` | Medium (by design) | Loopback-only guard in Lab API | Label as insecure baseline | None |
| No dependency advisory scan | Medium | Needs your tool approval | Run `cargo audit` equivalent | Your approval |
| No graceful drain | Medium | Open (R-05) | Drain on SIGTERM with tests | `main.rs` |

## 5. Documentation and storage

**Storage cleanup: complete for the authorised scope** (record: `storage_cleanup_2026-10-30.md`).
- **Category B:** ten build directories removed, each re-checked against the inventory immediately before removal; 112,469 files, 107,803,669,160 logical bytes (100.4 GiB). **Measured free-space gain: about 90.1 GiB** (107.41 to 197.50 GiB). The roughly 10 GiB gap is unexplained; hardlinks are an untested hypothesis. Recorded binary hashes remain only as text.
- **Category A:** seven snapshot folders copied to `.local/snapshots/` and SHA-256 verified against the originals, 0 mismatches (re-verified today). Four protected originals (`phase2-exit`, `phase3`, `phase6-entry`, `phase6-exit`) have an Everyone-DENY ACL and stay in `D:\Personal\phantom-cache-snapshots` by your decision; their contents are unverified. An empty directory `.local/snapshots/phase2-exit-…` is a leftover from my first failed copy.
- **Uncertain items retained:** `_m7_target` (still holds the debug binary used for browser verification), `_m7_target.zip` (6.9 GB, corrupt archive, unidentified), `D:\Temp\cf_old_target` (4.2 GB), active TEMP scratch, unidentified root files (`D:\h1.json`, `h2.json`, `rr.patch`, `wtb_*.js`), `D:\CargoTarget` root, `.phantom-sealed-m6`.

**Documentation audit:** four read-only audit agents started at about 11:42 and are running (D1 milestones, D2 `docs/research`, D3 everything else, D4 mechanical link and term checks). No results yet. Findings known from other work: the milestone numbering has five coexisting schemes; M8 appears in only two files; CLAUDE.md sections 5 and 6 contain stale remarks (60 s TTL literal lines now `server.rs:1405` and `:1688`; faithful eviction is in the serving cache at `cache.rs:209`); `docs/security/security-demo-matrix.md` names files and errors that do not exist; `docs/demo-lab/device-topologies.md` is partly stale; the accountability report and the master plan were not updated after 08:26 for G2 and G3 progress.

## 6. Accountability table (all tasks you assigned)

ETAs are my working estimates in agent working days from now (2026-10-30), assuming no owner decision is outstanding unless marked. They are not deadlines you set; you set none.

| # | Task | Original expectation | Status | Evidence | Why incomplete | Dependencies | Owner | ETA |
|---|---|---|---|---|---|---|---|---|
| 1 | M1 to M8 execution | Each milestone to its AC | **IMPLEMENTED / PARTIAL** (none COMPLETE) | Accountability report | Original AC were never frozen per package; many gaps in section 1 | Owner decisions listed | Lead | See individual rows |
| 2 | Honest audit and reconciliation of M1 to M6 | Independent audit, corrections | **COMPLETE** | Master plan, accountability report | n/a | | Lead | done |
| 3 | G1 Scenario B | Real-gateway experiment, verified | **COMPLETE (PASS with dispositions D1 to D5)** | `g1_verdict.md`, Node recompute | Dispositions remain (optional stale-while-revalidate run, etc.) | | Lead | done; dispositions 1 to 2 days |
| 4 | G2 Lab API | Implemented, reviewed | **COMPLETE (PASS with dispositions G2-D1 to D7)** | `g2_verdict.md`, 851 tests | Dispositions open (e.g. D2 release-gateway run, TerminateProcess review) | | Lab API writer / Lead | 1 to 2 days |
| 5 | G3 UI plus browser | UI on real API, browser-verified, defects fixed | **IN PROGRESS (last cycle)** | Browser reports 1 to 3, 212/212 UI tests | N-6, N-7, residual targets, N-2 cause, formal verdict not written | None technical | UI writer, browser agent | 0.5 day |
| 6 | G4 two whole-product review cycles | Cycle 1 then Cycle 2, severity-classified, fixes with regression tests | **NOT STARTED as defined** (many per-area reviews done) | Ledger | Needs a stable tree after G3 | G3 verdict | Independent reviewers | 2 to 3 days |
| 7 | G5 delivery: ~12 logical commits, per-commit tests, push (no force) | Reviewed commits on a feature branch | **NOT STARTED** | Commit plan in ledger | Must follow G4; scratch-worktree compile check | G4; addendum-2 batch | Lead | 1 day after G4 |
| 8 | R-01 freeze addendum | Standard suite green | **COMPLETE** | Addendum 1 plus pin | n/a | | Lead | done |
| 9 | Freeze addendum 2 batch (TTL patch, E1/E2 module registration, M3 patch, `manifest.rs` fix, one-time rustfmt) | Single batched change to frozen files | **PENDING** | TTL and M3 patches in repo | Frozen-file changes should be batched; M3 needs your approval | Owner approval for M3; independent review | Lead | 1 day after approval |
| 10 | Default TTL decision | Decide and handle | **COMPLETE (decision), DEFERRED (patch)** | `ttl_decision/` | Patch touches frozen `Cargo.toml` | Row 9 | Lead | with row 9 |
| 11 | M3 durability: investigate, do not merge | Reproduce, propose | **COMPLETE (investigation), REQUIRES OWNER DECISION (integration)** | `m3_durability/` | Merge forbidden without approval | Owner approval; independent review | Cluster writer | 2 days after approval |
| 12 | Intelligence investigation (alternatives to AIMD, preserve ambition) | Explain failure, propose alternatives | **COMPLETE (investigation)** | Report section 11, V1, V2 | n/a | | Lead | done |
| 13 | E2 exploratory dry run, E1 plant | Headroom evidence | **NOT STARTED** | E2 draft only | Tooling not built; confirmatory use needs your salt | Row 9 for module registration | Intelligence writer | 3 to 5 days |
| 14 | Protocol v3 (new salt and seed sets) | New pre-registered evaluation | **REQUIRES OWNER DECISION** (salt) | E2 draft | Salt must be generated by you offline; only its SHA-256 goes in the repo | You | Owner | cannot estimate |
| 15 | RT-0 prior-art closure | Close or withdraw novelty claims | **PENDING** | Coverage notes only | Needs a dedicated sweep | | Research agent | 1 to 2 days |
| 16 | Phantom Lab flagship demo (backed by real backend) | Lab API plus UI | **IMPLEMENTED** (verified as in rows 4, 5) | | Flagship story needs extra backend (event tap etc.) | Vision decisions | | See rows 22 to 25 |
| 17 | UI/UX Apple-inspired, responsive, accessible, browser-verified | Real-browser verification | **IN PROGRESS** | Rounds 1 to 3 | Row 5 | | | 0.5 day |
| 18 | Security: SEC-M7-09/10/11, advisory scan, drain | Open items closed or accepted | **PENDING** | Register | Not yet scheduled ahead of G3; scan needs your tool approval | Owner approval for scan | Security writer | 1 to 2 days |
| 19 | Security and multi-tenant investigations | Investigate | **COMPLETE (investigation)** | V3, V4 | Fixes need frozen-file approvals | Owner decisions D-04 etc. | | done |
| 20 | Docs R-07a amendments, wording corrections | Correct claims | **COMPLETE** | ADR-003, limitations, ledger | | | | done |
| 21 | Release-profile general suite, 2 h soak, benches rerun, `cargo fmt` | Fresh baselines | **NOT STARTED** | Accountability report section 4 | Needs a quiet machine and no competing builds; fmt deferred (38,042-line diff) | G4 tree stable | Lead | 1 day |
| 22 | V1 to V5 vision research plus consolidation | Report with 13 sections | **COMPLETE (research)** | `vision/00_…` | Report is hypothesis, not an implementation | | | done |
| 23 | Decisions D-01 to D-40 from the vision report | Owner decisions | **REQUIRES OWNER DECISION** | Report section 13 | Yours | You | Owner | n/a |
| 24 | Flagship "Launch Day" demo build | Build toward one coherent demo | **NOT STARTED** | V5 storyboard | Needs event tap, quotas, durable invalidation (prerequisites) | D-04, D-05, D-06 | | 2 to 3 weeks after approvals (estimate) |
| 25 | Demo Center UI | Unified UI | **NOT STARTED** | V5 | Same as row 24 | | | same |
| 26 | Storage cleanup and consolidation | Safe cleanup | **COMPLETE for the authorised scope** | `storage_cleanup_2026-10-30.md` | Four protected snapshots intentionally left; uncertain items deferred | | | done |
| 27 | Documentation consistency workstream | Audit, standard, cleanup | **IN PROGRESS** (audit phase) | D1 to D4 running | | | Lead, D1 to D4 | audit 1 to 2 hours; plan 0.5 day |
| 28 | Ledger, master plan, accountability report, claims ledger kept consistent | Four documents agree | **IN PROGRESS** | Ledger is current; accountability report and master plan are stale since 08:26 | Not yet updated for G2, G3 progress and the vision phase | | Lead | 0.5 day |
| 29 | CLAUDE.md stale remarks | Correct | **REQUIRES OWNER DECISION** (owner-controlled file) | D3 will list edits | | You | | n/a |
| 30 | Browser-process and TEMP hygiene | Never touch user's Edge or the codex node | **COMPLETE (observed throughout)** | Reports | | | | ongoing |

## 7. Why each pending task is still pending

| Task | What remains | Why | Justified? | Blocker type | Unblocks via | Parallel? | Needs you? |
|---|---|---|---|---|---|---|---|
| G3 verdict | N-6/N-7 fix, targeted re-check, written verdict | Round 3 finished minutes ago | Yes | Sequencing | One UI fix round | Yes | No |
| G4 | Two whole-product cycles | A review needs a stable tree; reviews were deferred behind G1/G2/G3 | Yes | Sequencing | G3 verdict | Cycle 1 sub-reviews in parallel; Cycle 2 after fixes | No |
| G5 | Commits and push | Must follow G4; owner forbade git changes before it | Yes | Sequencing and policy | G4 | Partly | No (push uses your normal PR workflow) |
| Addendum 2 batch | TTL, E1/E2 registration, M3, manifest, rustfmt | Batched deliberately to avoid repeated freeze updates | Yes | Policy (frozen files) | M3 approval and review | After G4 | **Yes (M3)** |
| M3 integration | Merge with tests | You asked for investigation first | Yes | Organisational | Your approval and an independent review | Yes | **Yes** |
| SEC-M7-09/10/11, scan, drain | Fixes or acceptance | Prioritised behind the demo path; not escalated earlier | **Partly: should have been scheduled before the vision phase** | Scheduling | Assign a writer now | Yes | Scan tool |
| Release-profile suite, soak, benches | Run on a quiet machine | Cannot run while agents build or while measuring | Yes | Resource | Schedule after G4 | No (exclusive machine) | No |
| RT-0 | Dedicated prior-art sweep | Not scheduled | **No, it has been open since M6** | Scheduling | Assign a research agent | Yes | No |
| E1/E2/E9 experiments | Tooling, then runs | Need module registration (frozen), then your salt for confirmatory | Yes | Research plus policy | Addendum 2; your salt | Dev-seed dry runs only | **Yes (salt)** |
| Vision decisions | D-01 to D-40 | Yours | n/a | Owner | You | n/a | **Yes** |
| Flagship demo and Demo Center | Build | Prerequisites (quotas, durable invalidation, event tap) require approvals | Yes | Architectural | D-04/05/06 | After prerequisites | **Yes** |
| Doc standardisation | Audit then plan then execute | Started today | Yes | Sequencing | Phase 1 audit | Yes | Plan approval |
| Accountability and master plan refresh | Update with G2, G3, vision, cleanup | I let the accountability report age since 08:26 | **No. Overdue; I own this.** | Discipline | Do it this session | Yes | No |

## 8. Prioritised plan

**P0: critical correctness, security, milestone blockers**

| Action | Deliverable | Owner | Depends on | Acceptance | Date (est.) | Parallel |
|---|---|---|---|---|---|---|
| Fix SEC-M7-09 and 10 (and decide 11) | Tests plus fixes or your written acceptance | Security writer | None known | Failing-before tests; no frozen file touched, or listed | 11-01 | Yes |
| Record G3 verdict after N-6/N-7 | `g3_verdict.md` | Lead plus UI writer | N-6/N-7 | Narrow browser re-check on affected pages | 10-30 (today) to 11-01 | Yes |
| Refresh accountability report, master plan and claims ledger | Consistent status | Lead | None | Docs agree on G1 to G3, sweep counts, storage | 10-30 | Yes |
| M3 integration decision | Your yes or no | You | | Decision recorded | your timing | |

**P1: active milestone gates**

| Action | Deliverable | Owner | Depends on | Acceptance | Date (est.) | Parallel |
|---|---|---|---|---|---|---|
| G4 Cycle 1 | Findings classified; fixes with regression tests | Independent reviewers | G3 verdict | Per accountability section 5 | 11-01 to 11-03 | Sub-reviews yes |
| G4 Cycle 2 | Fresh adversarial review incl. real browser and release readiness | New reviewers | Cycle 1 fixes | No valid Critical/High left | 11-03 to 11-04 | Partly |
| Addendum-2 batch | Frozen-file changes with verifier updated | Lead | M3 approval; review | Suite green; original records untouched | 11-04 | No |
| G5 commits and push | ~12 reviewed commits | Lead | G4 | Per-commit tests; scratch-worktree compile; final sweep | 11-05 | No |
| Release-profile suite, soak, benches, rerun | Retained logs | Lead | Quiet machine | Exit 0; counts recorded | 11-04 to 11-05 | No |

**P2: intelligence and research**

| Action | Deliverable | Owner | Depends on | Acceptance | Date (est.) |
|---|---|---|---|---|---|
| RT-0 prior-art sweep | Coverage record with sources | Research agent | None | Verified sources; claims table | 11-01 to 11-02 |
| E1 plant and E2 dry run (dev seeds) | Exploratory results, labelled | Intelligence writer | Addendum 2 | Reproducible, SIMULATED | 11-05 to 11-10 |
| E9 oracle-warm headroom test | Result: headroom exists or not | Intelligence writer | E1 | Kill criteria from V2 | 11-10 to 11-14 |
| Protocol v3 | Frozen protocol | Lead | **Your salt** | Commitment only in repo | cannot estimate |

**P3: product experience**

| Action | Deliverable | Date |
|---|---|---|
| Event tap (E0) with measured overhead | Bounded tiered per-request events | after your D-06 |
| Per-tenant accounting | Quotas, fair eviction, tenant telemetry | after your D-04 |
| Out-of-process warmer prototype | Tagged warm GETs, no gateway change | after D-09/10 |
| Launch Day flagship | Truthful interim version on current code first | 2 to 3 weeks, estimated |

**P4: documentation and maintenance**

| Action | Deliverable | Date |
|---|---|---|
| Doc audit plan, then incremental execution with link and status checks | `doc_audit/` plus standard and templates | plan 10-31; execution after approval |
| Docs-lint script (proposed by D4) | Automated checks with a historical allowlist | with execution |

## 9. Timeline

- **Today (10-30):** refresh accountability/master plan/claims ledger; N-6/N-7 UI fix; doc audit results and my consolidated audit; start RT-0; assign security writer for SEC-M7-09/10.
- **1 to 2 working days:** G3 verdict; SEC-M7-09/10 fixes; advisory-scan request answered; doc plan approved; G4 Cycle 1 begins.
- **3 to 5 working days:** G4 Cycle 1 fixes and Cycle 2; addendum-2 batch (if M3 approved); release-profile suite, soak, benches; G5 commits and push.
- **Longer:** E1/E2/E9 (1 to 2 weeks); event tap, quotas, durable invalidation, flagship demo (2 to 3 weeks after approvals, estimate); M7 residuals needing you (release variant, version, licence, clean-machine reproduction).
- **Critical path:** G3 verdict, then G4 Cycle 1, then fixes, then Cycle 2, then addendum 2, then G5. Earliest first push: **about 2026-11-05**, only if no valid Critical/High finding needs a larger redesign and the M3 decision arrives by 11-03.
- **Cannot be estimated:** protocol v3 (your salt), any real-trace-based forecasting (no trace), multi-host and release decisions (your devices and choices), the flagship's final scope (D-01 to D-40).
- **Milestone completion dates:** I will not give dates for M1 to M8 completion because their original AC were never frozen and several need owner decisions. What I can commit to is gate dates above.

## 10. Tasks From Your Instructions That Are Not Yet Completed

| Instruction | Exact status |
|---|---|
| Two mandatory whole-product review cycles (G4) | NOT STARTED as defined |
| Logical commits and push at G5 | NOT STARTED |
| Complete M1 to M8 | None complete; see section 1 |
| Browser-verified Apple-inspired UI | Lab UI verified in a real browser with low residuals; formal G3 verdict missing; no assistive-technology pass |
| Release-profile engine tests, soak, benches | NOT STARTED |
| `cargo fmt` | Deferred (38,042-line diff) |
| Investigate invalidation durability but do not merge without approval | Investigated and reproduced; integration awaits your approval |
| Decide TTL | Decided (keep 60 s); patch deferred to addendum 2 |
| E2 exploratory dry run, E1 plant, v3 protocol | NOT STARTED / needs your salt |
| Prior-art (RT-0) | Pending; 20 searches by V1 are not RT-0 |
| SEC-M7-09/10/11, advisory scan, drain | Open; not scheduled ahead of the vision phase |
| Keep the ledger, master plan, accountability report and claims ledger consistent | **Behind:** accountability report and master plan not updated since 08:26 |
| Documentation consistency workstream | Audit running; no standard, no cleanup yet |
| Flagship demo (stages A to F, 3 devices), Demo Center, predictive and security demos | Researched and designed (V1 to V5); **not built**; prerequisites need your decisions |
| Seven playgrounds list and demo audience | **Waiting for you** (the vision agents asked) |
| CLAUDE.md corrections | Waiting for you; D3 will list the edits |

**Categories you asked for**
- **Brainstormed only:** predictive demand intelligence, Demand Calendar, Phantom Advisor, multi-device pairing, flash-crowd detection, tenant intelligence.
- **Investigated but not implemented:** M3 durable log (scratch prototype), TTL wiring (patch), E2 headroom protocol (draft), security evidence runner.
- **Implemented but not independently verified:** the intelligence machinery (library code, not integrated); the Lab UI at runtime with assistive technology; the SEC-M7-01/02/03 fixes beyond two read-only reviews; the UI after the N-fixes beyond round 3's agent measurements.
- **Explicitly deferred:** PIE integration (R-20), T1/T2 agreement study, second-machine reproduction, weighted-rendezvous evaluation, dynamic membership, protocol model.
- **Silently dropped:** none found. **Superseded:** WP1 to WP5 and phase numbering; the 3-node demo cluster; the first lab runner and the M6 v1 report (retracted); the MVP-A idea.

## 11. How this report was checked (and its limits)
- Run today by me: `git rev-parse HEAD` (456bb25), `git status --short` (72 lines before this report), `node --test ui/test` (212 tests, 212 pass, 0 fail), process and storage checks (storage record).
- Not re-run today: the Rust suite (last full run 10:11 to 10:23: 851 passed, 0 failed, 8 ignored; clippy exit 0; build hash recorded), so any source change after 10:23 is not covered. No Rust source has changed since 10:10 [L].
- Agent-reported and not independently re-derived: details of rounds 2 and 3 of the browser review beyond the screenshots the agents say they opened; the V1 to V5 code claims except the 28 the integration writer spot-checked (27 held, 1 was wrong).
- ETAs are estimates and depend on owner decisions marked above.
