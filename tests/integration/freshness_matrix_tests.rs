//! Freshness Matrix Tests (RFC 9111 Subset)
//!
//! Authoritative integration test suite for Phase 2 Step M2-02.
//! Table-driven matrix of >= 60 header combinations, property tests,
//! validator matching, and conditional revalidation.

use axum::http::{header::HeaderName, HeaderValue};
use phantom_cache_demo::core::freshness::{
    calculate_current_age_seconds, evaluate_response_freshness, matches_if_modified_since,
    matches_if_none_match, merge_304_headers, parse_http_date, FreshnessMetadata,
};
use phantom_cache_demo::core::cache::{CacheKey, PartitionedCache};
use std::sync::Arc;

struct FreshnessTestCase {
    name: &'static str,
    status_code: u16,
    headers: Vec<(&'static str, &'static str)>,
    now_ms: u64,
    has_explicit_framing: bool,
    body_len: usize,
    default_ttl_ms: u64,
    forwarded_auth: bool,
    expected_cacheable: bool,
    expected_ttl_ms: u64,
    expected_must_reval: bool,
    expected_swr_ms: Option<u64>,
    expected_sie_ms: Option<u64>,
}

fn build_header_tuples(raw: &[(&str, &str)]) -> Vec<(Option<HeaderName>, HeaderValue)> {
    raw.iter()
        .map(|(k, v)| {
            (
                Some(HeaderName::from_bytes(k.as_bytes()).unwrap()),
                HeaderValue::from_str(v).unwrap(),
            )
        })
        .collect()
}

