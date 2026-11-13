# PhantomStore & Distributed Multi-Device Demonstration (Phantom Crowd)
## Flagship Engineering Charter & Architecture Specification

**Document Type:** Architecture & Technical Specification Charter  
**Status:** P0 FLAGSHIP WORK ITEM (Approved by Project Owner)  
**Date:** 2026-11-05  
**Preceding Deliverable:** Demo Hub Flagship UI/UX Redesign (Completed & Verified: 118 PASS, 0 FAIL)  
**Target Capabilities:** Multi-Device Connectivity, Distributed Virtual-User Workload Generation, Realistic E-Commerce Storefront, Real-Time Orchestration.

---

## 1. Executive Summary & Product Vision

PhantomCache has established rigorous distributed systems engineering:
- Request coalescing via singleflight leader locks.
- Cache memory eviction via circular FIFO SIEVE (NSDI 2024).
- Key distribution via deterministic 64-bit FNV-1a HRW Rendezvous Hashing.
- Tenant isolation via cryptographic namespace partitioning (`tenant_id::path`).

However, demonstrating these capabilities on a single loopback browser window limits the perceived realism of the system. Demonstrating load from 1 laptop simulating 1,000 customers demonstrates load generation; demonstrating **5 independent physical devices (laptops, phones, tablets) each generating 200 virtual customers** interacting with a realistic e-commerce storefront demonstrates **real-world distributed systems resilience**.

```
+----------------------------------------------------------------------------------------------------+
|                                    CENTRAL DEMONSTRATION HOST                                      |
|                                                                                                    |
|   +-----------------------+     +-------------------------------+     +------------------------+   |
|   |   Demo Hub & Crowd    |     |     PhantomCache Cluster      |     |     Slow Origin        |   |
|   |     Orchestrator      |<--->|   Gateway 1 (:18201) [Leader] |<--->|       Backend          |   |
|   |       (:3201)         |     |   Gateway 2 (:18202) [Peer]   |     |       (:19200)         |   |
|   |  - Central Scenarios  |     |   Gateway 3 (:18203) [Standby]|     |  - ~40-70 ms delay     |   |
|   |  - Live Device Grid   |     +-------------------------------+     +------------------------+   |
|   |  - Global Telemetry   |                     ^                                                  |
|   +-----------------------+                     | Real Storefront Traffic                          |
|               ^                                 | (/store, /img, /api)                             |
|               | Device Registration &           |                                                  |
|               | Experiment Sync                 v                                                  |
|               |                     +-----------------------+                                      |
|               +-------------------->|  PhantomStore Engine  |                                      |
|                                     |    Storefront UI      |                                      |
|                                     +-----------------------+                                      |
+-------------------------------------------------+--------------------------------------------------+
                                                  |
                           LAN / Wi-Fi Connection | (e.g. http://192.168.1.50:3201/crowd)
                                                  |
      +---------------------+---------------------+---------------------+---------------------+
      |                     |                     |                     |                     |
      v                     v                     v                     v                     v
+------------+        +------------+        +------------+        +------------+        +------------+
|  Laptop A  |        |  Laptop B  |        |  Laptop C  |        |  Mobile A  |        |  Mobile B  |
| 200 Users  |        | 200 Users  |        | 200 Users  |        | 200 Users  |        | 200 Users  |
| (Tenant A) |        | (Tenant A) |        | (Tenant B) |        | (Tenant B) |        | (Attacker) |
+------------+        +------------+        +------------+        +------------+        +------------+
      \                     \                     |                     /                     /
       =======================================================================================
                               1,000 AGGREGATE VIRTUAL CUSTOMERS
```

---

## 2. Fundamental Architectural Boundaries

### 2.1 The Two Independent Identities
A core architectural requirement is the strict separation between physical device identities and virtual customer identities:

| Entity | Identifier Format | Semantic Meaning | Lifecycle |
|---|---|---|---|
| **Physical Device** | `dev-<type>-<hash>` (e.g. `dev-laptop-7a2e`) | A real, physical hardware machine connected over the LAN. | Persistent across experiments; tied to physical browser session. |
| **Virtual Customer** | `cust-<dev>-<index>` (e.g. `cust-laptop7a-042`) | A simulated customer session generated by that machine's worker agent. | Transient; spun up and torn down per experiment workload plan. |
| **Tenant Namespace** | `tenant-a`, `tenant-b` | The organizational security domain for cache-key partitioning (`tenant_id::path`). | Configured per workload; a single physical device may simulate customers across multiple tenants. |

> [!IMPORTANT]
> Never conflate virtual customers with physical devices. A device is a compute source; a virtual customer is an authenticated application entity.

### 2.2 Host Authority & Security Invariants
1. **Host-Only Control:** Only the Host Machine (the operator) can start scenarios, trigger cluster failure injection, change cluster topology, or terminate experiments. External devices are strictly **traffic generators and telemetry observers**.
2. **Zero Internal API Exposure:** Connected devices interact strictly through the public storefront endpoints (`/store`, `/img`, `/api/v1/cache/`) and the client crowd socket (`/api/crowd/*`). Internal cluster gossip, raw process kill signals, and admin tokens are never exposed over the LAN.
3. **No Added Gateway:** PhantomStore routes directly through the existing PhantomCache gateways (`:18201-:18203`). We do not introduce Envoy, Nginx, or an auxiliary reverse proxy.
4. **Truthful Telemetry:** Generator metrics (requests sent, client latency, client network errors) are tracked independently from PhantomCache server metrics (cache hits, origin fetches, peer fills). Both are displayed side-by-side to highlight differences between client-perceived and server-measured performance.

---

## 3. High-Value Demonstration Experiments

| # | Experiment Name | Workload Mechanics | Architectural Proof |
|---|---|---|---|
| **E1** | **Distributed Flash Crowd** | 5 devices simultaneously trigger a flash sale on the exact same product SKU (`/product/flash-keyboard`). 1,000 concurrent client requests hit the gateways. | **Singleflight Coalescing:** 1,000 independent requests across 5 physical devices collapse into **exactly 1 origin fetch**. 999 clients receive instant coalesced responses upon origin return. |
| **E2** | **Distributed Organic Browsing** | 5 devices browse the 50-product catalog following a realistic Zipf popularity curve (`alpha = 0.9`). | **SIEVE Eviction & Hot-Key Memory:** Demonstrates cache hit ratio convergence and eviction of long-tail items with visited bits (`v: 1` / `v: 0`) under concurrent multi-device traffic. |
| **E3** | **Multi-Tenant Cross-Device Isolation** | Laptop 1 & 2 generate Tenant A customers; Laptop 3 & Mobile 1 generate Tenant B customers requesting identical product paths. | **Cryptographic Key Partitioning:** Zero cache-bleed across physical devices. Tenant B customers start cold in their isolated keyspace (`tenant-b::/product/12`). |
| **E4** | **Physical Generator Disconnection** | Laptop 2 abruptly disconnects (Wi-Fi turned off or tab closed) mid-flash crowd. | **Generator Failure vs. Cluster Failure:** The Host Demo Hub detects generator dropout immediately; aggregate traffic decreases by 20%, but PhantomCache cluster remains 100% healthy with zero error spikes. |
| **E5** | **Live Gateway Node Termination** | Host terminates Gateway Node 3 (`:18203`) while all 5 physical devices continue hammering the cluster. | **High-Availability HRW Failover:** Devices experience momentary retry on in-flight sockets, while surviving Gateways 1 and 2 rebalance keyspace via 64-bit FNV-1a (33.3% → 50%/50%) with zero cascading origin collapse. |

---

## 4. Technical Phasing Roadmap

