# Manifest of the two-layer build (generated 2026-10-30T12:46:59.422Z)

Word-count check (alphanumeric tokens): original 125398 = Part I 73256 + Part II 52142 + removed duplicates 0: **EQUAL**.
Figures: original 33, Part I 32, Part II 1. Judge boxes: original 93, Part I 23, Part II 70.
Cross-references in Part I (approximate regex): 111, of which 10 point to sections now in Part II.

| id | heading | layer | group | words | judge kept | judge moved | table words extracted |
|---|---|---|---|---|---|---|---|
| ch01 | Chapter 1: The Problem and the Vision | learning |  | 85 | 0 | 0 | 0 |
| ch01:what-you-will-learn | What you will learn | learning |  | 92 | 0 | 0 | 0 |
| ch01:1.1 | 1.1 What a cache is | learning |  | 508 | 0 | 0 | 0 |
| ch01:1.2 | 1.2 Why applications need caching | learning |  | 955 | 0 | 2 | 0 |
| ch01:1.3 | 1.3 Where caching goes wrong | learning |  | 1563 | 0 | 1 | 0 |
| ch01:1.4 | 1.4 Why one cache is not enough: the case for distribution | learning |  | 1128 | 1 | 0 | 0 |
| ch01:1.5 | 1.5 What PhantomCache is, in one minute | learning |  | 317 | 1 | 0 | 0 |
| ch01:1.6 | 1.6 Pillar 1: Distributed systems engineering | learning |  | 463 | 0 | 0 | 266 |
| ch01:1.7 | 1.7 Pillar 2: Workload intelligence | learning |  | 880 | 0 | 1 | 0 |
| ch01:1.8 | 1.8 Pillar 3: Product excellence, observability and demonstrability | learning |  | 386 | 0 | 0 | 0 |
| ch01:1.9 | 1.9 Intended users and practical use cases | learning |  | 244 | 0 | 0 | 0 |
| ch01:1.10 | 1.10 How the vision and the code relate (and where documents disagree) | reference | R5 | 429 | 0 | 0 | 0 |
| ch01:1.11 | 1.11 Where to look for the truth | learning |  | 378 | 0 | 1 | 0 |
| ch01:key-ideas-to-remember | Key ideas to remember | learning |  | 146 | 0 | 0 | 0 |
| ch01:what-you-should-now-understand | What you should now understand | learning |  | 113 | 0 | 0 | 0 |
| ch01:knowledge-check | Knowledge check | learning |  | 529 | 0 | 0 | 0 |
| ch01:codebase-exploration-tasks | Codebase exploration tasks | learning |  | 144 | 0 | 0 | 0 |
| ch01:challenge-questions | Challenge questions | learning |  | 348 | 0 | 0 | 0 |
| ch02 | Chapter 2: System Architecture | learning |  | 100 | 0 | 0 | 0 |
| ch02:what-you-will-learn | What you will learn | learning |  | 108 | 0 | 0 | 0 |
| ch02:2.1 | 2.1 Two viewpoints | learning |  | 559 | 0 | 0 | 0 |
| ch02:2.2 | 2.2 The component map | learning |  | 791 | 1 | 0 | 0 |
| ch02:2.3 | 2.3 The HTTP surface | learning |  | 645 | 0 | 0 | 311 |
| ch02:2.4 | 2.4 Request flow 1: a cache hit | learning |  | 701 | 0 | 1 | 0 |
| ch02:2.5 | 2.5 Request flow 2: a miss, coalescing and the origin fetch | learning |  | 1141 | 0 | 1 | 0 |
| ch02:2.6 | 2.6 Freshness, stale serving and origin failure | learning |  | 927 | 1 | 1 | 0 |
| ch02:2.7 | 2.7 DELETE, the cluster, and how nodes communicate | learning |  | 1483 | 0 | 2 | 0 |
| ch02:2.8 | 2.8 Where Phantom Lab and the UI sit | learning |  | 535 | 0 | 1 | 0 |
| ch02:2.9 | 2.9 Configuration and deployment | learning |  | 947 | 0 | 1 | 301 |
| ch02:2.10 | 2.10 Auth boundaries at a high level | learning |  | 151 | 0 | 0 | 0 |
| ch02:2.11 | 2.11 Discrepancies between documents and code | reference | R5 | 233 | 0 | 0 | 0 |
| ch02:key-ideas-to-remember | Key ideas to remember | learning |  | 170 | 0 | 0 | 0 |
| ch02:what-you-should-now-understand | What you should now understand | learning |  | 118 | 0 | 0 | 0 |
| ch02:knowledge-check | Knowledge check | learning |  | 519 | 0 | 0 | 0 |
| ch02:codebase-exploration-tasks | Codebase exploration tasks | learning |  | 135 | 0 | 0 | 0 |
| ch02:challenge-questions | Challenge questions | learning |  | 328 | 0 | 0 | 0 |
| ch03 | Chapter 3: Technology Stack and Engineering Decisions | learning |  | 79 | 0 | 0 | 0 |
| ch03:what-you-will-learn | What you will learn | learning |  | 115 | 0 | 0 | 0 |
| ch03:3.0 | 3.0 How to read this chapter: recorded decision versus reasoning | learning |  | 461 | 0 | 0 | 0 |
| ch03:3.1 | 3.1 The dependency picture in one table | reference | R1 | 376 | 0 | 0 | 0 |
| ch03:3.2 | 3.2 Rust | learning |  | 889 | 1 | 1 | 0 |
| ch03:3.3 | 3.3 Async programming and Tokio | learning |  | 705 | 0 | 1 | 0 |
| ch03:3.4 | 3.4 Axum, hyper and tower | learning |  | 837 | 0 | 1 | 0 |
| ch03:3.5 | 3.5 Concurrency primitives and the concurrency model | learning |  | 1976 | 1 | 1 | 0 |
| ch03:3.6 | 3.6 Data structures as actually used | reference | R1 | 228 | 0 | 0 | 0 |
| ch03:3.7 | 3.7 The TLS and mTLS stack | learning |  | 653 | 0 | 1 | 0 |
| ch03:3.8 | 3.8 JWT: jsonwebtoken | learning |  | 193 | 0 | 0 | 0 |
| ch03:3.9 | 3.9 serde, JSON and the binary frame protocol | learning |  | 217 | 0 | 0 | 0 |
| ch03:3.10 | 3.10 Build, packaging and deployment: what exists and what does not | learning |  | 272 | 0 | 0 | 186 |
| ch03:3.11 | 3.11 The frontend: React, Vite, plain CSS, hand-built SVG | learning |  | 877 | 0 | 1 | 0 |
| ch03:3.12 | 3.12 Testing and benchmarking tools | learning |  | 496 | 0 | 1 | 0 |
| ch03:3.13 | 3.13 Supply-chain observations (verified list) | reference | R5 | 357 | 0 | 0 | 0 |
| ch03:key-ideas-to-remember | Key ideas to remember | learning |  | 126 | 0 | 0 | 0 |
| ch03:what-you-should-now-understand | What you should now understand | learning |  | 101 | 0 | 0 | 0 |
| ch03:knowledge-check | Knowledge check | learning |  | 385 | 0 | 0 | 0 |
| ch03:codebase-exploration-tasks | Codebase exploration tasks | learning |  | 132 | 0 | 0 | 0 |
| ch03:challenge-questions | Challenge questions | learning |  | 269 | 0 | 0 | 0 |
| ch04 | Chapter 4: Core Cache Engine and Algorithms | learning |  | 92 | 0 | 0 | 0 |
| ch04:what-you-will-learn | What you will learn | learning |  | 116 | 0 | 0 | 0 |
| ch04:4.0 | 4.0 How to read the status labels | reference | R6 | 97 | 0 | 0 | 0 |
| ch04:4.1 | 4.1 Storage and lookup | learning |  | 481 | 0 | 0 | 0 |
| ch04:4.2 | 4.2 Sharded mutexes (partitioned buckets) | learning |  | 1476 | 0 | 2 | 0 |
| ch04:4.3 | 4.3 Concurrency in this engine: what runs where | learning |  | 136 | 0 | 0 | 0 |
| ch04:4.4 | 4.4 Request coalescing, single-flight and stampedes | learning |  | 1426 | 0 | 2 | 0 |
| ch04:4.5 | 4.5 Entry lifecycle and fencing (generation, instance, epoch) | learning |  | 979 | 1 | 0 | 0 |
| ch04:4.6 | 4.6 Memory management and byte accounting | learning |  | 697 | 0 | 1 | 0 |
| ch04:4.7 | 4.7 Invalidation and epochs | learning |  | 1682 | 1 | 1 | 0 |
| ch04:4.8 | 4.8 Freshness, TTL, stale serving and revalidation | learning |  | 1302 | 0 | 1 | 0 |
| ch04:4.9 | 4.9 Eviction | learning |  | 1836 | 1 | 1 | 0 |
| ch04:4.10 | 4.10 What happens when the origin fails | learning |  | 473 | 0 | 1 | 0 |
| ch04:4.11 | 4.11 Capability status table | reference | R1 | 254 | 0 | 0 | 0 |
| ch04:4.12 | 4.12 Where this chapter disagrees with other documents | reference | R5 | 226 | 0 | 0 | 0 |
| ch04:key-ideas-to-remember | Key ideas to remember | learning |  | 146 | 0 | 0 | 0 |
| ch04:what-you-should-now-understand | What you should now understand | learning |  | 144 | 0 | 0 | 0 |
| ch04:knowledge-check | Knowledge check | learning |  | 178 | 0 | 0 | 0 |
| ch04:codebase-exploration-tasks | Codebase exploration tasks | learning |  | 145 | 0 | 0 | 0 |
| ch04:challenge-questions | Challenge questions | learning |  | 271 | 0 | 0 | 0 |
| ch04:answers-to-the-knowledge-check | Answers to the knowledge check | learning |  | 353 | 0 | 0 | 0 |
| ch05 | Chapter 5: Distributed Systems and Workload Intelligence | learning |  | 92 | 0 | 0 | 0 |
| ch05:what-you-will-learn | What you will learn | learning |  | 277 | 0 | 0 | 0 |
| ch05:5.1 | 5.1 Distributed caching fundamentals, and the choice PhantomCache made | learning |  | 688 | 0 | 0 | 0 |
| ch05:5.2 | 5.2 Cluster membership and node-to-node communication | learning |  | 1488 | 0 | 2 | 0 |
| ch05:5.3 | 5.3 Rendezvous hashing | learning |  | 1816 | 0 | 2 | 0 |
| ch05:5.4 | 5.4 Epochs and versioning across nodes | learning |  | 890 | 0 | 1 | 0 |
| ch05:5.5 | 5.5 Invalidation propagation | learning |  | 1466 | 1 | 1 | 0 |
| ch05:5.6 | 5.6 Consistency: what is guaranteed, under which failure model | learning |  | 1270 | 1 | 1 | 0 |
| ch05:5.7 | 5.7 Node failure, recovery, partitions, and concurrency across nodes | learning |  | 1108 | 1 | 1 | 0 |
| ch05:5.8 | 5.8 The peer GetRequest stub, DR-029 and DR-027 | learning |  | 697 | 0 | 1 | 0 |
| ch05:5.9 | 5.9 The durability gap and the M3 sandbox work | learning |  | 372 | 0 | 0 | 0 |
| ch05:5.10 | 5.10 Workload intelligence: what exists and where it runs | learning |  | 579 | 0 | 0 | 270 |
| ch05:5.11 | 5.11 Telemetry: the histogram, with the maths | learning |  | 657 | 0 | 1 | 0 |
| ch05:5.12 | 5.12 Probabilistic structures, hot-key detection and workload patterns | learning |  | 779 | 0 | 1 | 0 |
| ch05:5.13 | 5.13 The adaptive algorithms, one by one | learning |  | 1377 | 0 | 0 | 0 |
| ch05:5.14 | 5.14 The M6 result: no benefit from adaptivity (sealed) | learning |  | 739 | 2 | 0 | 0 |
| ch05:5.15 | 5.15 Chapter status summary | reference | R1 | 298 | 0 | 0 | 0 |
| ch05:key-ideas-to-remember | Key ideas to remember | learning |  | 142 | 0 | 0 | 0 |
| ch05:what-you-should-now-understand | What you should now understand | learning |  | 123 | 0 | 0 | 0 |
| ch05:knowledge-check | Knowledge check | learning |  | 210 | 0 | 0 | 0 |
| ch05:codebase-exploration-tasks | Codebase exploration tasks | learning |  | 203 | 0 | 0 | 0 |
| ch05:challenge-questions | Challenge questions | learning |  | 297 | 0 | 0 | 0 |
| ch05:answers-to-the-knowledge-check | Answers to the knowledge check | learning |  | 442 | 0 | 0 | 0 |
| ch06 | Chapter 6: Security, Reliability and Performance | learning |  | 127 | 0 | 0 | 0 |
| ch06:what-you-will-learn | What you will learn | learning |  | 111 | 0 | 0 | 0 |
| ch06:6.1 | 6.1 Threat model and trust boundaries | learning |  | 454 | 0 | 0 | 0 |
| ch06:6.2 | 6.2 Authentication and authorisation | learning |  | 866 | 0 | 1 | 0 |
| ch06:6.3 | 6.3 Management and API access | learning |  | 755 | 0 | 1 | 0 |
| ch06:6.4 | 6.4 Peer-to-peer trust | learning |  | 1107 | 1 | 0 | 0 |
| ch06:6.5 | 6.5 Input validation, body limits, timeouts and connection management | learning |  | 900 | 0 | 1 | 0 |
| ch06:6.6 | 6.6 Tenant isolation and cache poisoning | learning |  | 878 | 0 | 1 | 0 |
| ch06:6.7 | 6.7 Data exposure and token handling | learning |  | 295 | 0 | 0 | 0 |
| ch06:6.8 | 6.8 Resource exhaustion, origin overload and SSRF | learning |  | 1131 | 0 | 2 | 0 |
| ch06:6.9 | 6.9 Failure recovery, lost invalidations and races | learning |  | 1584 | 1 | 1 | 0 |
| ch06:6.10 | 6.10 Memory safety and supply chain | learning |  | 545 | 0 | 1 | 0 |
| ch06:6.11 | 6.11 The security and failure-scenario matrix | reference | R1 | 1284 | 0 | 0 | 0 |
| ch06:6.12 | 6.12 Evaluating performance correctly | learning |  | 1208 | 0 | 2 | 0 |
| ch06:6.13 | 6.13 Every real measurement in the repository | reference | R5 | 1279 | 0 | 0 | 0 |
| ch06:6.14 | 6.14 Reliability, health and bottlenecks | learning |  | 827 | 0 | 2 | 0 |
| ch06:key-ideas-to-remember | Key ideas to remember | learning |  | 139 | 0 | 0 | 0 |
| ch06:what-you-should-now-understand | What you should now understand | learning |  | 104 | 0 | 0 | 0 |
| ch06:knowledge-check | Knowledge check | learning |  | 493 | 0 | 0 | 0 |
| ch06:codebase-exploration-tasks | Codebase exploration tasks | learning |  | 133 | 0 | 0 | 0 |
| ch06:challenge-questions | Challenge questions | learning |  | 277 | 0 | 0 | 0 |
| ch07 | Chapter 7: Phantom Lab, Experimentation and Innovation | learning |  | 111 | 0 | 0 | 0 |
| ch07:what-you-will-learn | What you will learn | learning |  | 126 | 0 | 0 | 0 |
| ch07:7.1 | 7.1 What Phantom Lab is, and why it exists | learning |  | 597 | 0 | 0 | 0 |
| ch07:7.2 | 7.2 Deterministic simulation, step by step | learning |  | 1778 | 0 | 0 | 0 |
| ch07:7.3 | 7.3 Reading the numbers: metrics, paired differences and BCa intervals | learning |  | 966 | 0 | 0 | 0 |
| ch07:7.4 | 7.4 Reproducibility, preregistration and evidence retention | learning |  | 1036 | 0 | 0 | 0 |
| ch07:7.5 | 7.5 T2: Scenario B on a real gateway over loopback | learning |  | 1207 | 0 | 0 | 0 |
| ch07:7.6 | 7.6 The M6 result: a negative outcome, reported as such | learning |  | 477 | 0 | 0 | 0 |
| ch07:7.7 | 7.7 The Lab API and UI | learning |  | 357 | 0 | 0 | 0 |
| ch07:7.8 | 7.8 Innovation: building the argument from evidence | learning |  | 2127 | 0 | 0 | 0 |
| ch07:7.9 | 7.9 Judge boxes | learning |  | 2051 | 3 | 9 | 0 |
| ch07:7.10 | 7.10 Discrepancies found while writing this chapter | reference | R5 | 168 | 0 | 0 | 0 |
| ch07:key-ideas-to-remember | Key ideas to remember | learning |  | 129 | 0 | 0 | 0 |
| ch07:what-you-should-now-understand | What you should now understand | learning |  | 90 | 0 | 0 | 0 |
| ch07:knowledge-check | Knowledge check | learning |  | 463 | 0 | 0 | 0 |
| ch07:codebase-exploration-tasks | Codebase exploration tasks | learning |  | 139 | 0 | 0 | 0 |
| ch07:challenge-questions | Challenge questions | learning |  | 327 | 0 | 0 | 0 |
| ch08 | Chapter 8: Limitations, Roadmap and Technical Defence | learning |  | 54 | 0 | 0 | 0 |
| ch08:what-you-will-learn | What you will learn | learning |  | 119 | 0 | 0 | 0 |
| ch08:8.1 | 8.1 How to read this chapter | learning |  | 229 | 0 | 0 | 0 |
| ch08:8.2 | 8.2 Where the project stands | learning |  | 983 | 0 | 0 | 480 |
| ch08:8.3 | 8.3 Architectural limitations | learning |  | 773 | 0 | 0 | 0 |
| ch08:8.4 | 8.4 Known defects and risks, by theme | reference | R5 | 1337 | 0 | 0 | 0 |
| ch08:8.5 | 8.5 Incomplete and deferred functionality | learning |  | 275 | 0 | 0 | 0 |
| ch08:8.6 | 8.6 Design trade-offs | learning |  | 354 | 0 | 0 | 0 |
| ch08:8.7 | 8.7 Technical debt and documentation honesty | reference | R5 | 678 | 0 | 0 | 0 |
| ch08:8.8 | 8.8 Remediation status: what is committed and what is not | learning |  | 434 | 0 | 0 | 0 |
| ch08:8.9 | 8.9 Future architecture | learning |  | 746 | 0 | 0 | 0 |
| ch08:8.10 | 8.10 Production-maturity checklist | reference | R1 | 258 | 0 | 0 | 0 |
| ch08:8.11 | 8.11 Claims you must NOT make | learning |  | 602 | 0 | 0 | 0 |
| ch08:8.12 | 8.12 Implementation versus roadmap matrix | reference | R1 | 646 | 0 | 0 | 0 |
| ch08:8.13 | 8.13 Preparing to defend the project | learning |  | 2382 | 4 | 8 | 0 |
| ch08:key-ideas-to-remember | Key ideas to remember | learning |  | 135 | 0 | 0 | 0 |
| ch08:what-you-should-now-understand | What you should now understand | learning |  | 83 | 0 | 0 | 0 |
| ch08:knowledge-check | Knowledge check | learning |  | 452 | 0 | 0 | 0 |
| ch08:codebase-exploration-tasks | Codebase exploration tasks | learning |  | 156 | 0 | 0 | 0 |
| ch08:challenge-questions | Challenge questions | learning |  | 355 | 0 | 0 | 0 |
| appA | Appendix A: PhantomCache on one page | learning |  | 584 | 0 | 0 | 0 |
| appB | Appendix B: The complete architecture, explained once | reference | R6 | 115 | 0 | 0 | 0 |
| appB:B.1 | B.1 What the system is, in one paragraph | reference | R6 | 134 | 0 | 0 | 0 |
| appB:B.2 | B.2 The process and what runs in it | reference | R6 | 179 | 0 | 0 | 0 |
| appB:B.3 | B.3 The front door: ingress, authentication, tenants | reference | R6 | 252 | 0 | 0 | 0 |
| appB:B.4 | B.4 The cache core: 8 shards | reference | R6 | 204 | 0 | 0 | 0 |
| appB:B.5 | B.5 A hit | reference | R6 | 77 | 0 | 0 | 0 |
| appB:B.6 | B.6 A miss: single-flight, then the origin | reference | R6 | 588 | 0 | 0 | 0 |
| appB:B.7 | B.7 Memory, freshness and eviction | reference | R6 | 416 | 0 | 0 | 0 |
| appB:B.8 | B.8 DELETE and the cluster | reference | R6 | 650 | 0 | 0 | 0 |
| appB:B.9 | B.9 Observation and intelligence | reference | R6 | 236 | 0 | 0 | 0 |
| appB:B.10 | B.10 Phantom Lab and the UI | reference | R6 | 305 | 0 | 0 | 0 |
| appB:B.11 | B.11 Security boundaries in brief | reference | R6 | 137 | 0 | 0 | 0 |
| appB:B.12 | B.12 Where the architecture stops | reference | R6 | 134 | 0 | 0 | 0 |
| appC | Appendix C: Technology and algorithm reference | reference | R1 | 105 | 0 | 0 | 0 |
| appC:C.1 | C.1 Technologies | reference | R1 | 452 | 0 | 0 | 0 |
| appC:C.2 | C.2 Algorithms and mechanisms | reference | R1 | 1000 | 0 | 0 | 0 |
| appC:C.3 | C.3 How to use the tables in a defence | reference | R1 | 97 | 0 | 0 | 0 |
| appD | Appendix D: Glossary and abbreviations | reference | R3 | 4903 | 0 | 0 | 0 |
| appE | Appendix E: Consolidated judge-question bank | reference | R4 | 194 | 0 | 0 | 0 |
| appE:ten-hard-questions-to-rehearse-first | Ten hard questions to rehearse first | reference | R4 | 158 | 0 | 0 | 0 |
| appE:never-say | Never say | reference | R4 | 476 | 0 | 0 | 0 |
| appE:1-fundamentals | 1. Fundamentals | reference | R4 | 619 | 0 | 0 | 0 |
| appE:2-architecture | 2. Architecture | reference | R4 | 1028 | 0 | 0 | 0 |
| appE:3-technology | 3. Technology | reference | R4 | 1310 | 0 | 0 | 0 |
| appE:4-algorithms | 4. Algorithms | reference | R4 | 1801 | 0 | 0 | 0 |
| appE:5-distributed-systems | 5. Distributed systems | reference | R4 | 2114 | 0 | 0 | 0 |
| appE:6-security | 6. Security | reference | R4 | 1582 | 0 | 0 | 0 |
| appE:7-innovation | 7. Innovation | reference | R4 | 962 | 0 | 0 | 0 |
| appE:8-evaluation | 8. Evaluation | reference | R4 | 1759 | 0 | 0 | 0 |
| appE:9-limitations-and-honesty | 9. Limitations and honesty | reference | R4 | 1529 | 0 | 0 | 0 |
| appF | Appendix F: Important source files and their responsibilities | reference | R2 | 142 | 0 | 0 | 0 |
| appF:1-entry-points-and-configuration | 1. Entry points and configuration | reference | R2 | 159 | 0 | 0 | 0 |
| appF:2-src-core-the-cache-engine | 2. src/core: the cache engine | reference | R2 | 128 | 0 | 0 | 0 |
| appF:3-src-api-http-surface | 3. src/api: HTTP surface | reference | R2 | 93 | 0 | 0 | 0 |
| appF:4-src-dist-cluster | 4. src/dist: cluster | reference | R2 | 176 | 0 | 0 | 0 |
| appF:5-src-security-authentication-and-trust | 5. src/security: authentication and trust | reference | R2 | 129 | 0 | 0 | 0 |
| appF:6-src-intel-telemetry-and-early-prototyp | 6. src/intel: telemetry and early prototypes | reference | R2 | 127 | 0 | 0 | 0 |
| appF:7-src-intelligence-the-intelligence-libr | 7. src/intelligence: the intelligence library | reference | R2 | 199 | 0 | 0 | 0 |
| appF:8-src-control-and-src-sim-legacy | 8. src/control and src/sim: legacy | reference | R2 | 102 | 0 | 0 | 0 |
| appF:9-src-lab-and-src-lab-live-phantom-lab | 9. src/lab and src/lab_live: Phantom Lab | reference | R2 | 394 | 0 | 0 | 0 |
| appF:10-tests-benchmarks-and-test-support | 10. Tests, benchmarks and test support | reference | R2 | 340 | 0 | 0 | 0 |
| appF:11-ui-scripts-and-packaging | 11. ui, scripts and packaging | reference | R2 | 301 | 0 | 0 | 0 |
| appF:12-key-documents-and-registers | 12. Key documents and registers | reference | R2 | 424 | 0 | 0 | 0 |
| appG | Appendix G: Implementation versus roadmap matrix | reference | R1 | 108 | 0 | 0 | 0 |
| appG:G.1 | G.1 Status vocabulary | reference | R1 | 206 | 0 | 0 | 0 |
| appG:G.2 | G.2 Pillar 1: Distributed systems engineering | reference | R1 | 445 | 0 | 0 | 0 |
| appG:G.3 | G.3 Pillar 2: Workload intelligence | reference | R1 | 182 | 0 | 0 | 0 |
| appG:G.4 | G.4 Pillar 3: Product excellence, observability and demonstrability | reference | R1 | 240 | 0 | 0 | 0 |
| appG:G.5 | G.5 Reconciled labels | reference | R1 | 253 | 0 | 0 | 0 |
| appH | Appendix H: The ten concepts you must be able to explain without help | learning |  | 79 | 0 | 0 | 0 |
| appH:H.1 | H.1 Cache hit, miss and origin load | learning |  | 179 | 0 | 0 | 0 |
| appH:H.2 | H.2 Stampede and single-flight coalescing | learning |  | 179 | 0 | 0 | 0 |
| appH:H.3 | H.3 Sharded locking | learning |  | 179 | 0 | 0 | 0 |
| appH:H.4 | H.4 TTL, staleness and the stale cap | learning |  | 179 | 0 | 0 | 0 |
| appH:H.5 | H.5 Generation and epoch fencing | learning |  | 177 | 0 | 0 | 0 |
| appH:H.6 | H.6 Invalidation and what a 204 means | learning |  | 157 | 0 | 0 | 0 |
| appH:H.7 | H.7 Rendezvous hashing and why GET is local-only | learning |  | 171 | 0 | 0 | 0 |
| appH:H.8 | H.8 The failure model, and what is NOT guaranteed | learning |  | 216 | 0 | 0 | 0 |
| appH:H.9 | H.9 The origin concurrency gate, and what the evidence shows | learning |  | 188 | 0 | 0 | 0 |
| appH:H.10 | H.10 Simulation versus real execution, and the M6 negative result | learning |  | 216 | 0 | 0 | 0 |
| appH:H.11 | H.11 Self-test: ten questions | learning |  | 250 | 0 | 0 | 0 |

