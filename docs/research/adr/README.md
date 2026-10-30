# ADR index

**Status:** Active (index)
**Audience:** architects, reviewers, the product owner
**Last reviewed:** 2026-10-30
**Authority and related:** the ADR files are the records; this index only lists them and quotes their headers as written. No ADR was changed. Owner decisions on ratification are open items B-2 and B-8 of [../doc_audit/00_consolidated_documentation_audit_and_plan.md](../doc_audit/00_consolidated_documentation_audit_and_plan.md). ADR template: [../../contributing.md](../../contributing.md) section 5.

| ADR | Title | Status as written in the file | Date | Decision makers as written | Owner ratification |
|---|---|---|---|---|---|
| [ADR-001](ADR-001-present-architecture-and-direction.md) | Present Architecture, Gaps, and Recommended Direction | PROPOSED (awaiting owner approval) | 2026-10-28 | not stated in the file | Not recorded in the repository (evidence gap; absence is not proof of no authorization) |
| [ADR-002](ADR-002-serving-path-contract.md) | Serving-Path Contract (Coalescing, Cancellation, Timeouts, and Fencing) | Accepted | 2026-10-29 | no "decision makers" line; header names Author: Antigravity (AG) | Not recorded in the repository (evidence gap; absence is not proof of no authorization) |
| [ADR-003](ADR-003-distributed-architecture.md) | Distributed Cache Architecture, Failure Model, and Consistency Contract | ACCEPTED (Gate G-ARCH Resolution); dated amendment 2026-10-30 07:26 +05:30 | 2026-10-29 | Antigravity (Implementation and Verification Agent), Claude (Research Lead Architecture) | Not recorded in the repository (evidence gap; absence is not proof of no authorization) |

## Reading notes

- **ADR-001 versus ADR-003.** ADR-001 is a present-state analysis and a recommendation; its direction was later decided in ADR-002 and ADR-003, and the file carries no supersession note ([../doc_audit/d2_research_docs_audit.md](../doc_audit/d2_research_docs_audit.md) section 4). Whether to mark it Superseded is an owner decision (audit item B-2/D2 open question 5); this index does not change its status.
- **ADR-003 amendment.** The text above the amendment is unchanged; the amendment (section "Amendment 2026-10-30 07:26") qualifies statements that overstate what the implementation guarantees (invalidation durability and the staleness bound). Read ADR-003 together with that amendment, with [../../user/limitations.md](../../user/limitations.md), and with claim C6 in [../claims_ledger.md](../claims_ledger.md).
- **Both "accepted" ADRs were accepted by AI agents** per their own headers, and [../../../CLAUDE.md](../../../CLAUDE.md) section 2 requires owner approval for architectural changes. The accountability report records that the owner did not select the architecture option ([../phantomcache_milestone_accountability_report.md](../phantomcache_milestone_accountability_report.md), M3 row). Treat "Accepted" as agent-accepted until the owner records a decision.
- **Decision-shaped records outside `adr/`**: [../g1_verdict.md](../g1_verdict.md), [../g2_verdict.md](../g2_verdict.md), [../ttl_decision/recommendation.md](../ttl_decision/recommendation.md) (default TTL, proposed, nothing applied), [../m3_durability/design.md](../m3_durability/design.md) (invalidation durability, proposed, nothing integrated), and owner-decision lists in the ledger ([../progress_ledger.md](../progress_ledger.md)). There is no single decision register yet.
- **Adding an ADR:** use the next number, fill the template sections (context, decision, alternatives, consequences, ratified by owner and date), add a row here, and never edit an accepted ADR body; amend with a dated section.
