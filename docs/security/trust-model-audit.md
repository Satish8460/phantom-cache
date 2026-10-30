# PhantomCache — Security Trust Model & Request Path Audit

## 1. Request Path Trace & Operating Model

This section documents the actual, verified execution paths of PhantomCache based on direct inspection of `src/` and `tests/`.

### 1.1 Ingress Routing & Operating Model Classification
PhantomCache is implemented as an **Explicit Application-Level REST Cache Gateway API**, rather than a standard RFC 9110 HTTP Reverse Proxy or Forward Proxy.

* **Code Evidence:** [src/api/server.rs](file:///d:/Personal/phantom-cache/src/api/server.rs#L61-L80)
  ```rust
  pub fn create_app(shared_state: Arc<ApiState>) -> Router {
      let cors = CorsLayer::permissive();
      Router::new()
          .route("/health", get(health_check))
          .route("/api/events", get(sse_handler))
          .route("/api/control", post(control_handler))
          .route(
              "/api/v1/cache/*key",
              get(get_cache_handler).delete(delete_cache_handler),
          )
          .route(
              "/api/v1/cache",
              get(get_cache_handler_root).delete(delete_cache_handler_root),
          )
          .route("/api/v1/diagnostics/summary", get(diagnostics_summary_handler))
          .route("/api/v1/diagnostics/events", get(diagnostics_events_handler))
          .with_state(shared_state)
          .layer(axum::extract::DefaultBodyLimit::max(512 * 1024))
          .layer(cors)
  }
  ```

* **Operating Model Reality:**
  - **Not an RFC 9110 Reverse Proxy:** A standard reverse proxy transparently maps arbitrary paths (e.g., `GET /items/42 HTTP/1.1`) to an upstream server (`GET /items/42`). In PhantomCache, all cacheable requests must be explicitly prefixed with `/api/v1/cache/` (e.g., `GET /api/v1/cache/items/42`).
  - **Not an RFC 9110 Forward Proxy:** The server does not support the HTTP `CONNECT` method (RFC 9110 §9.3.6) for TCP tunneling, nor does it parse absolute-URI request targets in the request line (e.g., `GET http://example.com/items/42 HTTP/1.1`).
  - **Restricted HTTP Methods:** Only `GET` (cache fetch) and `DELETE` (invalidation) are registered. `POST`, `PUT`, `PATCH`, and `HEAD` requests to `/api/v1/cache/*` return HTTP 405 Method Not Allowed.

* **Network Interface Binding:**
  - In [src/api/server.rs](file:///d:/Personal/phantom-cache/src/api/server.rs#L87-L92):
    ```rust
    let addr = SocketAddr::from(([127, 0, 0, 1], 8085));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    ```
  - **Finding:** The server binds strictly to the loopback interface (`127.0.0.1:8085`). It is physically inaccessible to remote devices over a LAN without code modification.

---

### 1.2 Upstream Request Creation & Destination Selection
When a cache miss or cache bypass occurs, PhantomCache creates an outbound connection to an upstream origin.

* **Destination Selection:**
  - In [src/main.rs](file:///d:/Personal/phantom-cache/src/main.rs#L101):
    ```rust
    let origin_url = std::env::var("ORIGIN_URL").ok();
    ```
  - In [src/api/server.rs](file:///d:/Personal/phantom-cache/src/api/server.rs#L250-L253) and [Line 346-349](file:///d:/Personal/phantom-cache/src/api/server.rs#L346-L349):
    ```rust
    let origin_base = state.origin_url.as_deref().unwrap_or("http://127.0.0.1:0");
    ```
  - **Finding:** The upstream destination is configured exclusively via the static `ORIGIN_URL` environment variable. The client cannot dynamically choose the upstream destination in request parameters.

* **Outbound Transport & Origin Fetcher (`fetch_origin_raw`):**
  - In [src/api/server.rs](file:///d:/Personal/phantom-cache/src/api/server.rs#L703-L720):
    ```rust
    let clean_base = origin_base
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let host_port = clean_base.trim_end_matches('/');

    let stream = tokio::time::timeout(
        Duration::from_millis(5000),
        tokio::net::TcpStream::connect(host_port),
    ).await...;
    ```
  - **Critical Finding (TLS Stripping):** If `ORIGIN_URL` is configured with `https://`, the scheme prefix is stripped (`trim_start_matches("https://")`) and a raw, unencrypted `tokio::net::TcpStream` is opened directly to the destination host and port. **PhantomCache does not establish TLS to upstream origins.**
  - **Finding (Redirect Handling):** PhantomCache does not follow HTTP 301/302 redirects. Upstream status codes outside 200 are logged as `OriginFetchNon200` and returned as `OriginError::UpstreamStatus(code)`.

* **Header Forwarding Behavior:**
  - In [src/api/server.rs](file:///d:/Personal/phantom-cache/src/api/server.rs#L727-L735):
    ```rust
    for (k, v) in headers {
        let name = k.as_str();
        if name.starts_with("x-") {
            if let Ok(v_str) = v.to_str() {
                req_str.push_str(&format!("{}: {}\r\n", name, v_str));
            }
        }
    }
    ```
  - **Critical Finding (Header Filtering):** PhantomCache forwards **only** headers beginning with `x-`. It strips standard headers including:
    - `Authorization`
    - `Cookie`
    - `Accept`
    - `Accept-Encoding`
    - `User-Agent`

---

### 1.3 Ingress & Upstream TLS Capabilities
- **Ingress TLS:** None. Axum listens on raw plaintext TCP. No TLS handshake occurs, no server certificate is presented, and no client certificates are requested.
- **Upstream TLS:** None. `fetch_origin_raw` opens raw plaintext TCP.
- **HTTPS Tunneling:** None. The HTTP `CONNECT` verb is unsupported.
- **Certificates Validated:** Zero.
- **Identities Authenticated:** Zero.

---

### 1.4 Cache Keys, Tenant Identity, and Cacheability Rules

* **Cache Key Structure:**
  - In [src/core/cache.rs](file:///d:/Personal/phantom-cache/src/core/cache.rs#L20-L56):
    ```rust
    pub struct CacheKey {
        pub tenant_id: String,
        pub path: String,
        pub query: Option<String>,
    }
    ```
    Canonical string representation: `{tenant_id}:{path}:?{query}`.

* **Tenant Identity Source:**
  - In [src/api/server.rs](file:///d:/Personal/phantom-cache/src/api/server.rs#L224) & [Line 555](file:///d:/Personal/phantom-cache/src/api/server.rs#L555):
    ```rust
    let tenant_id = headers.get("x-tenant-id").and_then(|v| v.to_str().ok());
    ```
  - **Critical Finding (Tenant Spoofing):** The tenant identifier is extracted directly from the unauthenticated client header `x-tenant-id`. The gateway performs no signature verification, token validation, or access control check on this header. Any requester can impersonate any tenant.

* **Response Cacheability Filters:**
  - In [src/api/server.rs](file:///d:/Personal/phantom-cache/src/api/server.rs#L372-L386):
    ```rust
    let is_uncacheable = headers_out.iter().any(|(k, v)| {
        if let Some(name) = k {
            let n = name.as_str();
            if n.eq_ignore_ascii_case("cache-control") {
                if let Ok(s) = v.to_str() {
                    return s.contains("private") || s.contains("no-store");
                }
            }
            if n.eq_ignore_ascii_case("set-cookie") {
                return true;
            }
        }
        false
    });
    ```
  - **Status (Verified):** Responses containing `Cache-Control: private`, `Cache-Control: no-store`, or `Set-Cookie` are marked `FinalizeOutcome::Uncacheable`. They are delivered to current in-flight waiters but are **never stored** in `shard.map`. Verified in `tests/integration/m1_http_tests.rs::test_m1_t19_uncacheable_headers_and_bypass`.

* **RFC 9111 Cache Isolation Deficiencies:**
  1. **Missing `Authorization` Header Handling (RFC 9111 §3.5):** If a client sends an `Authorization: Bearer <secret>` header (or custom `x-auth-token`) and the origin returns a public response without `private` or `no-store`, PhantomCache stores it under `{tenant_id}:{path}`. A subsequent unauthenticated client requesting the same path will receive a **CACHE HIT**, bypassing origin authentication.
  2. **Missing `Vary` Header Support (RFC 9111 §4.1):** PhantomCache does not store or evaluate the `Vary` header. If the origin returns content varying on headers (such as `Accept-Encoding: gzip` vs `identity` or custom headers), all clients receive the single cached variant.

---

### 1.5 Endpoints & Administrative Access

| Endpoint | Method | Function in Code | Access Control | Purpose |
| :--- | :--- | :--- | :--- | :--- |
| `/health` | `GET` | `health_check` | **None** (Public) | Returns `"OK"` |
| `/api/events` | `GET` | `sse_handler` | **None** (Public) | Emits SSE telemetry every 100ms |
| `/api/control` | `POST` | `control_handler` | **None** (Public) | Mutates workload state; triggers node failures |
| `/api/v1/cache/*key` | `GET` | `get_cache_handler` | **None** (Public) | Fetches / coalesces cached entries |
| `/api/v1/cache/*key` | `DELETE` | `delete_cache_handler` | **None** (Public) | Invalidates key from cache shard |
| `/api/v1/diagnostics/summary` | `GET` | `diagnostics_summary_handler`| **None** (Public) | Emits snapshot of all atomic counters |
| `/api/v1/diagnostics/events` | `GET` | `diagnostics_events_handler` | **None** (Public) | Dumps last 100 diagnostic events |

* **Critical Finding (Unauthenticated Admin Controls):**
  [src/api/server.rs#L163-L189](file:///d:/Personal/phantom-cache/src/api/server.rs#L163-L189) exposes `/api/control` to any caller. Sending `POST /api/control` with `{ "event": "NODE_FAILURE" }` marks cluster nodes failed. Sending `{ "event": "FLASH_CROWD" }` alters simulated workload rates. There is zero authentication, token validation, or role check.

---

## 2. Prioritized Security Gap Matrix

| ID | Vulnerability Description | Code Location | Severity | Exploitability | Impact | Recommended Remediation | Effort |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **GAP-01** | **Unauthenticated Control API** | [server.rs:163](file:///d:/Personal/phantom-cache/src/api/server.rs#L163) | **Critical** | Trivial (Single HTTP POST) | High: Denial of service, cluster state tampering | Require Bearer JWT with `role: admin` or `role: operator` | 2 Days |
| **GAP-02** | **Tenant Header Spoofing** | [server.rs:224](file:///d:/Personal/phantom-cache/src/api/server.rs#L224) | **Critical** | Trivial (`x-tenant-id` header) | High: Cross-tenant data tampering / invalidation | Derive tenant ID strictly from authenticated JWT/mTLS claims | 3 Days |
| **GAP-03** | **Unencrypted Ingress / No TLS** | [server.rs:87](file:///d:/Personal/phantom-cache/src/api/server.rs#L87) | **High** | Easy (LAN packet sniffing) | High: Plaintext exposure of cached data & tokens | Implement Rustls TLS 1.3 ingress termination | 3 Days |
| **GAP-04** | **Origin TLS Stripping** | [server.rs:704](file:///d:/Personal/phantom-cache/src/api/server.rs#L704) | **High** | Inherent in code | High: Plaintext transit over upstream network | Implement outbound TLS client with SNI validation | 3 Days |
| **GAP-05** | **SSRF Risk in Upstream Fetch** | [server.rs:716](file:///d:/Personal/phantom-cache/src/api/server.rs#L716) | **High** | Moderate (if origin configurable) | High: Probing loopback or cloud metadata (169.254.169.254) | Enforce destination allowlist & IP range blacklist | 2 Days |
| **GAP-06** | **RFC 9111 Auth Leakage** | [server.rs:210](file:///d:/Personal/phantom-cache/src/api/server.rs#L210) | **Medium** | Moderate | Medium: Cache hit served to unauthenticated client | Enforce RFC 9111 §3.5 rules on `Authorization` headers | 2 Days |
| **GAP-07** | **Unauthenticated Distributed TCP** | [node.rs:82](file:///d:/Personal/phantom-cache/src/dist/node.rs#L82) | **Medium** | Easy (if nodes exposed to LAN) | Medium: Rogue nodes injecting fake cache hits | Mutual TLS (mTLS) with shared cluster CA | 4 Days |
| **GAP-08** | **Unauthenticated SSE Stream** | [server.rs:98](file:///d:/Personal/phantom-cache/src/api/server.rs#L98) | **Low** | Easy | Low: Information disclosure of traffic metrics | Require read-only bearer token on `/api/events` | 1 Day |
| **GAP-09** | **Missing Release Signatures** | CI / Release | **Medium** | Hard | Medium: Execution of tampered binary | Cosign / SLSA release manifest + `phantom-verify` CLI | 3 Days |

