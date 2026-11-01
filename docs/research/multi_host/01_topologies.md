# Multi-host validation: topologies and devices

**Status:** Draft for owner review. **Audience:** product owner (device supply), operator. **Last reviewed:** 2026-10-30 18:18 (+05:30, machine clock).

Tags: FACT (file:line), PROPOSAL, UNVERIFIED. I invented no hardware; every device slot is blank for the owner.

---

## 1. Roles in a topology

Roles follow `docs/demo-lab/device-topologies.md` (Requester, Traffic Generator, PhantomCache Node, Origin, Observer, Peer Node), which is a 2026-10-26 planning record with known stale points (its "hardcoded loopback" statements are out of date, `PHANTOM_BIND_ADDR` is configurable, FACT `src/config.rs:98`). For this study:

| Role | What runs | Notes |
|---|---|---|
| **Node (Nk)** | `phantom-cache-demo` gateway, release build, cluster mTLS | One per host. Node ids are 1 to 255 (FACT `config.rs:149`). |
| **Origin (O)** | A versioned HTTP origin that returns a version in the body and `Cache-Control: max-age=...`, and can be mutated on command | A standalone one does not exist yet (the test one is `VersionedMockOrigin` in `tests/support/cluster_harness.rs`, test code). To be written (`04`, section 6). |
| **Client / observer (C)** | Load generator and propagation prober, single clock for method P1 | To be written. |
| **Controller (X)** | Orchestration: SSH/WinRM, fault injection commands, evidence collection | Can be the same machine as C. |

Keep O and C off the node hosts in the physical topologies so that origin and client load do not share CPU with a node (PROPOSAL). In the minimum topology that costs extra hosts; see the options.

## 2. Option A: minimum viable physical topology (PROPOSAL)

**3 node hosts + 1 combined client/origin/controller host, one switch.**

```
            isolated switch / VLAN (no uplink)
   +--------+--------+--------+--------+
   |        |        |        |        |
  N1       N2       N3     C+O+X     (optional: management NIC, separate)
```

- 3 identical (or documented) hosts for N1 to N3. Three is the smallest cluster in which every DELETE has an issuer, a primary and at least one third member (the M3 scenario uses exactly issuer G, victim B, primary P).
- The fourth host runs the client, the origin and the controller. Honest limit: the origin and the client then share a host, so E11 (origin gate) concurrency is measured on the origin process itself, not on a separate origin machine; document it.
- If only 3 physical machines exist: run C+O+X on N1's host is **not** recommended (it perturbs the node under test); use a laptop as the fourth device or reduce to the "2.1 variant" below.

