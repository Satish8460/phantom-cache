# Multi-host validation: fault injection

**Status:** Draft for owner review. **Audience:** operator, product owner (install approvals). **Last reviewed:** 2026-10-30 18:18 (+05:30, machine clock).

Tags: FACT (file:line), PROPOSAL, UNVERIFIED. Nothing here has been executed. Every command is UNVERIFIED unless a tag says otherwise. Replace `<...>` placeholders. **No tool in this file has been installed or run by me; every install is a request in section 8.**

---

## 1. Principles

1. Inject faults on the **node hosts** (or on a programmable switch), on the inter-node cluster listener traffic only. Do not touch the management NIC, the SSH/WinRM path, or client-to-node ingress unless the experiment says so.
2. Every fault has: a command to apply, a command to **verify** it is active, a command to remove, and a self-expiring watchdog (section 6).
3. Faults are applied in one direction at a time so that asymmetry is explicit: "from X to Y" means packets sent by X addressed to Y's cluster port are affected.
4. Record the exact command, the host, the start and end times (controller clock and node clock) and the verification output in the run log (`04`).
5. The cluster port is the port in `PHANTOM_CLUSTER_LISTEN_ADDR` (FACT `src/config.rs:150` to `:151`). In the commands below it is `$CP` (for example 9443). Ingress HTTP is a different port and is not impaired unless stated.

## 2. Linux hosts (the best-supported path)

Requires root. Kernel modules `sch_netem`, `iptables`/`nft` must exist (UNVERIFIED for any specific host; check with `modinfo sch_netem` and `tc qdisc help`).

### 2.1 Delay and loss (netem) on the egress of one node

Impair only packets to the cluster port of selected peers, so the rest of the traffic is untouched. A `prio` qdisc with a `u32` filter does this.

```sh
IF=eth0                      # data NIC, never the management NIC
CP=9443                      # cluster listener port
PEER=10.77.0.12              # destination peer (repeat for more peers)

# apply
tc qdisc add dev $IF root handle 1: prio
tc qdisc add dev $IF parent 1:3 handle 30: netem delay 20ms 5ms distribution normal loss 5%
tc filter add dev $IF protocol ip parent 1:0 prio 3 u32 \
    match ip dst $PEER/32 match ip dport $CP 0xffff flowid 1:3

# verify
tc -s qdisc show dev $IF
tc filter show dev $IF

# remove
tc qdisc del dev $IF root
```