## Judge boxes moved to Part II (R4)
- ch01:1.2#1: Why is caching necessary at all? Why not just make the database faster?
- ch01:1.2#2: Why not let the application query the database directly and keep the architecture simple?
- ch01:1.3#1: What does caching introduce that you did not have before? What are the costs?
- ch01:1.7#1: What is the actual intelligence in PhantomCache? Is it AI?
- ch01:1.11#1: Your tests pass. Does that mean the system is correct and ready?
- ch02:2.4#1: Walk me through what happens to one request, from the network to the response.
- ch02:2.5#1: What happens on a cache miss? And what if a thousand clients ask for the same missing key?
- ch02:2.6#1: What happens when the origin is unavailable?
- ch02:2.7#1: How do the nodes communicate?
- ch02:2.7#2: Which node serves a read for a key, and does rendezvous hashing decide that?
- ch02:2.8#1: Is the dashboard showing real data? And how does the Lab relate to the real system?
- ch02:2.9#1: Is this deployable in production today? What exists for deployment?
- ch03:3.2#2: Why not Go or C#? Go has goroutines, a good HTTP server, and is the language of many caches.
- ch03:3.3#1: Why Tokio, and what does it give you that threads would not?
- ch03:3.4#1: Why Axum rather than another framework, and why did you write your own HTTP client for the origin?
- ch03:3.5#2: What are the concurrency implications of your design? Where could it deadlock or stall?
- ch03:3.7#1: Why rustls and not OpenSSL for the mTLS between nodes? And why do you have both `ring` and `aws-lc-r
- ch03:3.11#1: Why React with plain CSS and no UI or chart libraries? Isn't that reinventing the wheel?
- ch03:3.12#1: You have hundreds of tests. Do they prove your concurrent cache is correct?
- ch04:4.2#1: Why partitioned mutexes? Why not one lock, a lock-free map, or a concurrent hash map crate?
- ch04:4.2#2: What about lock contention? How do you know it is not a bottleneck?
- ch04:4.4#1: What stops a cache stampede? What happens to the origin when a hot key expires?
- ch04:4.4#2: What happens with a hot-key overload? Say one key gets most of the traffic.
- ch04:4.6#1: How do you know the cache does not leak memory? Is the byte count the real memory use?
- ch04:4.7#1: Explain epochs. Why not just delete the keys? What does invalidation guarantee?
- ch04:4.8#1: How does expiry work? What stops the cache from serving very old data? Is the default TTL configurab
- ch04:4.9#2: State the time and space complexity of the cache operations.
- ch04:4.10#1: The origin goes down. What does the user see, and what protects the origin when it comes back?
- ch05:5.2#1: How do your nodes discover each other and communicate? What if I add a node?
- ch05:5.2#2: You have heartbeats. So does the cluster detect a dead node?
- ch05:5.3#1: You say you use weighted rendezvous hashing. Walk me through it. How does the weight enter, and what
- ch05:5.3#2: Walk me through a read. Which node serves it, and where does the hash come in?
- ch05:5.4#1: You mention epochs. What is an epoch here, and does it give you a global version of the cache?
- ch05:5.5#2: What if the invalidation message is lost or arrives late?
- ch05:5.6#1: Can two of your nodes serve different values for the same key at the same time?
- ch05:5.7#1: What happens if a node fails? What if the network partitions?
- ch05:5.8#1: Your node protocol has a GET request. Can a peer poison your cache with it?
- ch05:5.11#1: How accurate are your latency percentiles, and how do you compute them without slowing the request p
- ch05:5.12#1: How do you detect and handle hot keys?
- ch06:6.2#1: How do you stop an unauthorised user reading or deleting cached data?
- ch06:6.3#1: Your control endpoint changes cluster node state. Who can call it, and what stops a demo control fro
- ch06:6.5#1: What happens if someone sends a malicious request: a huge body, a slow body, a crafted path?
- ch06:6.6#1: How is tenant isolation enforced, and could one tenant ever see another's data?
- ch06:6.8#1: How do you prevent resource exhaustion and protect the origin from overload?
- ch06:6.8#2: Could a user make your gateway fetch an internal address (SSRF)?
- ch06:6.9#1: What happens if an invalidation is lost or delayed? Can two nodes serve different values?
- ch06:6.10#1: Is the system memory-safe, and how do you manage third-party risk?
- ch06:6.12#1: How did you measure performance, and why should I trust the numbers?
- ch06:6.12#2: Is your benchmark fair? Could you be fooling yourselves?
- ch06:6.14#1: What would you improve first, and what are your real bottlenecks?
- ch06:6.14#2: What is still unresolved? Give me the honest list.
- ch07:7.9#2: Why not just use Redis or Memcached?
- ch07:7.9#3: What is the actual intelligence in the system?
- ch07:7.9#4: What is your strongest technical contribution?
- ch07:7.9#6: Where are established products stronger than PhantomCache?
- ch07:7.9#7: How would I reproduce one of your experiments?
- ch07:7.9#8: What is the difference between your simulation and a real test?
- ch07:7.9#9: You report that your adaptive controller failed. Why should I trust anything else?
- ch07:7.9#10: What would you improve with more time?
- ch07:7.9#11: What does "H1 NOT_SUPPORTED" actually say, and what does it not say?
- ch08:8.13#3: A client sends DELETE and gets 204. Is that content guaranteed gone from every node?
- ch08:8.13#4: Is this linearizable? What is your consistency model?
- ch08:8.13#5: Can one tenant, or one cluster node, hurt another?
- ch08:8.13#6: The sealed evaluation says H1 is NOT_SUPPORTED. What does that mean for your "intelligent" caching c
- ch08:8.13#9: What would you do with more time?
- ch08:8.13#10: What have you chosen not to claim, and what is untested?
- ch08:8.13#11: Your repository is mostly uncommitted. Isn't that a process failure?
- ch08:8.13#12: The README and an older handoff say the system passed a formal acceptance. Why do your records say o
