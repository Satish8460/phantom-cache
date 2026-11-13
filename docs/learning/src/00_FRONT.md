---
title: "PhantomCache Engineering and Technical Defence Handbook"
subtitle: "From first principles to the deepest implementation detail"
---

# About this handbook

**Purpose.** This book teaches you what PhantomCache is, how it works inside, why it was built this way, and how to defend it in front of technically strong judges. It starts with the problem, not the product, and ends with an honest account of what is not finished.

**Who it is for.** A reader who knows programming basics and is building deeper expertise in distributed systems and systems engineering. Technical terms are always explained where they first appear.

**State of the project described here.** The text describes the repository as it was on 2026-10-30, on the feature branch `feat/publish-phantomcache-core`. Parts of the working tree were still uncommitted while the book was written, and several repair patches (U-01, G4A-01, M3 durable invalidation) exist only as sandbox work that is not merged. Where a statement depends on the exact state, it says "at the time of writing".

## How the chapters are organised
1. **The Problem and the Vision.** Caching from first principles, why it breaks down, and what PhantomCache tries to solve.
2. **System Architecture.** The components, the request paths, the routes and the configuration, with diagrams.
3. **Technology Stack and Engineering Decisions.** Rust, Tokio, Axum and the rest, and which choices are recorded decisions versus reasoning after the fact.
4. **Core Cache Engine and Algorithms.** Sharded locks, single-flight, expiry, eviction, fencing and epochs, with worked examples.
5. **Distributed Systems and Workload Intelligence.** Rendezvous hashing, invalidation, failure behaviour, and what the adaptive control really is.
6. **Security, Reliability and Performance.** The failure-scenario matrix, what protections exist, and what evidence supports any number.
7. **Phantom Lab, Experimentation and Innovation.** The simulator, the real-gateway experiments, the negative result, and the evidence-based innovation argument.
8. **Limitations, Roadmap and Technical Defence.** What is incomplete, what you must not claim, and the hardest questions.

The appendices at the end give a one-page overview, the architecture explained once, a technology and algorithm reference, a glossary, a consolidated judge-question bank, a source-file map, an implementation-versus-roadmap matrix and the ten concepts to master.

## Three ways to read it
- **One hour before a presentation:** Appendix A, Appendix H, then the judge bank (Appendix E) and the claims-you-must-not-make table in Chapter 8.
- **One day:** Chapters 1, 2, 4 and 8, then the judge boxes of Chapters 5 to 7.
- **To master it:** read the chapters in order, do the knowledge checks and the codebase exploration tasks at the end of each chapter, and answer the challenge questions aloud.

## The status labels used everywhere
Every claim about a capability carries one of these labels. They describe evidence, not hope.

| Label | Meaning |
|---|---|
| **IMPLEMENTED** | The code exists in the repository. It says nothing about whether it works. |
| **TESTED** | A named test exercises it. The text says what the test proves and what it does not. |
| **INDEPENDENTLY VERIFIED** | A human or external reviewer verified it. Almost nothing in the project reaches this level, and the book says so. |
| **PARTIAL** | Part of the behaviour exists. |
| **PROTOTYPE ONLY** | Exists as sandbox or library code that the running gateway does not use. |
| **DESIGNED BUT NOT IMPLEMENTED** | A design exists; the code does not. |
| **DEFERRED** | Consciously postponed. |
| **BLOCKED** | Cannot proceed until an owner decision or another piece of work. |
| **UNVERIFIED** | Could not be checked; the text says why. |

## What this book does and does not establish
- It was written and quality-checked by AI agents, each chapter against the current code. Reviews described as "AI second look" are not human verification. No part of the project, including this book, has had an independent human review.
- Passing tests are not proof of correctness. Several tests assert stub behaviour, and the book names them.
- Performance numbers appear only with their build profile, host, workload, seed, repetitions and limits. Loopback, simulated and physical-host results are kept separate. The book makes no claim that PhantomCache is faster or better than any other cache.
- Documents in the repository sometimes disagree with the code. Each chapter records the discrepancies it found; none were silently fixed.

## Judge boxes
Look for the boxes headed **JUDGE MAY ASK**. Each holds a question a technically strong judge might ask, a suggested answer you can say in 30 to 60 seconds, a deeper follow-up, and the evidence to know. If a feature is incomplete, the answer says so honestly. That is deliberate: the strongest defence is an accurate one.
