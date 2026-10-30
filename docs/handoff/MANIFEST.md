> Notice (2026-10-30): This manifest describes the handoff package as of 2026-10-28 and is not edited. Its characterisations (for example "zero known gaps" and "formal ... acceptance") are superseded by [handoff CORRECTIONS](CORRECTIONS.md) and `CLAUDE.md` section 6; the files it lists are historical (see the notice on each). Current status: [docs/STATUS.md](../STATUS.md).
>
> Links in the text below, including `file:///d:/...` links, are unchanged and work only on the original machine.
>
> Original text below is unchanged.

# PhantomCache — AI Handoff Package Manifest

**Document ID:** `docs/handoff/MANIFEST.md`  
**Purpose:** Master catalog and index of all deliverables in the AI-to-AI handoff package.  
**Audience:** Incoming AI assistants, systems architects, and project stakeholders.  
**Authoritative Baseline Date:** October 28, 2026  

---

## 1. Master Deliverable Catalog

| Document ID | File Path | Purpose | Primary Sources / Code Paths | Coverage & Gaps |
| :--- | :--- | :--- | :--- | :--- |
| **00** | [`00_START_HERE.md`](file:///d:/Personal/phantom-cache/docs/handoff/00_START_HERE.md) | Primary entry point, immediate operating rules, safeguards, and quick start guide. | `Cargo.toml`, Git baseline | Covers ground rules, repository layout, immediate commands. Zero known gaps. |
| **01** | [`01_PROJECT_MASTER_CONTEXT.md`](file:///d:/Personal/phantom-cache/docs/handoff/01_PROJECT_MASTER_CONTEXT.md) | Exhaustive technical architecture, data flows, subsystems, and threat model. | `src/core/`, `src/dist/`, `src/security/`, `src/api/` | Covers P1–P4 and Stages 3A–4C. Distinguishes implemented vs planned features. |
| **02** | [`02_PHASE_HISTORY_AND_STATUS.md`](file:///d:/Personal/phantom-cache/docs/handoff/02_PHASE_HISTORY_AND_STATUS.md) | Chronological phase history, audit findings, remediations, and test count progression. | `docs/security/`, `tests/integration/` | Reconciles test counts from 220 -> 234 -> 242 tests across 19 targets. |
| **03** | [`03_ARCHITECTURE_AND_PROTOCOL_REFERENCE.md`](file:///d:/Personal/phantom-cache/docs/handoff/03_ARCHITECTURE_AND_PROTOCOL_REFERENCE.md) | Binary frame formats, sequence spaces, generational fencing, and anti-entropy algorithms. | `src/dist/frame.rs`, `src/dist/anti_entropy.rs`, `src/dist/router.rs` | Complete binary layouts for 0x01 through 0x08. Mathematical memory formulas. |
| **04** | [`04_TESTS_BENCHMARKS_AND_EVIDENCE.md`](file:///d:/Personal/phantom-cache/docs/handoff/04_TESTS_BENCHMARKS_AND_EVIDENCE.md) | Complete test target inventory, commands, benchmark measurements, and evidence. | `tests/`, `Cargo.toml` | Full inventory of 242 tests. Distinguishes loopback benchmarks from physical LAN. |
| **05** | [`05_OPEN_RISKS_AND_NEXT_STEPS.md`](file:///d:/Personal/phantom-cache/docs/handoff/05_OPEN_RISKS_AND_NEXT_STEPS.md) | Prioritized risk register (P0–P2), pre-deployment checklist, and proposed roadmap. | `docs/security/phase-4-design.md` | Documents zero open P0 code blockers; outlines proposed Phases 5–9. |
| **06** | [`06_AGENT_WORKFLOW_AND_GIT_SAFEGUARDS.md`](file:///d:/Personal/phantom-cache/docs/handoff/06_AGENT_WORKFLOW_AND_GIT_SAFEGUARDS.md) | Agent engineering workflow, test discipline, audit gates, and Git safety invariants. | Git config, Cargo configuration | Explicit rules prohibiting destructive checkout, clean, or unauthorized commits. |
| **07** | [`07_CONVERSATION_AND_DECISION_INDEX.md`](file:///d:/Personal/phantom-cache/docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md) | Index of user instructions, design sessions, audit findings, and transcript files. | `docs/handoff/transcripts/`, `transcript_copy.jsonl` | References full transcripts of AG Session 1 & 2; notes ChatGPT export status. |
| **08** | [`08_NEW_AI_ONBOARDING_PROMPT.md`](file:///d:/Personal/phantom-cache/docs/handoff/08_NEW_AI_ONBOARDING_PROMPT.md) | Copy-ready prompt for the user to initialize the next AI coding assistant. | `docs/handoff/00_START_HERE.md` | Clean markdown prompt containing all mandatory rules and first actions. |
| **Manifest** | [`MANIFEST.md`](file:///d:/Personal/phantom-cache/docs/handoff/MANIFEST.md) | This master inventory and index document. | `docs/handoff/` | Catalog of all 10 handoff documents and transcripts. |

---

## 2. Preserved Transcripts Inventory

| File Path | Size | Description |
| :--- | :---: | :--- |
| [`docs/handoff/transcripts/ag_conversation_session_1_oct11_to_oct25.jsonl`](file:///d:/Personal/phantom-cache/docs/handoff/transcripts/ag_conversation_session_1_oct11_to_oct25.jsonl) | 1.28 MB | Steps 0 to 1,344 of Antigravity Session 1 (`3ee3c52a-c7bd-475f-a0aa-faa69eff6e74`). Initial foundation, P1/P2, and Phase 3 transport design. |
| [`docs/handoff/transcripts/ag_conversation_session_2_oct25_to_oct28.jsonl`](file:///d:/Personal/phantom-cache/docs/handoff/transcripts/ag_conversation_session_2_oct25_to_oct28.jsonl) | 13.34 MB | Complete raw transcript of Antigravity Session 2 (`eaca48fa-9967-405d-b4d3-54047091810e`). Phase 4A–4C implementation, audits, and Gate B remediations. |
| `transcript_copy.jsonl` (workspace root) | 1.28 MB | Preserved untouched in root workspace directory. |

---

## 3. Existing Authoritative Security & Architectural Documents

The handoff package indexes and references the following pre-existing authoritative documentation in `docs/security/`:
- **[`docs/security/phase-4-final-acceptance-report.md`](file:///d:/Personal/phantom-cache/docs/security/phase-4-final-acceptance-report.md):** Formal independent acceptance sign-off for Phase 4 (242 tests passing, Clippy clean).
- **[`docs/security/phase-4-design.md`](file:///d:/Personal/phantom-cache/docs/security/phase-4-design.md):** Architectural design specification for distributed invalidation, acting-primary failover, and anti-entropy.
- **[`docs/security/stage-4c-progress.md`](file:///d:/Personal/phantom-cache/docs/security/stage-4c-progress.md):** Progress report detailing Stage 4C implementation and Gate B remediations.
- **[`docs/security/phase-3-design.md`](file:///d:/Personal/phantom-cache/docs/security/phase-3-design.md):** Transport security specification (mTLS 1.3, binary framing, connection pooling).
- **[`docs/security/phase-2-specification.md`](file:///d:/Personal/phantom-cache/docs/security/phase-2-specification.md):** Upstream egress and anti-SSRF policy specification.
- **[`docs/security/phase-1-specification.md`](file:///d:/Personal/phantom-cache/docs/security/phase-1-specification.md):** Zero-trust ingress and role-based access control specification.
- **[`docs/PHASE_2_WORK_REPORT.md`](file:///d:/Personal/phantom-cache/docs/PHASE_2_WORK_REPORT.md):** Course Project Work Report on PhantomCache (BCS703B).

