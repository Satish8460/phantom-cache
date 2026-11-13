# PhantomCache demo: a narrated 10-minute walkthrough

For the person presenting. Everything here is what the Demo Hub actually does; every number was measured on one Windows machine (release build, loopback, three gateways and the origin on the same computer) in the runs listed in `docs/lanes/`. Your numbers will differ a little from run to run. Say that once, early: "these are measured here, on this laptop, not promised".

## 1. Start (1 minute)

1. Double-click `Start-Phantom-Cluster-Demo.bat` in the project folder. Leave its window open. After about 30 seconds the Demo Hub opens in your browser at `http://127.0.0.1:3201`. If it does not open, type that address yourself.
2. What you see: a header with three green pills ("node 1: Active", "node 2", "node 3"), "Acting as Tenant A (alice)", and an "Origin requests" counter. Below it, "Start here", and the list of 30 cards. The right side is the live feed.
3. Say: "PhantomCache is a caching gateway. It sits in front of a slow server, called the origin, and remembers answers, so the second person asking is answered fast and the slow server is bothered less. Everything you will see is real: real pictures, real requests, three real gateways on this machine. Nothing is simulated."
4. How every card works: press the blue **Demonstrate** button. The live feed shows each step as it happens, with a plain sentence under it saying what it means. At the end a badge appears: **Passed**, **Shows ...** (it shows a behaviour rather than a pass/fail), or **Needs attention**. The numbers carry a tag (measured, derived, unavailable). **Run again** repeats it; **Next scenario** jumps to the next card; **Stop** ends a run; **Copy the log** copies the feed.
5. To stop everything at the end: press Ctrl+C once in the batch window. If it asks "Terminate batch job", answer N.

## 2. Three routes

**3-minute route (the headline).** Cards 2, 3, 15, 16. Say the one-line summary of each, point at one number each, finish.

**10-minute route (the recommended talk).** Cards 2, 3, 5, 8, 10, 15, 16, 20, 21, then 29 (or 28 if the Control Center is open). About 2 to 3 minutes of runs and the rest is talking.

**Full route.** Press **Demonstrate everything** at the top and talk over the feed while it runs. It runs the whole list in order (it skips the tour and the two-window card, which need your clicks) and ends with the session summary strip. Measured: about 3.5 minutes of running for 27 scenarios. Then use the sections below as your speaking notes in order.

The "5-minute guided tour" button is an alternative opening: Next and Back (or the arrow keys) step through eight short explanations and runs some scenarios for you. If one step shows an error text instead of a result, press **Next** and carry on; you can run that card yourself from the list (for the safety step, use card 8, "Who may come in").

## 3. The thirty scenarios in storyline order

For each: what to say, what to click, what you should see, what number to point at. "Origin" means the slow server. A MISS means the gateway had to ask the origin; a HIT means it answered from memory; "peer" means a neighbouring gateway answered; "coalesced" means the request waited for another identical request already on its way.

### Start