### Phase P0.1: Multi-Device Connectivity & Registration Hub
- **LAN Binding:** Update `scripts/demo_console/server.mjs` to bind to `0.0.0.0` with configurable `--host` flag. Auto-detect host LAN IPv4 address (e.g. `192.168.1.x`).
- **Join URL & QR Code:** Demo Hub displays a high-visibility "Connect Devices" banner with short link `http://<LAN-IP>:3201/crowd` and an inline SVG QR code for mobile camera joining.
- **Device Registration Handshake:**
  - `POST /api/crowd/register`: Device submits `{ device_id, type: 'mobile'|'laptop'|'desktop', name, ua }`. Receives `{ assigned_id, host_time, cluster_state }`.
  - `POST /api/crowd/heartbeat`: 2-second heartbeat with generator health status and ping estimation.
  - `GET /api/crowd/stream`: Server-Sent Events stream delivering experiment state transitions to connected devices.
- **Host Device Cockpit:** Demo Hub displays a real-time **Connected Devices Grid** with live status pills, device icons, ping, and assigned user quotas.

### Phase P0.2: PhantomStore E-Commerce Storefront
- **Storefront Application (`/store`):** A fast, modern e-commerce web application inspired by flagship retail platforms (clean typography, product cards, inventory badges, instant search, responsive grid).
- **Synthetic Product Catalog:**
  - 50 realistic products with high-resolution images, metadata, pricing, reviews, and stock.
  - Image URLs route through `/api/v1/cache/store/img/*` backed by PhantomCache gateways.
- **Customer User Actions:**
  - View Product Detail Modal.
  - Instant Add-to-Cart.
  - Flash Sale Countdown & Buy Button.

### Phase P0.3: Distributed Workload Agent Engine
- **In-Browser Multi-Customer Generator:**
  - Runs in Web Worker (or lightweight async fiber pool) on each connected device.
  - Configurable `virtual_users` slider (1 to 200 per device).
  - Emulates concurrent customer sessions: think time, browsing paths, product selection.
- **Telemetry Gathering:**
  - Tracks local request rate, latency histogram (p50, p95, max), HTTP status breakdown (`200`, `401`, `503`), and local network errors.
  - Batched upstream telemetry reporting to Host Demo Hub every 500ms.

### Phase P0.4: Demonstration Intelligence & Live Experiment Coordination
- **Synchronized Flash Crowd Launch:** Host triggers experiment countdown (3.. 2.. 1.. FIRE). All connected devices transition from `STANDBY` to `BURST` simultaneously.
- **Real-Time Aggregator:** Demo Hub aggregates telemetry across all physical devices in real time:
  - Global Concurrency dial (e.g. `1,000 Active Customers across 5 Devices`).
  - Total HTTP Request Rate (`2,840 req/s`).
  - Origin Shielding Gauge (`99.8% Offload`).
  - Real-time Sparklines per physical device.

### Phase P0.5: Security, Chaos Resilience & Automated Acceptance
- **Strict Role-Based Authority:** Observer devices are strictly read-only. Generators have scoped request-generation authority.
- **Chaos Resilience:** Tolerates physical device dropouts, intermittent mobile radio latency, and browser background throttling without desynchronizing central hub.
- **Automated Verification Harness:** Dedicated test suite (`scripts/demo_console/crowd_check.mjs`) verifying:
  - Multi-device registration handshake.
  - Virtual customer concurrency scaling.
  - Zero token leakage across LAN endpoints.
  - 100% preservation of the 118 existing Demo Hub test assertions.

---

## 5. Verification & Acceptance Criteria

1. **Physical Multi-Device Proof:** Successfully connect at least 2 distinct physical machines/phones on local LAN simultaneously.
2. **Virtual Scale Proof:** Demonstrate aggregate virtual user traffic scaling to 1,000 concurrent customers across connected devices.
3. **Storefront Realism:** Real products, images, and prices served through PhantomCache with sub-millisecond edge response on hits.
4. **Singleflight Flash Crowd Proof:** 1,000 concurrent requests across multiple physical devices on a single product SKU yield **exactly 1 origin fetch**.
5. **No Regressions:** Existing `hub_check.mjs` test suite maintains **118 PASS, 0 FAIL**.

