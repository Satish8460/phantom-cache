# WP3 — Distributed Failure & Experimentation Plan (Revised)

## 1. Objective
Model a 3-node virtual cluster, implementing key partition routing via Weighted Rendezvous Hashing, a real inter-node replication pipeline (reproducing network congestion delays), and a state-machine driven failure recovery controller.

---

## 2. Node State Machine
A virtual node (`N1`, `N2`, `N3`) is governed by an explicit state machine:

```text
       ┌───────────┐
       │  JOINING  │
       └─────┬─────┘
             │ (Sync complete)
             ▼
       ┌───────────┐         (Congested)        ┌────────────┐
       │  ACTIVE   ├───────────────────────────►│  DEGRADED  │
       └─────┬─────┘                            └─────┬──────┘
             │ (Heartbeat lost)                       │ (Lost)
             ▼                                        ▼
       ┌───────────┐                            ┌────────────┐
       │  SUSPECT  ├───────────────────────────►│   FAILED   │
       └─────┬─────┘                            └─────┬──────┘
             │ (Rerouted recovery)                    │ (Restarted)
             ▼                                        ▼
       ┌───────────┐                            ┌────────────┐
       │ RECOVERING│◄───────────────────────────┤  OFFLINE   │
       └─────┬─────┘                            └────────────┘
             │ (Replicas rebuilt)
             ▼
       ┌───────────┐
       │  ACTIVE   │
       └───────────┘
```

* **Routing Impact:** Failed or suspect nodes are immediately excluded from hash weight computations by the Rendezvous Router. Degraded nodes receive reads but are excluded from primary write paths.
* **Replication Impact:** Rejoining nodes transition to `RECOVERING` and fetch missing replica keys via version-epoch sync logs before returning to `ACTIVE`.

---

## 3. Weighted Rendezvous Hashing (HRW)
Nodes partition keys using deterministic Rendezvous hashing:

$$\text{Weight}(K, N_i) = \text{Hash}(K \mathbin{\Vert} N_i) \times \text{CapacityWeight}_i$$

* **Primary Key:** Node yielding the highest weight.
* **Replica keys:** Nodes yielding the 2nd and 3rd highest weights.
* **Epoch-aware Consensus:** Epoch number updates are tracked globally, preventing stale partition splits.

---

## 4. Real Replication Pipeline
Replication operates via simulated asynchronous queues subject to network delay and drop models:

```text
[Node 1: Write] ──► [Replication Message Queue] ──► [Network Delay Jitter] ──► [Node 2: Version Validation]
```
If network packet congestion is high, packet delays rise, preventing version synchronization and causing stale replica keys to be discarded on receiver validation.

---

## 5. Traceable Event Composability
The Event Scheduler compiles multiple overlapping environmental variables without hardcoded logic:
* `FLASH_CROWD` increases Traffic rates and skews Zipfian skews.
* `NETWORK_DEGRADATION` applies delays to the network delay queues.
* `NODE_FAILURE` transitions node states to `FAILED`.
* `BACKEND_SLOWDOWN` decreases database IOPS ceilings.

---

## 6. Implementation Files
* `src/dist/node.rs`
* `src/dist/router.rs`
* `src/sim/events.rs`

## 7. Completion Criteria
* Virtual Node state machines implemented.
* Weighted Rendezvous Hash routing algorithm implemented and tested.
* Composable Event structures defined.
* All unit tests passed.

## Status
* **COMPLETED**
