# G4A-01 analysis (src/core/cache.rs, main tree as of HEAD 3e6e03e + uncommitted work)

Evidence tags: V = verified in code.

| Step | Location | Lock / ordering held | Note |
|---|---|---|---|
| shard mutex acquired | :1411 | shard mutex, held to end of fn (guard `shard`) | V |
| begin_fetch stamps `entry_*_epoch` = epochs at fetch start | :1255, :1370-1371 | shard mutex | so check #1 compares fetch-start epochs vs now |
| fence check #1: `get_effective_epochs` + compare | :1462-1466 | shard mutex; inside: epoch_snapshot RwLock read (released on return, :562-567), `global_epoch` SeqCst load (:568) | the two epoch reads are not one atomic snapshot |
| body/metadata prep, byte checks, eviction loop (<=32) | :1490-1516 | shard mutex | unbounded-ish window; widened by evictions |
| re-read #2: `get_effective_epochs` | :1517 | shard mutex + brief snapshot read | the flaw |
| stamp entry with re-read epochs | :1532-1533 | shard mutex | new epoch laundered onto old body |
| `try_invalidate_tenant` | :576-607 | tenant_epochs mutex -> epoch_snapshot write (:577, :600); NO shard mutex | can run at any point while a shard is held |
| `try_invalidate_all` | :613-632 | tenant_epochs mutex -> `global_epoch` store -> snapshot write; NO shard mutex | |
| hit path / begin_fetch compare | :656, :1261 | shard mutex | compares stamped vs current; sees laundered entry as fresh |

Reasoning: the shard mutex serialises shard readers, but invalidation does not participate in it, so an epoch bump can complete between :1466 and :1517. The body was fetched before the bump; check #1 passed; check #2 does not exist, only a re-read, so the entry is stamped with the new epoch and every later read passes :656. Served until TTL.

Lock order (CLAUDE.md s5): finalize holds shard and only touches the epoch_snapshot RwLock (read); invalidate holds tenant_epochs then snapshot (write). No inverse edge, no deadlock (the sandbox test invalidated from another thread while the shard mutex was held and completed). The documented "shard then tenant_epochs" order is not exercised in finalize; the real issue is the absence of any ordering between invalidation and the shard-held window, not a lock-order violation.

Generation fencing: `delete` bumps `generation`; epoch invalidation does NOT bump per-key generation, so the generation check (:1463) cannot catch an epoch bump. Epoch equality is the only fence, and it is defeated by the re-read.

Other note: the global-fallback path in :587-592 stores `global_epoch` before the snapshot is cleared (G4A-17 in the review); not investigated here.

Severity framing: needs an invalidation to land within the finalize window; impact is a pre-invalidation body served after an acknowledged invalidation, until TTL.