**1. Guided tour (5 minutes).** Say: "If you only have five minutes, this walks you through the main story with an explanation after each step." Click **Take the 5-minute guided tour** (or card 1's Demonstrate). Expect an eight-step window; "Run this step" runs the real scenario and prints a verdict and two sentences of meaning. Point at: nothing in particular; read the "after" sentence aloud.

### Speed and savings

**2. Cold vs warm gallery.** Say: "Twelve real pictures are loaded twice. First time the gateway has to ask the origin for each. Second time it should not." Expect: Passed in about 4 seconds; the first twelve lines say MISS, source origin (the first tile took 322 ms in one run), then HIT, source local, a few milliseconds each; every line ends "sha256 OK". Point at: "origin requests counted by the origin process during this scenario: 12" for 24 requests, and "integrity failures: 0" (every picture was compared byte for byte with what the origin sent).

**3. Stampede: 300 at once.** Say: "Three hundred people ask for the same new picture at the same moment. What does the origin see?" Expect: Passed in about 5 seconds. Point at: "the origin was asked 1 time; the 300 answers came from coalesced 280, local 19, origin 1" (one run), and, spread over three gateways, "the origin was asked 3 times" (one per gateway). The crowd waited for the first answer instead of hitting the origin 300 times.

**4. Peer fill: a neighbour answers.** Say: "A gateway that has never seen a picture can ask its neighbour instead of the origin." Expect: "Shows ..." in about 5 seconds. Point at: in the first check, 24 pictures read on three gateways cost 24 origin requests, and the other 48 reads were filled from a peer. Be honest about the second check: when the reads happen in a fixed order (node 1, 2, 3) the same 72 reads cost 32 origin requests, so the saving depends on which gateway sees a picture first. The 1.17 MiB picture is not shared this way: three reads cost three origin requests.

**5. Origin offload.** Say: "3,000 client requests. How many does the origin itself count?" Expect: Passed in about 4 seconds. Point at: requests sent 3,000 against the origin's own count (502 in one run), "errors 0", "integrity failures 0".

**6. Workload shapes.** Say: "The gateway helps most when keys repeat, and it is honest when they do not." Expect: "Shows ..." in about 4 seconds. Point at the hit ratio per shape: hot keys 98.0%, popularity mix 77.2%, a scan of all-different keys 5.0%, a burst of 100 different keys 0.0%. A scan and a burst of new keys cannot be helped by a cache, and the card says so.

**7. Real requests at scale.** Say: "Now the numbers people ask for: 1,000, then 5,000, then 10,000 requests over 315 real objects, from a Node client." Expect: Passed in about 15 to 20 seconds. Point at: "requests sent 16,000", "errors 0", "integrity failures 0", "bytes transferred" about 226 MB. For throughput context only: in an earlier run of the same scenario, 10,000 requests at concurrency 500 ran at about 6,000 to 6,500 requests per second with all answers correct (one machine, three runs).

### Safety

**8. Who may come in.** Say: "Every request must carry a valid, current token addressed to this gateway." Expect: Passed in under 2 seconds; each line says "Blocked as expected" in green. Point at the six checks: no token, expired token, wrong audience, forged token, garbage token, and a valid token with no customer (403). A red-looking refusal here is the security working.

**9. Roles: read-only vs invalidate.** Say: "What you may do depends on your role." Expect: Passed. Point at: the read-only token reads (200) but its DELETE gets 403, and the next read of that key is still a HIT: the refused delete changed nothing.

**10. Tenant isolation.** Say: "Two customers can ask for the same address and never see each other's cached copy." Expect: Passed. Point at: tenant A gets MISS then HIT; tenant B gets its own MISS and the origin count goes up by one. B on another gateway is not served A's entry either.

**11. Unsafe addresses.** Say: "Malformed addresses are stopped at the gateway." Expect: Passed. Point at: line breaks, a NUL byte, a space and a dot-segment path each get 400, and the origin count did not move (+0).

**12. Cluster certificates (mTLS).** Say: "The gateways talk to each other over mutual TLS: only known members are answered." Expect: Passed in about 3 seconds. Point at: one real member certificate is answered, four kinds of intruder certificate (none, not a member, member name with a different key, a foreign authority) get no answer. The lines contain technical error codes; say "that is the handshake being refused".

**13. Fail closed at start-up.** Say: "A gateway that requires login, or listens on the network, refuses to start without a secret." Expect: Passed. Point at: "exit 2" and the single "[CONFIG ERROR] Fail-Closed" line.

**14. Two people, two windows.** Say: "Two Control Center windows, two identities, one set of gateways." Click Demonstrate and allow pop-ups. Expect: two windows open (tenant A and tenant B); each receives its own token by a one-time message, never in the address bar. Requires the Control Center to be running (the batch file starts it). If a window stays empty, allow pop-ups for this page and press **Run again**. Point at: "windows signed in by message: 2".

### Resilience

**15. Delete a key everywhere.** Say: "A picture is cached on all three gateways. We delete it through one." Expect: Passed in about 3 seconds. Point at: the three first reads (MISS origin, then MISS source peer twice), "all HIT", "DELETE answered 204", then "node 2 stopped serving the entry after 40 ms" and node 3 after 43 ms (another run: 56 and 58 ms). Say clearly: "That is one measured run on this machine, not a guarantee."

**16. Kill a node while traffic runs.** Say: "A gateway crashes in the middle of traffic." Expect: Passed in 7 to 12 seconds. Point at: 6,000 requests, errors 0, integrity failures 0, "25 requests retried on another node by the client", node 1's view showing node 3 as Recovering, then the restart and "every member Active". The "Recovering/unreachable" words are the cluster noticing; they are expected here. The last counters line can look odd after a restart; ignore it.

**17. Delete storm with a dead peer.** Say: "400 deletes at once while a gateway is down." Expect: "Shows ..." after about 30 seconds (a quiet wait while the feed says it is watching node 2). Point at: "400 DELETEs answered in 446 ms, all 204" and "node 2 stopped serving all 260 deleted keys: first after 36 ms, median 4,171 ms, last after 8,616 ms". Say: "Deletes are paced, so a very large storm takes seconds to reach everyone, and we show that rather than hide it."

**18. Burst of deletes.** Say: "A purge storm must not leave a gateway stuck." Expect: Passed in about 20 seconds. Point at: "200 concurrent DELETEs, all acknowledged, every member Active afterwards" (all 204; last key cleared after about 6.8 s) and "a purged key is not served stale by another gateway". Do not read the "requests sent: 0" row; it counts reads, and this scenario sends deletes.

**19. Neighbour copies expire in time.** Say: "A copy fetched from a neighbour must not outlive its source." Expect: Passed after about 13 seconds (it waits 10 seconds on purpose). Point at: an object with an 8-second lifetime is a MISS everywhere after 10 seconds, never a plain HIT.

### Freshness and memory

**20. Freshness suite.** Say: "Content is kept as long as, and no longer than, the origin allows." Expect: Passed in about 13 seconds with 63 checks; each line names the rule (stale-while-revalidate, stale-if-error, no-store and private not cached, ETag revalidation giving a 304, the staleness cap). Point at: "stale_cap_violations is 0" and one 304 line.

**21. Memory budget and eviction.** Say: "A gateway with an 8 MiB memory budget is flooded with pictures." Expect: Passed in about 17 seconds, 27 checks. Point at: resident bytes peaked at 8,140,015 of an 8,388,608-byte budget (never above), 239 evictions, and the hot pictures still HIT.

**22. Large objects (the 1 MiB rule).** Say: "One huge file must not push everything else out." Expect: Passed in 2 to 3 seconds. Point at the three pictures: 0.49 MiB is kept (second read HIT), 1.27 MiB is served but not kept (second read is a MISS again), 2.54 MiB is refused with a clean 502. The 502 is the designed answer here, not a fault.

### Protection

**23. Origin protection (static cap).** Say: "A fragile origin can be protected: the gateway limits how many requests reach it at the same time." Expect: the card starts its own gateway with a cap of 6 and an origin that takes 400 ms per request, sends 60 simultaneous cold keys, and checks the origin's own peak against the cap. Point at: "the origin's own peak of simultaneous requests" against "cap 6", and the line "503 answers are the cap doing its job". This card needs the newer gateway program built by the start file; if it says the build does not include the cap, see section 4.

### Operability

**24. Fail fast, health, abuse.** Say: "Bad configuration is refused at start-up with one clear line per problem." Expect: Passed in about 14 seconds, 31 checks. Point at: four independent configuration mistakes give exit 2 and four `[CONFIG ERROR]` lines; abusive requests get 4xx, never 5xx.

**25. Kill and restart under load.** Say: "A gateway is killed and restarted while a client keeps asking." Expect: Passed in about 6 seconds. Point at: "ready 140 ms after start", 381 requests after recovery with 0 problems, and "no wrong bytes were ever served". The outage itself is visible as clean "connection refused" failures; that is the point.

### Intelligence

**26. Policy engine (observe and advise).** Say: "The policy engine watches real traffic and suggests what it would do. It never changes the cache." Expect: "Shows ..." with the registry, live signals and decision records. Point at: "applied: none". Only the modes Off, Observe and Advisory exist in this demo.

**27. Policy engine: negative control.** Say: "A steady workload should produce no advice; a changed workload should." Expect: "Shows ..." after up to a minute: no recommendation first, a record after the change. Point at: the first phase says no action; the second phase has a record.

### Observability

**28. Live dashboards.** Say: "While traffic runs, the Control Center screens move." Open the Control Center (button on the page, or `http://localhost:3000`, sign in using the token hand-off) before or right after pressing Demonstrate. Expect: "Shows ..." in about 6 to 8 seconds in the current build (the text says "about 20 seconds"). Point at: "sent 6000, gateways counted 6000". If the run ends before you open a screen, press **Run again** with the Control Center already open on the Overview screen.

### Performance

**29. Quick benchmark.** Say: "The same real requests straight to the origin, through one gateway, and through three; once with an instant origin and once with a slow one." Expect: "Shows ..." in about half a minute with three repetitions. Point at the ratios against "direct", and say what the full protocol found (10 repetitions, 95% confidence intervals, release build, this machine): with a zero-latency origin the gateway is slower than going direct (single gateway throughput ratio 0.85 plus or minus 0.03 at concurrency 200); with a 100 ms origin it is faster everywhere (single 4.79 plus or minus 0.12 at concurrency 200, p50 latency between 0.05 and 0.23 of direct). The honest message: a cache pays off when the origin is slow.

### All

**30. Demonstrate everything.** Say: "The whole story in order." Click the green button at the top. Expect: about 3.5 minutes for 27 scenarios (the tour and the two-window card are skipped), then the **Session summary** strip: how many runs, how many passed or showed, how many need attention, total measured run time. Point at: "0 need attention" after a clean build (older history is counted too; press **Reset demo** for a clean count if the strip shows old results).

## 4. If a step looks wrong

| What you see | What to click |
|---|---|
| A card says **Needs attention** | Click **Show the log**, read the red line (it says what failed), then **Run again** once. If it fails twice, press **Next scenario** and say "that one is not working on this machine, let me show the next". |
| A card says the build "does not include" the policy engine or the origin cap | That gateway program is older than the page. Skip cards 23, 26 and 27 and say so. |
| The header shows a node as **Stopped** | Just press **Demonstrate** on any card that needs the cluster: it restarts the stopped gateway first and says so in the feed. Or click the Restart link in the banner. |
| Something is stuck or a run is taking too long | Press **Stop** under the feed. The cluster is put back together by itself. |
| Several things are odd | Press **Reset demo**, confirm, wait about 20 seconds until three green pills are back. Counters start from zero. |
| A pop-up window did not open (card 14) | Allow pop-ups for the hub page and press **Run again**. |
| The page shows nothing at phone width | Scroll to the top: the live feed sits above the scenario list on narrow screens. |
| You need a number from the run | Click **Copy the log** and paste it somewhere. |

## 5. Twelve likely questions

1. **Is this a real cache or a mock-up?** Real. The gateways are the real program, the origin is a real HTTP server on this machine serving real pictures and files, and every response is compared byte for byte (sha256) with what the origin sent. The page does not play back anything.
2. **Does it make things faster?** When the origin is slow, yes: with a 100 ms origin the full protocol measured about 4.8 times the throughput of going direct (single gateway, concurrency 200, 10 repetitions). With an instant origin it is slower than direct, about 0.85 times. A cache pays off when the origin is the bottleneck.
3. **What does it do when the origin goes down?** Entries that are allowed to be served stale are served stale for a bounded time (card 20 shows the stale-if-error cases and the cap). A key that was never cached gets a clean error from the gateway; in a separate check this took about 2 seconds on this machine.
4. **If I delete something, is it gone everywhere?** On the gateway that took the delete, the next read goes to the origin. The other gateways stopped serving it after tens of milliseconds in our runs (40 and 43 ms in one run; seconds for a storm of hundreds of deletes). Those are measurements on one machine, not a guarantee, and the project does not claim a cross-gateway time bound.
5. **Can one customer see another customer's data?** In the tests, no: the cache is kept per customer (tenant), so tenant B asking for the same address gets its own MISS (card 10).
6. **How are gateways kept from being impersonated?** The gateways talk to each other over mutual TLS with certificates; card 12 shows a member answered and four kinds of intruder refused.
7. **What happens with huge files?** Entries above one eighth of the memory budget are served but not kept; bodies above 2 MiB are refused with a 502 (card 22).
8. **Can it run out of memory?** In the eviction test the resident size stayed at or below the configured budget the whole time, with old entries evicted and hot ones kept (card 21).
9. **What is the policy engine? Does it change my cache?** No. It observes real traffic and gives advice with an explanation, and every record says "applied: none". Only Off, Observe and Advisory exist here.
10. **How many requests can it handle?** In the demo's scale card, 16,000 requests over three gateways with zero errors and zero damaged files; an earlier run reached about 6,000 to 6,500 requests per second at concurrency 500. All client, gateways and origin share one computer, so treat it as a demonstration, not a capacity figure.
11. **Is this production ready? Does it run on several machines?** No claim of that. This is a lab demonstration: three gateways on one machine, loopback network. Behaviour across separate machines and the open findings are listed in the limitations document (`docs/user/limitations.md`).
12. **Why is the hit ratio only 5% in one of the workloads?** Because that workload asks for all-different keys, a scan. A cache cannot help with requests it has never seen; card 6 shows this on purpose so the good numbers are not cherry-picked.
