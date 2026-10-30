# RFC 9111 HTTP Caching Conformance & Supported Subset

**Specification Reference:** RFC 9111 (HTTP Caching) & RFC 9110 (HTTP Semantics)  
**Implementation:** `src/core/freshness.rs`  
**Test Suite:** `tests/integration/freshness_matrix_tests.rs` (65 table-driven matrix test cases + property tests)  
**Status:** M2-02 VERIFIED  

---

## 1. Supported Response Directives & Headers

| Directive / Header | RFC 9111 Section | Supported Behavior in PhantomCache | Status |
|---|---|---|---|
| `Cache-Control: no-store` | §5.2.2.5 | Uncacheable. The response is never stored into memory or shard map. | **VERIFIED** |
| `Cache-Control: private` | §5.2.2.7 | Uncacheable. PhantomCache acts as a shared cache gateway; private responses are not stored. | **VERIFIED** |
| `Cache-Control: no-cache` | §5.2.2.4 | Stored with freshness lifetime 0. Mandatory conditional revalidation before serving. | **VERIFIED** |
| `Cache-Control: max-age=N` | §5.2.2.1 | Sets freshness lifetime to `N * 1000` ms. | **VERIFIED** |
| `Cache-Control: s-maxage=N` | §5.2.2.10 | Shared cache precedence: overrides `max-age` and `Expires` for freshness lifetime. | **VERIFIED** |
| `Cache-Control: must-revalidate` | §5.2.2.2 | Forbids serving stale responses after expiry even under stale-serving policies. | **VERIFIED** |
| `Cache-Control: proxy-revalidate` | §5.2.2.9 | Treated identically to `must-revalidate` for shared cache gateway. | **VERIFIED** |
| `Cache-Control: public` | §5.2.2.9 | Permits shared caching of authenticated requests (`Authorization`) and `Set-Cookie`. | **VERIFIED** |
| `Cache-Control: stale-while-revalidate=N` | §5.2.2.11 / RFC 5861 | Allows serving stale for up to `N` seconds while triggering background revalidation (bounded by hard cap `PHANTOM_MAX_STALE_MS`). | **VERIFIED** |
| `Cache-Control: stale-if-error=N` | §5.2.2.12 / RFC 5861 | Allows serving stale for up to `N` seconds if origin returns 5xx or connection error. | **VERIFIED** |
| `Expires` & `Date` | §5.3 / §5.1.1 | When `max-age` and `s-maxage` are absent, lifetime = `max(0, Expires - Date)`. | **VERIFIED** |
| `Age: N` | §5.1 | Incorporated into current age calculation per §4.2.3; injected into all cache hits. | **VERIFIED** |
| `ETag` | RFC 9110 §8.8.3 | Stored as validator; supports strong (`"..."`) and weak (`W/"..."`) tags. | **VERIFIED** |
| `Last-Modified` | RFC 9110 §8.8.2 | Stored as validator in standard HTTP date format. | **VERIFIED** |
| `If-None-Match` | RFC 9110 §13.1.2 | Evaluated on GET requests; weak comparison permitted per §8.8.3.2; returns `304 Not Modified` on match. | **VERIFIED** |
| `If-Modified-Since` | RFC 9110 §13.1.3 | Evaluated on GET requests; returns `304 Not Modified` if resource unchanged since client date. | **VERIFIED** |
| 304 Header Merge | §4.3.4 | On 304 Not Modified, cached metadata (freshness, validators) updated without reallocating body. | **VERIFIED** |

---

## 2. Policy & Security Boundaries

1. **Authorization Restriction (RFC 9111 §3.5):** Requests forwarding an `Authorization` header are uncacheable by default unless the origin response explicitly contains `public`, `s-maxage`, or `must-revalidate`.
2. **Set-Cookie Restriction:** Responses containing `Set-Cookie` are treated as private and uncacheable by default unless explicitly permitted by `public`, `s-maxage`, or `must-revalidate`.
3. **Vary Restriction (O2-3):** Responses with `Vary: *` or varying on headers other than normalized `Accept-Encoding` are treated as uncacheable. Secondary key index caching is explicitly out of scope.
4. **Status Codes:** Status 200 is cacheable by default. Statuses 301, 404, and 410 are cacheable only when explicit freshness directives (`max-age`, `s-maxage`, or `Expires`) are present. All 5xx and error statuses are uncacheable.
5. **Close-Delimited Framing:** Responses without explicit framing (`Content-Length` or `Transfer-Encoding: chunked`) are never stored into cache, preventing smuggling and truncated body caching.
6. **Default TTL Fallback (O2-1):** In the absence of any origin freshness headers, the gateway defaults to `PHANTOM_DEFAULT_TTL_MS = 60000` ms (preserving Phase 1 baseline behavior).

---

## 3. Explicitly Unsupported Features (Non-Goals)

- `Vary` secondary cache key indexing (responses with other Vary headers are uncacheable).
- Heuristic freshness calculation for arbitrary status codes (explicit directives required).
- Range requests / Partial content (`206 Partial Content`).
- Client request `Cache-Control` directives (gateway enforces server-side policy).
- `HEAD` method response caching (GET and DELETE only).

