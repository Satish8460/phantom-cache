# Demo Hub: every scenario, one click each

A development demo, not a product feature. It needs Node 24, `openssl` on the PATH (the hub says so on first load if it is missing), Chrome for the checks, and a built release gateway. Everything runs on one host over loopback.

## Start

Normally the start batch file starts the cluster, the Control Center and the hub; open `http://127.0.0.1:3201/`. By hand:

```
node scripts/demo_flow.mjs --exe <gateway.exe> --dir <work dir> --console [--ui-base http://localhost:3000]
```

`demo_flow` starts 3 gateways with mutual TLS, JWT required and the policy engine in Advisory mode (nothing is ever applied), mints the demo tokens (24 h, except the deliberately expired and wrong-audience ones) into `<dir>/tokens.json`, starts the real-content origin and serves the hub. The JWT secret and tokens stay in `<dir>` and are never printed or shown. If nothing is running when the hub opens, it offers a "Start the demo cluster" button.

## Using the hub

- Numbered cards, 1 to 30, in storyline order (speed and savings, safety, resilience, freshness and memory, protection, operability, intelligence, observability, performance). Each card has **Demonstrate**, a plain "what you will see", "what to look at" and "what this proves", a result panel (verdict, numbers labelled measured/derived, checks passed), **Show the log**, **Run again** and **Next scenario**. Groups have **Run all in this group**; card 30 and the top button run everything.
- The **live feed** (right side, above on a phone) narrates the real scenario as it runs: each step with a timestamp, what was done, what it means, the real request/response headers (collapsed), sampled responses for bulk traffic, live counters and an elapsed timer. **Stop**, **Pause queue** and **Copy the log** are there. Raw script output is in a collapsible panel.
- Every run starts with an automatic **prepare** step (cluster whole and Active, a stopped node restarted, counters snapshotted so only differences are shown, fresh keys) and ends with **clean-up**. A step that does not pass is retried once, and the feed says so. Scenarios that start their own gateway use ports 183xx/193xx and remove it again.
- A refusal that the scenario expects (401, 403, 400, blocked certificate) is shown as **Blocked as expected: this is the security working**. Attention colours appear only for an unexpected result.
- **Session summary** strip: runs, passed or showed, needs attention.
- **Acting as**: tenant A, tenant B, read-only, expired, wrong audience, no token (decoded claims only; the token string is never displayed). It resets to tenant A when a scenario starts.
- **Open the Control Center** per user and node: the page hands the token over by `postMessage` after the Control Center asks for it; no token is ever in an address.
- **Help** has the glossary and a plain statement of what is real (everything is measured on this machine; nothing is simulated).

## Scenario ids

tour, gallery, stampede, peerfill, offload, workloads, scale, auth, roles, isolation, targetsafety, mtls, failclosed, multiuser, invalidation, deadnode, deletestorm, burstdelete, peerexpiry, freshness, eviction, largeobjects, originprotect, operability, restart, intelligence, intelcontrol, dashboards, benchmark, everything. Scenario scripts accept `--events` (one `@EV {json}` line per step: action, observation, check, progress, summary) without changing their normal output or exit code.

## How to read the numbers

Release gateway, loopback, one host shared by gateways, origin and client, Windows. Latency is client-side and closed-loop. Repetition spread is shown; no confidence interval is claimed for small n. The content origin has a 25 ms service time (an origin parameter). Not a product benchmark; the full protocol is in `docs/research/benchmark_protocol.md`.

## Honest findings the demo shows

- Peer fill helps when the key's primary already holds the entry; a cold read on a non-primary costs an extra origin request. Objects above 1 MiB are never peer-filled.
- Purge delivery to other gateways is paced: 200 concurrent DELETEs were fully applied on a peer after about 7 s (first 0.3 s, median 3.7 s), with or without a dead third node (measured, one run each).
- Known security gaps in `security.md` stay open and are listed by the security cards.
- The Origin protection and Intelligence cards need a gateway build with the static origin cap and the policy-engine routes; on an older build they say so plainly and report Needs attention.