Variants: `delay 100ms` fixed; `loss 1%/5%/10%/20%` (C6's stated failure model F is up to 20 percent loss, FACT claims_ledger.md C6); `loss 10% 25%` correlated loss; `reorder 25% 50%` with delay; `duplicate 1%`; `corrupt 0.1%` (corruption should be caught by TLS; this tests the error path). Netem adds delay to each packet, not to flows; it affects both data and ACK segments that pass the egress filter. Because TCP retransmits, "loss" shows up as extra latency, not as lost invalidations; say so in the report.

### 2.2 Partition (drop) with iptables or nftables

```sh
# symmetric partition between this node and PEER: run on BOTH hosts
iptables -I INPUT  -s $PEER -p tcp --dport $CP -j DROP
iptables -I OUTPUT -d $PEER -p tcp --dport $CP -j DROP

# asymmetric: this node cannot SEND to PEER, but can receive from it
iptables -I OUTPUT -d $PEER -p tcp --dport $CP -j DROP        # run only here

# one-way where packets in one direction vanish at the receiver
iptables -I INPUT  -s $PEER -p tcp --dport $CP -j DROP        # run only on the receiver

# verify
iptables -L -n -v | grep $CP
# remove (exact rule match)
iptables -D OUTPUT -d $PEER -p tcp --dport $CP -j DROP
```

Notes. (a) `DROP` makes connections hang until timeout (a "blackhole"); `REJECT --reject-with tcp-reset` makes them fail fast (an "explicit refusal"). The two behave differently in the gateway (timeout versus error); run both. (b) Established connections: a new rule drops new packets of existing connections too (the pool may hold open mTLS connections); to emulate "all new connections refused, existing continue" use `-m conntrack --ctstate NEW`. (c) The gateway's per-attempt cluster timeout is 150 ms by default (FACT `config.rs:159`, `PHANTOM_CLUSTER_TIMEOUT_MS`), so a blackhole is perceived at that scale.

### 2.3 Process faults (Linux)

| Fault | Command | What it models |
|---|---|---|
| Crash (SIGKILL) | `kill -9 <pid>` | hard crash with no handlers; the M3 reproduction used TerminateProcess on Windows as the closest equivalent (FACT m3_durability/reproduction_report.md section 1) |
| Graceful stop | `kill -TERM <pid>` | the gateway's handling of SIGTERM is UNVERIFIED on Linux (the code path on Windows is `ctrl_c`/`ctrl_break`; shutdown does not drain pending invalidations, FACT limitations.md "Shutdown", and `src/main.rs:509` to `:514` as cited there) |
| Pause (frozen process) | `kill -STOP <pid>` ... `kill -CONT <pid>` | a node that is up at the TCP level but does not respond (a gray failure); keeps sockets open, so peers see delay, not reset |
| CPU starvation | `cpulimit`/`taskset` + `stress-ng` (needs install) | slow-node case |
| Disk-full or read-only log dir (M3 only) | `mount -o remount,ro <dir>` or a small tmpfs | the log's write-error paths (K15 in the matrix) |

### 2.4 Power loss and VM faults

- **Real power cut:** a switchable PDU or smart plug on a sacrificial node (needs owner hardware; in the device list as an optional line). Run only for E8m where the question is fsync honesty. Run after a clean `sync`-free sequence: DELETE (wait for the 204) then cut power within a fixed offset; repeat at least 20 times with random offsets in a pre-registered distribution.
- **VM hard power-off:** `virsh destroy <vm>` (KVM) or `Stop-VM -TurnOff` (Hyper-V). This tests process and guest-kernel loss but depends on the hypervisor's write cache; label it `VM_SINGLE_HOST` and do not call it power loss on a physical disk.
- **Magic SysRq crash:** `echo b > /proc/sysrq-trigger` reboots without syncing disks (UNVERIFIED on a given host; needs `kernel.sysrq` enabled). It is closer to a crash than to a power cut because the disk cache is not lost.
- The M3 matrix marks K18 (power loss) "NOT RUN" and says it needs "a VM power cut or a fault-injecting block device" (FACT m3_durability/remediation/crash_point_test_matrix.md, K18 row). dm-flakey/dm-log-writes on Linux are options (install/config approval).

### 2.5 Clock faults (Linux)

```sh
timedatectl set-ntp false          # stop sync first
date -s "2026-10-30 18:30:00"      # step (record the intended delta)
chronyc makestep                   # restore, or: timedatectl set-ntp true
```

Run only on VMs or sacrificial hosts. Slewing (a slow drift) can be emulated by `adjtimex --tick` (UNVERIFIED). `faketime` (LD_PRELOAD) can skew one process without touching the system clock (an install; UNVERIFIED that the Rust binary honours it, since it may use the vDSO).

## 3. Windows hosts

Windows has no netem. Built-in options cover partitions and process kills; delay and probabilistic loss need a third-party driver.

### 3.1 Partition: Windows Defender Firewall rules (built in)

Run in an elevated PowerShell on the node.

```powershell
$cp   = 9443
$peer = '10.77.0.12'

# one-way: this node cannot SEND to PEER's cluster port
New-NetFirewallRule -DisplayName 'MH-block-out-N2' -Direction Outbound -Action Block `
    -Protocol TCP -RemoteAddress $peer -RemotePort $cp

# one-way: this node cannot RECEIVE from PEER (new connections to our listener)
New-NetFirewallRule -DisplayName 'MH-block-in-N2'  -Direction Inbound  -Action Block `
    -Protocol TCP -RemoteAddress $peer -LocalPort $cp

# verify
Get-NetFirewallRule -DisplayName 'MH-*' | Format-Table DisplayName,Enabled,Direction,Action

# remove
Remove-NetFirewallRule -DisplayName 'MH-block-out-N2','MH-block-in-N2'
```

Limits. (a) Windows Firewall evaluates connection establishment statefully; already-established flows can keep working after an inbound block rule is added (UNVERIFIED for these rule types; the verification step must check that an existing mTLS connection really stops carrying traffic, for example by sending a probe and observing the gateway's `nodes[].state`). To force it, also restart the connection by killing the pool (restart the peer) or use a deny rule plus `Get-NetTCPConnection | Where-Object RemotePort -eq $cp` to confirm the connection list is empty. (b) Block rules give blackhole behaviour (timeouts), not fast reset. (c) Requires admin rights; a firewall change is a system setting and needs owner approval (CLAUDE.md section 2) even though it is built in.

### 3.2 Delay and loss: third-party tools (all install requests)

| Tool | Mechanism | Provides | Notes |
|---|---|---|---|
| **clumsy** (WinDivert-based GUI/CLI) | user-mode packet divert | lag, drop, throttle, out-of-order, duplicate, tamper, with a filter such as `tcp.DstPort == 9443 and ip.DstAddr == 10.77.0.12` | Easiest. Loads a kernel driver (WinDivert, signed). The filter language must be validated; the effective loss rate and delay must be measured with a probe after applying (verification), not assumed. Maintenance state and licence: UNVERIFIED (check before approval). |
| **WinDivert / custom WFP callout** | write a small tool | exact, scriptable | A development task; needs a code-signing/driver decision; not recommended unless clumsy is rejected. |
| **Windows Filtering Platform via `netsh wfp`** | built-in diagnostics | capture/diagnose only, not delay or loss | Not an injection tool. |
| **Windows QoS policy** (`New-NetQosPolicy -ThrottleRateActionBitsPerSecond`) | built-in | bandwidth throttling, not delay or loss | Useful only for a bandwidth-limit experiment (not in this plan). |
| **A Linux router/bridge in the middle** | put a Linux box (or VM) as the L2/L3 path with netem | same as 2.1 for all hosts, whatever their OS | Cleanest for a mixed-OS cluster: the impairment is applied on a dedicated device, not on a node. Needs one more device (in `01` it is an optional line) and a routed topology. PROPOSAL. |

### 3.3 Process faults (Windows)

| Fault | Command | Notes |
|---|---|---|
| Crash | `taskkill /F /PID <pid>` or `Stop-Process -Id <pid> -Force` | TerminateProcess; no handlers run. Used by the harness `kill_node` and the M3 reproduction (FACT reproduction_report.md section 2). |
| Graceful stop | CTRL_BREAK to the process group (FACT reproduction_report.md section 4; the M3 build's `ctrl_break` listener is proposed, not present in G) | In build G, plain Ctrl-C reaches only a console-attached process; a service wrapper behaviour is UNVERIFIED. |
| Pause | `Suspend-Process` (PsSuspend from Sysinternals; install request) | Gray failure. |
| Reboot | `shutdown /r /t 0 /f` | Hard-ish stop. |

### 3.4 Clock faults (Windows)

```powershell
Stop-Service w32time
Set-Date -Date '2026-10-30 18:30:00'      # step; record the intended delta
Start-Service w32time; w32tm /resync /force   # restore
```

VM or sacrificial host only.

## 4. The existing Rust fault proxy (`tests/support/fault_proxy.rs`)

- **FACT:** `FaultMode` = `Normal`, `Delay { ms }`, `Blackhole`, `Partition`, `DropNewConnections` (`fault_proxy.rs:21` to `:27`). Each proxy forwards TCP from a local listener to one peer's cluster address and can change mode at runtime (`set_mode`, line 126). It exposes counters (`ProxyMetrics`: connections accepted/dropped, bytes forwarded, delayed events, lines 30 to 35).
- **FACT:** the cluster harness starts one proxy per ordered node pair and points each node's `PHANTOM_CLUSTER_PEER_ADDRS` at the proxy listening address, not at the peer (`cluster_harness.rs` around lines 430 to 489). This is how the loopback tests inject a partition on a single link in one direction (`set_link_mode(from, to, mode)`, line 773; `set_symmetric_partition`, line 779).
- **Limits.** (a) There is no random packet-loss mode; "loss" is only `Blackhole`/`Partition`/`DropNewConnections` (consistent with the auditor note in claims_ledger.md C6 "no loss fault mode"). (b) It is **test support code**, compiled into test crates; it is not a standalone program. Running it on a separate host as a proxy needs a small binary wrapper (a code change, owner approval; listed in `04`, section 6). (c) It adds a user-space hop on the data path of the *cluster* link, which perturbs latency even in `Normal` mode; do not mix proxied and unproxied runs in one comparison. (d) It operates at TCP level on the encrypted stream; it can delay or cut connections but cannot drop individual TLS records selectively.
- **Role in this plan:** use it for the loopback and WSL2 rehearsal (identical logic to the existing tests), and as a cross-check on physical hosts only if the OS-level tools are unavailable. Prefer kernel-level injection (netem, firewall) for the physical study because it does not add a user-space hop.

## 5. Verification of each fault (mandatory, recorded)

| Fault | Verification (before the measured trial) |
|---|---|
| Partition (drop/block) | From the sender host: `curl`/`nc -vz -w 2 <peer> <cp>` times out; from the controller: gateway `nodes[]` view changes (via `GET /api/v1/metrics`, FACT harness `node_states`, around line 705). Also confirm the reverse direction behaves as intended for asymmetric faults. |
| Delay | `ping -c 20 <peer>` shows the added RTT (ICMP is **not** filtered by the cluster-port filters in 2.1; use a TCP connect timer such as `curl -w '%{time_connect}' --connect-timeout 2 telnet://<peer>:<cp>` or the gateway's own latency telemetry). Record measured delay, not the configured one. |
| Loss | `iperf3 -u` or repeated TCP connects: measured loss rate over at least 1,000 packets within the configured band. For netem, `tc -s qdisc` shows drop counters. |
| Process kill | `Get-Process`/`ps` confirms the pid is gone; the gateway log's last line and the controller's timestamp are recorded. |
| Clock step | `date`/`Get-Date` on the node, and the controller's observed delta, recorded before and after. |

If the verification fails the trial is invalid (`00`, 6.4).

## 6. Watchdogs (safety)

Always start a watchdog **before** applying a fault:

```sh
# Linux: remove the netem qdisc and flush the study's iptables rules after 120 s
( sleep 120; tc qdisc del dev $IF root 2>/dev/null; iptables -D OUTPUT -d $PEER -p tcp --dport $CP -j DROP 2>/dev/null ) &
```

```powershell
# Windows: schedule removal of the study's firewall rules after 120 s
Start-Job { Start-Sleep 120; Remove-NetFirewallRule -DisplayName 'MH-*' -ErrorAction SilentlyContinue }
```

Choose the watchdog delay longer than the trial and shorter than the operator's patience. Record that the watchdog ran or was cancelled.

## 7. What to record for every fault event

`fault_id`, experiment id, trial index, host(s), direction(s), mechanism and tool version, exact command line (secrets redacted), requested parameters (delay ms, loss %, mode), start and stop time on the controller clock and on each affected node's clock, the verification output, the measured effective parameters, the node states before and after, the watchdog status, and the operator. Also record the same fields for the **no-fault control** trials, with `fault = none`.

## 8. Install and approval requests (nothing is installed)

Each item needs the owner's approval before anyone installs it (CLAUDE.md section 2: "installing software, changing system or global settings"). Alternatives are listed so the owner can choose the least intrusive.

| # | Request | Where | Purpose | Alternative | Notes |
|---|---|---|---|---|---|
| R1 | `iproute2` (`tc`) and `iptables`/`nftables` | Linux hosts | netem and partitions | none needed on most distros (already present: UNVERIFIED) | check before installing |
| R2 | clumsy | Windows node hosts | delay/loss on Windows | a Linux middle box with netem (3.2, last row) | loads a signed kernel driver; disable afterwards |
| R3 | `iperf3` (or equivalent) | all hosts | link floor and effective-loss check | `ping`/`curl` based checks (weaker) | portable binary |
| R4 | `chrony` (Linux) / `w32time` (Windows built in) | all hosts | clock sync and measured offset | none | needed for method P2 |
| R5 | OpenSSH server or WinRM enabled | hosts | remote control | manual console operation (slow, error-prone) | opens a management surface; keep it on the isolated segment only |
| R6 | Sysinternals PsSuspend | Windows hosts | process pause | none | optional |
| R7 | `stress-ng`, `cpulimit` | Linux hosts | CPU starvation | none | optional |
| R8 | A switchable PDU or smart plug | one node | real power loss (E8m) | VM power-off (weaker, labelled) | hardware; optional |
| R9 | A hypervisor feature (Hyper-V role) or another hypervisor | one Windows machine | option C2 | WSL2 (C1) or loopback (C0) | system setting; the machine's edition may not support Hyper-V (UNVERIFIED) |
| R10 | `openssl` (or a small Rust/Go cert tool) | controller | certificate and membership-file generation (`03`, section 3) | the existing `scripts/gen-demo-certs.*` covers ingress certs only | openssl is usually present on Linux and Git for Windows (UNVERIFIED) |
| R11 | Standalone wrappers (fault proxy binary, load generator, origin) | build host | `04`, section 6 | none | **code changes**: separate approval; not installs |