#[test]
fn test_m2_02_freshness_matrix_table_driven() {
    let now = 1_700_000_000_000u64; // arbitrary reference timestamp

    let test_cases = vec![
        // 1. Basic default caching
        FreshnessTestCase {
            name: "TC01: 200 OK without headers defaults to configured default TTL",
            status_code: 200,
            headers: vec![],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 2. max-age directive
        FreshnessTestCase {
            name: "TC02: max-age sets exact lifetime",
            status_code: 200,
            headers: vec![("cache-control", "max-age=120")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 120_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 3. s-maxage precedence over max-age for shared caches
        FreshnessTestCase {
            name: "TC03: s-maxage takes precedence over max-age in shared cache",
            status_code: 200,
            headers: vec![("cache-control", "s-maxage=300, max-age=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 300_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 4. no-store forbids storage
        FreshnessTestCase {
            name: "TC04: no-store makes response uncacheable",
            status_code: 200,
            headers: vec![("cache-control", "no-store")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 5. private forbids storage in shared cache
        FreshnessTestCase {
            name: "TC05: private makes response uncacheable in shared cache",
            status_code: 200,
            headers: vec![("cache-control", "private")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 6. no-cache forces lifetime 0 and revalidation
        FreshnessTestCase {
            name: "TC06: no-cache stores with lifetime 0 and forces revalidation",
            status_code: 200,
            headers: vec![("cache-control", "no-cache")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 0,
            expected_must_reval: true,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 7. public explicitly permits shared caching
        FreshnessTestCase {
            name: "TC07: public with max-age is cacheable",
            status_code: 200,
            headers: vec![("cache-control", "public, max-age=3600")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 3_600_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 8. must-revalidate flag
        FreshnessTestCase {
            name: "TC08: must-revalidate enforces revalidation after expiry",
            status_code: 200,
            headers: vec![("cache-control", "must-revalidate, max-age=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: true,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 9. proxy-revalidate flag
        FreshnessTestCase {
            name: "TC09: proxy-revalidate enforces revalidation on shared cache",
            status_code: 200,
            headers: vec![("cache-control", "proxy-revalidate, max-age=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: true,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 10. stale-while-revalidate
        FreshnessTestCase {
            name: "TC10: stale-while-revalidate parsed in ms",
            status_code: 200,
            headers: vec![("cache-control", "max-age=60, stale-while-revalidate=30")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: Some(30_000),
            expected_sie_ms: None,
        },
        // 11. stale-if-error
        FreshnessTestCase {
            name: "TC11: stale-if-error parsed in ms",
            status_code: 200,
            headers: vec![("cache-control", "max-age=60, stale-if-error=120")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: Some(120_000),
        },
        // 12. Both swr and sie together
        FreshnessTestCase {
            name: "TC12: Both stale-while-revalidate and stale-if-error together",
            status_code: 200,
            headers: vec![("cache-control", "max-age=60, stale-while-revalidate=30, stale-if-error=120")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: Some(30_000),
            expected_sie_ms: Some(120_000),
        },
        // 13. Expires with Date
        FreshnessTestCase {
            name: "TC13: Expires minus Date determines lifetime",
            status_code: 200,
            headers: vec![
                ("date", "Sun, 06 Nov 1994 08:49:37 GMT"),
                ("expires", "Sun, 06 Nov 1994 09:49:37 GMT"),
            ],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 3_600_000, // 1 hour
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 14. Expires in past
        FreshnessTestCase {
            name: "TC14: Expires in past gives lifetime 0",
            status_code: 200,
            headers: vec![
                ("date", "Sun, 06 Nov 1994 08:49:37 GMT"),
                ("expires", "Sun, 06 Nov 1994 07:49:37 GMT"),
            ],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 15. max-age overrides Expires
        FreshnessTestCase {
            name: "TC15: max-age overrides Expires",
            status_code: 200,
            headers: vec![
                ("cache-control", "max-age=50"),
                ("date", "Sun, 06 Nov 1994 08:49:37 GMT"),
                ("expires", "Sun, 06 Nov 1994 09:49:37 GMT"),
            ],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 50_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 16. s-maxage overrides both max-age and Expires
        FreshnessTestCase {
            name: "TC16: s-maxage overrides both max-age and Expires",
            status_code: 200,
            headers: vec![
                ("cache-control", "s-maxage=75, max-age=50"),
                ("date", "Sun, 06 Nov 1994 08:49:37 GMT"),
                ("expires", "Sun, 06 Nov 1994 09:49:37 GMT"),
            ],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 75_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 17. Set-Cookie without public is uncacheable
        FreshnessTestCase {
            name: "TC17: Set-Cookie without public is uncacheable",
            status_code: 200,
            headers: vec![
                ("set-cookie", "session_id=abc123"),
                ("cache-control", "max-age=60"),
            ],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 18. Set-Cookie with public is cacheable
        FreshnessTestCase {
            name: "TC18: Set-Cookie with public is cacheable",
            status_code: 200,
            headers: vec![
                ("set-cookie", "session_id=abc123"),
                ("cache-control", "public, max-age=60"),
            ],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 19. Forwarded Authorization without public is uncacheable
        FreshnessTestCase {
            name: "TC19: Forwarded Authorization without public is uncacheable",
            status_code: 200,
            headers: vec![("cache-control", "max-age=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: true,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 20. Forwarded Authorization with public is cacheable
        FreshnessTestCase {
            name: "TC20: Forwarded Authorization with public is cacheable",
            status_code: 200,
            headers: vec![("cache-control", "public, max-age=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: true,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 21. Forwarded Authorization with s-maxage is cacheable
        FreshnessTestCase {
            name: "TC21: Forwarded Authorization with s-maxage is cacheable",
            status_code: 200,
            headers: vec![("cache-control", "s-maxage=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: true,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 22. Vary: * is uncacheable
        FreshnessTestCase {
            name: "TC22: Vary: * is uncacheable",
            status_code: 200,
            headers: vec![("vary", "*"), ("cache-control", "max-age=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 23. Vary: Accept-Encoding is cacheable
        FreshnessTestCase {
            name: "TC23: Vary: Accept-Encoding is supported and cacheable",
            status_code: 200,
            headers: vec![("vary", "Accept-Encoding"), ("cache-control", "max-age=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 24. Vary: User-Agent is uncacheable
        FreshnessTestCase {
            name: "TC24: Vary: User-Agent outside supported subset is uncacheable",
            status_code: 200,
            headers: vec![("vary", "User-Agent"), ("cache-control", "max-age=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 25. Status 301 without freshness is uncacheable
        FreshnessTestCase {
            name: "TC25: Status 301 without explicit freshness is uncacheable",
            status_code: 301,
            headers: vec![],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 26. Status 301 with max-age is cacheable
        FreshnessTestCase {
            name: "TC26: Status 301 with max-age is cacheable",
            status_code: 301,
            headers: vec![("cache-control", "max-age=300")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 300_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 27. Status 404 with max-age is cacheable
        FreshnessTestCase {
            name: "TC27: Status 404 with max-age is cacheable (negative caching)",
            status_code: 404,
            headers: vec![("cache-control", "max-age=30")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 30_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 28. Status 410 with max-age is cacheable
        FreshnessTestCase {
            name: "TC28: Status 410 with max-age is cacheable",
            status_code: 410,
            headers: vec![("cache-control", "max-age=45")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 45_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 29. Status 500 is uncacheable even with max-age
        FreshnessTestCase {
            name: "TC29: Status 500 is uncacheable even with max-age",
            status_code: 500,
            headers: vec![("cache-control", "max-age=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 30. Close-delimited response is uncacheable
        FreshnessTestCase {
            name: "TC30: Close-delimited response without explicit framing is uncacheable",
            status_code: 200,
            headers: vec![("cache-control", "max-age=60")],
            now_ms: now,
            has_explicit_framing: false,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 31. Invalid max-age directive is treated conservatively as uncacheable
        FreshnessTestCase {
            name: "TC31: Invalid max-age directive treated conservatively as uncacheable",
            status_code: 200,
            headers: vec![("cache-control", "max-age=invalid_number")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 32. Quoted max-age value
        FreshnessTestCase {
            name: "TC32: Quoted max-age=\"120\" is parsed correctly",
            status_code: 200,
            headers: vec![("cache-control", "max-age=\"120\"")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 120_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 33. Multiple Cache-Control headers combined
        FreshnessTestCase {
            name: "TC33: Multiple Cache-Control headers merged",
            status_code: 200,
            headers: vec![
                ("cache-control", "public"),
                ("cache-control", "max-age=90"),
                ("cache-control", "must-revalidate"),
            ],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 90_000,
            expected_must_reval: true,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 34. max-age=0 is cacheable with 0 lifetime
        FreshnessTestCase {
            name: "TC34: max-age=0 gives lifetime 0",
            status_code: 200,
            headers: vec![("cache-control", "max-age=0")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 35. max-age=86400 (1 day)
        FreshnessTestCase {
            name: "TC35: max-age=86400 parses 24 hours",
            status_code: 200,
            headers: vec![("cache-control", "max-age=86400")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 86_400_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 36. s-maxage=0
        FreshnessTestCase {
            name: "TC36: s-maxage=0 gives 0 lifetime",
            status_code: 200,
            headers: vec![("cache-control", "s-maxage=0")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 37. public, no-cache
        FreshnessTestCase {
            name: "TC37: public with no-cache mandates revalidation and 0 lifetime",
            status_code: 200,
            headers: vec![("cache-control", "public, no-cache")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 0,
            expected_must_reval: true,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 38. public, no-store
        FreshnessTestCase {
            name: "TC38: no-store strictly wins over public",
            status_code: 200,
            headers: vec![("cache-control", "public, no-store")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 39. private, max-age
        FreshnessTestCase {
            name: "TC39: private strictly wins over max-age in shared cache",
            status_code: 200,
            headers: vec![("cache-control", "private, max-age=3600")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 40. no-store, max-age
        FreshnessTestCase {
            name: "TC40: no-store strictly wins over max-age",
            status_code: 200,
            headers: vec![("cache-control", "no-store, max-age=3600")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 41. no-cache, max-age
        FreshnessTestCase {
            name: "TC41: no-cache overrides max-age to 0 lifetime and forces revalidation",
            status_code: 200,
            headers: vec![("cache-control", "no-cache, max-age=3600")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 0,
            expected_must_reval: true,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 42. must-revalidate and proxy-revalidate
        FreshnessTestCase {
            name: "TC42: both must-revalidate and proxy-revalidate together",
            status_code: 200,
            headers: vec![("cache-control", "must-revalidate, proxy-revalidate, max-age=120")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 120_000,
            expected_must_reval: true,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 43. stale-while-revalidate=0
        FreshnessTestCase {
            name: "TC43: stale-while-revalidate=0 parsed as 0ms",
            status_code: 200,
            headers: vec![("cache-control", "max-age=60, stale-while-revalidate=0")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: Some(0),
            expected_sie_ms: None,
        },
        // 44. stale-if-error=0
        FreshnessTestCase {
            name: "TC44: stale-if-error=0 parsed as 0ms",
            status_code: 200,
            headers: vec![("cache-control", "max-age=60, stale-if-error=0")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: Some(0),
        },
        // 45. min-fresh ignored in response
        FreshnessTestCase {
            name: "TC45: min-fresh ignored in response Cache-Control",
            status_code: 200,
            headers: vec![("cache-control", "max-age=10, min-fresh=5")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 10_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 46. only-if-cached ignored in response
        FreshnessTestCase {
            name: "TC46: only-if-cached ignored in response Cache-Control",
            status_code: 200,
            headers: vec![("cache-control", "max-age=10, only-if-cached")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 10_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 47. immutable extension ignored gracefully
        FreshnessTestCase {
            name: "TC47: immutable extension handled gracefully",
            status_code: 200,
            headers: vec![("cache-control", "max-age=10, immutable")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 10_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 48. custom-extension parsed
        FreshnessTestCase {
            name: "TC48: custom extension token ignored gracefully",
            status_code: 200,
            headers: vec![("cache-control", "max-age=10, custom-flag")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 10_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 49. Empty tokens with commas
        FreshnessTestCase {
            name: "TC49: multiple commas and empty tokens handled",
            status_code: 200,
            headers: vec![("cache-control", ",,,max-age=60,,,")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 50. Whitespace around equals sign
        FreshnessTestCase {
            name: "TC50: whitespace around equals sign in max-age",
            status_code: 200,
            headers: vec![("cache-control", "max-age = 60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 51. Whitespace around equals sign in s-maxage
        FreshnessTestCase {
            name: "TC51: whitespace around equals sign in s-maxage",
            status_code: 200,
            headers: vec![("cache-control", "s-maxage = 120")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 120_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 52. Whitespace around equals sign in stale-while-revalidate
        FreshnessTestCase {
            name: "TC52: whitespace around equals in stale-while-revalidate",
            status_code: 200,
            headers: vec![("cache-control", "max-age=60, stale-while-revalidate = 45")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: Some(45_000),
            expected_sie_ms: None,
        },
        // 53. Whitespace around equals sign in stale-if-error
        FreshnessTestCase {
            name: "TC53: whitespace around equals in stale-if-error",
            status_code: 200,
            headers: vec![("cache-control", "max-age=60, stale-if-error = 90")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: Some(90_000),
        },
        // 54. Uppercase MAX-AGE
        FreshnessTestCase {
            name: "TC54: uppercase MAX-AGE is case-insensitive",
            status_code: 200,
            headers: vec![("cache-control", "MAX-AGE=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 55. Uppercase S-MAXAGE
        FreshnessTestCase {
            name: "TC55: uppercase S-MAXAGE is case-insensitive",
            status_code: 200,
            headers: vec![("cache-control", "S-MAXAGE=120")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 120_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 56. Uppercase NO-STORE
        FreshnessTestCase {
            name: "TC56: uppercase NO-STORE is case-insensitive",
            status_code: 200,
            headers: vec![("cache-control", "NO-STORE")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 57. Uppercase NO-CACHE
        FreshnessTestCase {
            name: "TC57: uppercase NO-CACHE is case-insensitive",
            status_code: 200,
            headers: vec![("cache-control", "NO-CACHE")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 0,
            expected_must_reval: true,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 58. Uppercase PUBLIC
        FreshnessTestCase {
            name: "TC58: uppercase PUBLIC is case-insensitive",
            status_code: 200,
            headers: vec![("cache-control", "PUBLIC, MAX-AGE=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 59. Uppercase PRIVATE
        FreshnessTestCase {
            name: "TC59: uppercase PRIVATE is case-insensitive",
            status_code: 200,
            headers: vec![("cache-control", "PRIVATE")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false,
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 60. Uppercase MUST-REVALIDATE
        FreshnessTestCase {
            name: "TC60: uppercase MUST-REVALIDATE is case-insensitive",
            status_code: 200,
            headers: vec![("cache-control", "MUST-REVALIDATE, MAX-AGE=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: true,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 61. Expires without Date uses now_ms as reference base
        FreshnessTestCase {
            name: "TC61: Expires without Date uses now_ms as base",
            status_code: 200,
            headers: vec![("expires", "Sun, 06 Nov 1994 09:49:37 GMT")],
            now_ms: 784_111_777_000 + 3_600_000 - 30_000, // now is 30s before expires
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 30_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 62. Malformed Expires ignored, falls back to default TTL
        FreshnessTestCase {
            name: "TC62: Malformed Expires ignored, falls back to default TTL",
            status_code: 200,
            headers: vec![("expires", "not-a-valid-date-string")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 63. Duplicate Accept-Encoding in Vary
        FreshnessTestCase {
            name: "TC63: Duplicate Accept-Encoding in Vary handled",
            status_code: 200,
            headers: vec![("vary", "Accept-Encoding, accept-encoding"), ("cache-control", "max-age=60")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 64. Status 204 with 0 body length
        FreshnessTestCase {
            name: "TC64: Status 204 with 0 body length without explicit framing",
            status_code: 204,
            headers: vec![],
            now_ms: now,
            has_explicit_framing: false,
            body_len: 0,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: false, // 204 is not 200/301/404/410
            expected_ttl_ms: 0,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
        // 65. Empty Cache-Control header string
        FreshnessTestCase {
            name: "TC65: Empty Cache-Control header string defaults to default TTL",
            status_code: 200,
            headers: vec![("cache-control", "")],
            now_ms: now,
            has_explicit_framing: true,
            body_len: 100,
            default_ttl_ms: 60_000,
            forwarded_auth: false,
            expected_cacheable: true,
            expected_ttl_ms: 60_000,
            expected_must_reval: false,
            expected_swr_ms: None,
            expected_sie_ms: None,
        },
    ];

    for tc in test_cases {
        let headers = build_header_tuples(&tc.headers);
        let decision = evaluate_response_freshness(
            tc.status_code,
            &headers,
            tc.now_ms,
            tc.has_explicit_framing,
            tc.body_len,
            tc.default_ttl_ms,
            tc.forwarded_auth,
        );

        assert_eq!(
            decision.is_cacheable, tc.expected_cacheable,
            "Failed cacheable check for: {}",
            tc.name
        );
        if tc.expected_cacheable {
            assert_eq!(
                decision.ttl_ms, tc.expected_ttl_ms,
                "Failed TTL check for: {}",
                tc.name
            );
            assert_eq!(
                decision.must_revalidate, tc.expected_must_reval,
                "Failed must_revalidate check for: {}",
                tc.name
            );
            assert_eq!(
                decision.stale_while_revalidate_ms, tc.expected_swr_ms,
                "Failed swr check for: {}",
                tc.name
            );
            assert_eq!(
                decision.stale_if_error_ms, tc.expected_sie_ms,
                "Failed sie check for: {}",
                tc.name
            );
        }
    }
}

#[test]
fn test_m2_02_http_date_parsing_formats() {
    // 1. IMF-fixdate (RFC 1123 / RFC 7231)
    let imf = "Sun, 06 Nov 1994 08:49:37 GMT";
    let imf_ms = parse_http_date(imf).expect("Valid IMF-fixdate");
    assert_eq!(imf_ms, 784_111_777_000);

    // 2. RFC 850 format
    let rfc850 = "Sunday, 06-Nov-94 08:49:37 GMT";
    let rfc850_ms = parse_http_date(rfc850).expect("Valid RFC 850 date");
    assert_eq!(rfc850_ms, 784_111_777_000);

    // 3. ANSI C asctime() format
    let asctime = "Sun Nov  6 08:49:37 1994";
    let asctime_ms = parse_http_date(asctime).expect("Valid asctime date");
    assert_eq!(asctime_ms, 784_111_777_000);

    // All three representations of the same time MUST produce the identical epoch timestamp
    assert_eq!(imf_ms, rfc850_ms);
    assert_eq!(rfc850_ms, asctime_ms);

    // Invalid dates must return None safely without panic
    assert!(parse_http_date("invalid date string").is_none());
    assert!(parse_http_date("").is_none());
    assert!(parse_http_date("Sun, 32 Nov 1994 08:49:37 GMT").is_none()); // Day 32 invalid
    assert!(parse_http_date("Sun, 06 Xyz 1994 08:49:37 GMT").is_none()); // Month invalid
    assert!(parse_http_date("Sun, 06 Nov 1994 25:49:37 GMT").is_none()); // Hour 25 invalid
}

#[test]
fn test_m2_02_etag_matching_weak_and_strong() {
    // 1. Exact strong match
    assert!(matches_if_none_match("\"v1\"", Some("\"v1\"")));
    assert!(!matches_if_none_match("\"v1\"", Some("\"v2\"")));

    // 2. Weak ETag matches strong in If-None-Match (RFC 9110 §8.8.3.2)
    assert!(matches_if_none_match("W/\"v1\"", Some("\"v1\"")));
    assert!(matches_if_none_match("\"v1\"", Some("W/\"v1\"")));
    assert!(matches_if_none_match("W/\"v1\"", Some("W/\"v1\"")));

    // 3. Wildcard *
    assert!(matches_if_none_match("*", Some("\"any_etag\"")));
    assert!(matches_if_none_match("*", Some("W/\"any_weak_etag\"")));
    assert!(!matches_if_none_match("*", None));

    // 4. Comma-separated list of ETags
    assert!(matches_if_none_match("\"tag1\", \"tag2\", \"tag3\"", Some("\"tag2\"")));
    assert!(matches_if_none_match("\"tag1\", W/\"tag2\", \"tag3\"", Some("\"tag2\"")));
    assert!(!matches_if_none_match("\"tag1\", \"tag2\"", Some("\"tag3\"")));
}

#[test]
fn test_m2_02_if_modified_since_matching() {
    let client_date = "Sun, 06 Nov 1994 08:49:37 GMT";
    let client_ms = parse_http_date(client_date).unwrap();

    // Resource created before client condition -> NOT modified (matches condition)
    assert!(matches_if_modified_since(client_date, None, client_ms - 1000));

    // Resource created exactly at client condition -> NOT modified
    assert!(matches_if_modified_since(client_date, None, client_ms));

    // Resource created AFTER client condition -> MODIFIED (does not match)
    assert!(!matches_if_modified_since(client_date, None, client_ms + 1000));

    // Resource with explicit Last-Modified header
    let old_lm = "Sun, 06 Nov 1994 07:00:00 GMT";
    assert!(matches_if_modified_since(client_date, Some(old_lm), client_ms + 5000));

    let new_lm = "Sun, 06 Nov 1994 09:00:00 GMT";
    assert!(!matches_if_modified_since(client_date, Some(new_lm), client_ms));

    // Invalid client date string is safely ignored (returns false, forcing full GET)
    assert!(!matches_if_modified_since("not-a-date", None, client_ms));
}

#[test]
fn test_m2_02_age_monotonicity_properties() {
    let created_at_ms = 1_000_000;
    let initial_age_ms = 5_000;

    let mut prev_age = calculate_current_age_seconds(created_at_ms, initial_age_ms, created_at_ms);
    assert_eq!(prev_age, 5); // 5000ms initial age = 5s

    for advance_sec in 1..=50 {
        let now_ms = created_at_ms + advance_sec * 1000;
        let age = calculate_current_age_seconds(created_at_ms, initial_age_ms, now_ms);
        assert!(age >= prev_age, "Age must be monotonic with increasing now_ms");
        assert_eq!(age, 5 + advance_sec);
        prev_age = age;
    }
}

#[test]
fn test_m2_02_header_merge_304() {
    let mut meta = FreshnessMetadata {
        must_revalidate: false,
        stale_while_revalidate_ms: None,
        stale_if_error_ms: None,
        last_modified: Some("Sun, 06 Nov 1994 08:00:00 GMT".to_string()),
        date_ms: Some(1000),
        initial_age_ms: 0,
    };

    let update_headers = build_header_tuples(&[
        ("cache-control", "must-revalidate, stale-while-revalidate=45"),
        ("last-modified", "Sun, 06 Nov 1994 09:00:00 GMT"),
    ]);

    merge_304_headers(&mut meta, &update_headers, 2000);

    assert!(meta.must_revalidate);
    assert_eq!(meta.stale_while_revalidate_ms, Some(45_000));
    assert_eq!(meta.last_modified.unwrap(), "Sun, 06 Nov 1994 09:00:00 GMT");
}

#[test]
fn test_m2_02_cache_freshness_query_integration() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = CacheKey::new(Some("tenant1"), "/freshness/test", None);
    let now = 1000;

    let meta = FreshnessMetadata {
        must_revalidate: true,
        stale_while_revalidate_ms: Some(30_000),
        stale_if_error_ms: Some(60_000),
        last_modified: Some("Sun, 06 Nov 1994 08:49:37 GMT".to_string()),
        date_ms: Some(now),
        initial_age_ms: 0,
    };

    // First initiate atomic fetch so KeyState placeholder is registered
    let _reg = cache.begin_fetch_atomic(&key, now);

    cache.update_freshness_metadata(&key, meta.clone());

    let queried = cache.get_freshness(&key);
    assert_eq!(queried, Some(meta));

    // Delete clears freshness
    cache.delete(&key);
    assert_eq!(cache.get_freshness(&key), None);
}