**Variant A-3 (three devices total, degraded).** N1, N2, N3 are hosts; the client/origin/controller run on N1. Allowed only as a clearly labelled reduced study: E1 to E3, E4 to E7 remain meaningful; E11 and the single-clock method P1 are compromised (the client shares N1's clock and CPU). Mark every result `PHYSICAL_LAN (client co-located on N1)`.

**What A supports:** E1, E2, E3 (all peer pairs), E4 (kill one node), E5 and E6 (partition one node from two), E7, E8, E10, E11 (N=3). E9 on a node that can have its clock changed (a dedicated VM or sacrificial host).

**What A cannot support:** quorum-like behaviour with a 2+2 split (needs 4 or 5 nodes); behaviour of the rendezvous replica set beyond primary plus 2 (FACT ledger: replicas limited to primary+2); scale statements; asymmetric topology with more than one unreachable peer; a separate origin machine (unless a fifth device is added).

## 3. Option B: extended topology (5 nodes)

**5 node hosts + 1 client + 1 origin (+ controller on the client), one switch.** PROPOSAL.

- Needed for: 2-versus-3 partitions, two simultaneous node failures, a 5-way pre-warm and connection-pool measurement (the ledger notes a "5-node measured test" does not exist, FACT claims_ledger.md C6 amendment end), replica sets larger than the minimum.
- The harness already supports 5 nodes on loopback (`ClusterHarness::start_with_options` takes any list of ids, `cluster_harness.rs` around line 379).
- Uplinks: one switch port each. A single switch is a single point of failure and a shared fabric; this is acceptable and must be stated. A second switch (to test an inter-switch link fault) is out of scope.

## 4. Option C: non-physical intermediate step (clearly NOT multi-host evidence)

Use only to debug the scripts and to get calibration numbers before touching physical devices. Every result carries tier label `WSL2` or `VM_SINGLE_HOST`.

| Variant | What it adds over loopback | What it does not add | Notes |
|---|---|---|---|
| **C1. Several WSL2 distros / instances** on one Windows machine | A Linux userland, `tc`/`iptables` (kernel support in the WSL2 kernel is UNVERIFIED: netem and some netfilter modules may be missing), a virtual NIC with NAT or mirrored networking | Independent clocks (one hypervisor clock), independent hardware, a real switch, real NIC offloads | WSL2 distros share one VM and one kernel unless separate WSL2 VMs are used; many distros therefore share a network namespace boundary only per distro. UNVERIFIED whether each distro gets its own IP. Windows-to-WSL2 forwarding has its own NAT behaviour and latency. |
| **C2. Hyper-V (or other) VMs** with an internal/private virtual switch | Separate kernels, separate IPs, independently settable guest clocks (E9 is safe here), a virtual switch where traffic can be impaired on the host, VM power-off for crash tests | Real NICs and physical latency; VM disks buffer writes differently from physical disks | Windows 11 Home (this machine's edition per the environment: "Windows 11 Home Single Language") does **not** include Hyper-V by default (UNVERIFIED for this machine; Hyper-V is a Pro/Enterprise feature, and enabling features or installing another hypervisor needs owner approval). |
| **C3. Loopback on aliases** (several loopback IPs, for example 127.0.0.2 to 127.0.0.4) | Distinct addresses for the cluster listener | Everything physical | Windows binds all of 127.0.0.0/8 by default (UNVERIFIED for the gateway's SSRF and loopback rules); Linux does. |
| **C0. Pure loopback** (the existing harness) | n/a | n/a | This is the existing evidence. Use it as the rehearsal and as the comparison baseline. |

**Rules for option C:** label the tier on every table; do not average with physical data; do not write "multi-host" in any claim from these runs; state the host's CPU count and whether the host was otherwise idle (the project's Scenario B rule forbids browsers and other heavy load during a run).

## 5. Network isolation (all options)

- A dedicated switch or a port-isolated VLAN with **no default route** and no connection to the owner's normal LAN is required for physical runs. If a unmanaged switch is used, do not connect its uplink.
- Use a private static IPv4 plan, for example 10.77.0.0/24 (PROPOSAL; any private range not used elsewhere): N1 10.77.0.11, N2 .12, N3 .13, ... C .50, O .60. Fix addresses by static configuration so that the membership file is stable.
- Disable power-saving, sleep and Wi-Fi on test hosts; record the NIC speed/duplex and MTU in the manifest.
- Time source: one host runs the time server (chrony on Linux or `w32time` on Windows) with the others syncing to it, or all sync to a LAN-local PTP/NTP reference. The achieved offset is recorded before and after each run (`04`).
- Management access on a second NIC or out-of-band so that fault rules cannot lock you out.

## 6. What each topology can support (summary)

| Experiment | A (3+1) | A-3 | B (5+2) | C1 WSL2 | C2 VMs |
|---|---|---|---|---|---|
| E1 to E3 propagation distribution across hosts | yes | partly (client co-located) | yes | rehearsal only | rehearsal only |
| E4 node failure | yes | yes | yes (incl. two failures) | rehearsal | yes (VM stop) |
| E5 symmetric partition 1 vs 2 | yes | yes | yes (also 2 vs 3) | rehearsal | yes |
| E6 asymmetric partition | yes | yes | yes | rehearsal | yes |
| E7 loss/delay | yes | yes | yes | rehearsal | yes |
| E8 issuer crash (G) | yes | yes | yes | rehearsal | yes |
| E8m M3 incl. power loss | process kill: yes; power cut: yes (on a host that tolerates it) | same | same | process kill only | VM power-off is not a disk-faithful power loss |
| E9 clock skew | on a sacrificial node | same | same | no (single clock) | yes |
| E10 mTLS identity/rotation/revocation | yes | yes | yes | rehearsal | yes |
| E11 origin gate | yes (client/origin share a host) | no | yes (own origin host) | rehearsal | rehearsal |

"Rehearsal" means the script can be debugged but no claim may be drawn from the result.

## 7. Device requirements (owner fills in the blanks)

Rules for the list: hosts should be as identical as practicable (PROPOSAL) so that a node-specific effect is not confused with a hardware effect; if they differ, record each and treat the OS/hardware mix as a factor (`05`, D2).

### 7.1 Nodes

| Slot | Role | OS and version | CPU (model, cores) | RAM | NIC (speed, chipset, driver) | Disk (type) | Admin rights? | Owner supplies (Y/N) |
|---|---|---|---|---|---|---|---|---|
| N1 | node 1 | ________ | ________ | ____ | ________ | ________ | ____ | ____ |
| N2 | node 2 | ________ | ________ | ____ | ________ | ________ | ____ | ____ |
| N3 | node 3 | ________ | ________ | ____ | ________ | ________ | ____ | ____ |
| N4 (extended) | node 4 | ________ | ________ | ____ | ________ | ________ | ____ | ____ |
| N5 (extended) | node 5 | ________ | ________ | ____ | ________ | ________ | ____ | ____ |

### 7.2 Support devices

| Slot | Role | OS | CPU | RAM | NIC | Notes | Supplied? |
|---|---|---|---|---|---|---|---|
| C | client / observer / controller | ________ | ________ | ____ | ________ | single-clock probe host | ____ |
| O | origin | ________ | ________ | ____ | ________ | own host in option B | ____ |
| SW | switch | model ________ | ports ____ | speed ____ | managed? ____ | VLAN capability for isolation | ____ |
| Cables | | type ________ | count ____ | | | | ____ |
| Console | out-of-band access for lockout recovery | ________ | | | | | ____ |

### 7.3 Software prerequisites on each host (all already needed to run the binary unless noted)

- The release binary `phantom-cache-demo` built from a recorded commit (`03`, section 2). Windows x86-64 is the only platform ever exercised (FACT limitations.md); building and running on Linux is **UNVERIFIED** (no Linux run of the cluster tests is recorded). If the OS mix includes Linux, add a Linux build-and-pass step to the pre-study checklist and accept that Linux behaviour may differ.
- A way to copy files and run commands remotely (OpenSSH or WinRM). UNVERIFIED which is available; an install request if neither exists.
- A time-sync daemon (built into Windows and most Linux).
- Optional tools requiring approval: see `02`, section 8.

### 7.4 Minimum counts

| Option | Hosts needed | Switch ports |
|---|---|---|
| A | 4 (3 nodes + C/O/X) | 4 + management |
| A-3 | 3 | 3 |
| B | 7 (5 nodes + C + O) | 7 + management |

## 8. Pre-study device checklist (to be signed by the operator)

- [ ] Hosts reachable on the isolated segment; no route to other networks (`ping` to an outside address fails).
- [ ] Static IPs match the plan; firewalls on each host allow only the study ports between the study hosts (HTTP ingress port, cluster listener port, origin port, SSH/WinRM from the controller).
- [ ] NIC speed/duplex/MTU recorded; no packet errors on a 60 s ping test.
- [ ] Clock offsets recorded; sync daemon running.
- [ ] Binary hash identical on every node.
- [ ] A keyboard/console path exists for every host (or a watchdog that restores network rules).
