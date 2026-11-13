//! RFC 9111 HTTP Caching Freshness Engine
//!
//! Authoritative implementation for Phase 2 (M2-02).
//! Pure, std-only, total-function parser and freshness evaluator.
//! Never panics on any input.

use axum::http::{header::HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};

/// Parsed HTTP Cache-Control directives for response caching per RFC 9111 §5.2.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CacheControlDirectives {
    pub no_store: bool,
    pub no_cache: bool,
    pub private: bool,
    pub public: bool,
    pub max_age_seconds: Option<u64>,
    pub s_maxage_seconds: Option<u64>,
    pub must_revalidate: bool,
    pub proxy_revalidate: bool,
    pub stale_while_revalidate_seconds: Option<u64>,
    pub stale_if_error_seconds: Option<u64>,
    pub has_invalid_directive: bool,
}

impl CacheControlDirectives {
    /// Parse Cache-Control header values into structured directives.
    /// Total function: never panics, handles arbitrary whitespace, quotes, and commas.
    pub fn parse(header_values: &[&str]) -> Self {
        let mut directives = Self::default();

        for header in header_values {
            for token in header.split(',') {
                let token = token.trim();
                if token.is_empty() {
                    continue;
                }

                if let Some((name, val)) = token.split_once('=') {
                    let name = name.trim();
                    let val = val.trim().trim_matches('"');

                    match name.to_ascii_lowercase().as_str() {
                        "max-age" => {
                            if let Ok(secs) = val.parse::<u64>() {
                                directives.max_age_seconds = Some(secs);
                            } else {
                                directives.has_invalid_directive = true;
                            }
                        }
                        "s-maxage" => {
                            if let Ok(secs) = val.parse::<u64>() {
                                directives.s_maxage_seconds = Some(secs);
                            } else {
                                directives.has_invalid_directive = true;
                            }
                        }
                        "stale-while-revalidate" => {
                            if let Ok(secs) = val.parse::<u64>() {
                                directives.stale_while_revalidate_seconds = Some(secs);
                            } else {
                                directives.has_invalid_directive = true;
                            }
                        }
                        "stale-if-error" => {
                            if let Ok(secs) = val.parse::<u64>() {
                                directives.stale_if_error_seconds = Some(secs);
                            } else {
                                directives.has_invalid_directive = true;
                            }
                        }
                        _ => {
                            // Unrecognized parameter directive
                        }
                    }
                } else {
                    match token.to_ascii_lowercase().as_str() {
                        "no-store" => directives.no_store = true,
                        "no-cache" => directives.no_cache = true,
                        "private" => directives.private = true,
                        "public" => directives.public = true,
                        "must-revalidate" => directives.must_revalidate = true,
                        "proxy-revalidate" => directives.proxy_revalidate = true,
                        _ => {
                            // Unrecognized flag directive
                        }
                    }
                }
            }
        }

        directives
    }
}

/// Metadata stored alongside a cached resource to support RFC 9111 revalidation & stale serving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshnessMetadata {
    pub must_revalidate: bool,
    pub stale_while_revalidate_ms: Option<u64>,
    pub stale_if_error_ms: Option<u64>,
    pub last_modified: Option<String>,
    pub date_ms: Option<u64>,
    pub initial_age_ms: u64,
}

/// Freshness evaluation outcome for an HTTP origin response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshnessDecision {
    pub is_cacheable: bool,
    pub ttl_ms: u64,
    pub must_revalidate: bool,
    pub stale_while_revalidate_ms: Option<u64>,
    pub stale_if_error_ms: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub initial_age_ms: u64,
    pub date_ms: Option<u64>,
    pub reason: &'static str,
}

/// Evaluates response cacheability and freshness lifetime according to RFC 9111.
///
/// Rules applied:
/// 1. Close-delimited responses without explicit framing are uncacheable.
/// 2. `no-store` or `private` => uncacheable.
/// 3. Invalid `Cache-Control` directives => uncacheable (conservative RFC 9111 §4.2.1).
/// 4. Requests with `Authorization` or responses with `Set-Cookie` are only cacheable
///    if explicitly shared-cacheable (`public`, `s-maxage`, or `must-revalidate`).
/// 5. `Vary: *` or any `Vary` header other than normalized `accept-encoding` => uncacheable.
/// 6. Stored status codes: 200 by default; 301, 404, 410 only with explicit freshness information;
///    all others uncacheable.
/// 7. Lifetime calculation priority:
///    a. `s-maxage` (shared cache precedence)
///    b. `max-age`
///    c. `Expires` - `Date` (or `Expires` - `now_ms` if Date missing)
///    d. `no-cache` => lifetime = 0 (requires conditional revalidation)
///    e. Default TTL (`default_ttl_ms`, default 60,000 ms per O2-1).
pub fn evaluate_response_freshness(
    status_code: u16,
    headers: &[(Option<HeaderName>, HeaderValue)],
    now_ms: u64,
    has_explicit_framing: bool,
    body_len: usize,
    default_ttl_ms: u64,
    forwarded_authorization: bool,
) -> FreshnessDecision {
    // 1. Framing check: close-delimited responses must not be cached
    if body_len > 0 && !has_explicit_framing {
        return FreshnessDecision::uncacheable("Missing explicit framing (close-delimited)");
    }

    // Extract relevant header strings
    let mut cc_values = Vec::new();
    let mut etag = None;
    let mut last_modified = None;
    let mut date_str = None;
    let mut expires_str = None;
    let mut age_seconds = None;
    let mut has_set_cookie = false;
    let mut vary_headers = Vec::new();

    for (k, v) in headers {
        if let Some(name) = k {
            let n = name.as_str();
            if n.eq_ignore_ascii_case("cache-control") {
                if let Ok(s) = v.to_str() {
                    cc_values.push(s);
                }
            } else if n.eq_ignore_ascii_case("etag") {
                if let Ok(s) = v.to_str() {
                    etag = Some(s.trim().to_string());
                }
            } else if n.eq_ignore_ascii_case("last-modified") {
                if let Ok(s) = v.to_str() {
                    last_modified = Some(s.trim().to_string());
                }
            } else if n.eq_ignore_ascii_case("date") {
                if let Ok(s) = v.to_str() {
                    date_str = Some(s.trim());
                }
            } else if n.eq_ignore_ascii_case("expires") {
                if let Ok(s) = v.to_str() {
                    expires_str = Some(s.trim());
                }
            } else if n.eq_ignore_ascii_case("age") {
                if let Ok(s) = v.to_str() {
                    if let Ok(val) = s.trim().parse::<u64>() {
                        age_seconds = Some(val);
                    }
                }
            } else if n.eq_ignore_ascii_case("set-cookie") {
                has_set_cookie = true;
            } else if n.eq_ignore_ascii_case("vary") {
                if let Ok(s) = v.to_str() {
                    vary_headers.push(s);
                }
            }
        }
    }

    // Parse Cache-Control directives
    let cc = CacheControlDirectives::parse(&cc_values);

    // 2. Directives that strictly forbid storage in a shared cache
    if cc.no_store {
        return FreshnessDecision::uncacheable("Cache-Control: no-store");
    }
    if cc.private {
        return FreshnessDecision::uncacheable("Cache-Control: private");
    }
    if cc.has_invalid_directive {
        return FreshnessDecision::uncacheable("Invalid Cache-Control directive");
    }

    // 3. Status code check
    let has_explicit_freshness = cc.max_age_seconds.is_some()
        || cc.s_maxage_seconds.is_some()
        || expires_str.is_some();

    let status_cacheable = match status_code {
        200 => true,
        301 | 404 | 410 => has_explicit_freshness,
        _ => false,
    };

    if !status_cacheable {
        return FreshnessDecision::uncacheable("Status code not cacheable");
    }

    // 4. Authorization / Set-Cookie restrictions
    let is_explicitly_public = cc.public || cc.s_maxage_seconds.is_some() || cc.must_revalidate;
    if (forwarded_authorization || has_set_cookie) && !is_explicitly_public {
        return FreshnessDecision::uncacheable("Authorization/Set-Cookie without explicit public sharing");
    }

    // 5. Vary header check (RFC 9111 §4.1)
    for vary in &vary_headers {
        for part in vary.split(',') {
            let part = part.trim();
            if part == "*" {
                return FreshnessDecision::uncacheable("Vary: *");
            }
            if !part.is_empty() && !part.eq_ignore_ascii_case("accept-encoding") {
                return FreshnessDecision::uncacheable("Vary header outside supported subset (Accept-Encoding)");
            }
        }
    }

    // 6. Dates and Age calculation per RFC 9111 §4.2.3
    let date_ms = date_str.and_then(parse_http_date);
    let expires_ms = expires_str.and_then(parse_http_date);

    let apparent_age_ms = if let Some(d) = date_ms {
        now_ms.saturating_sub(d)
    } else {
        0
    };
    let age_header_ms = age_seconds.unwrap_or(0).saturating_mul(1000);
    let initial_age_ms = apparent_age_ms.max(age_header_ms);

    // 7. Freshness lifetime calculation
    let (ttl_ms, must_revalidate) = if cc.no_cache {
        (0, true) // no-cache forces freshness lifetime to 0 and mandates revalidation
    } else if let Some(s_max) = cc.s_maxage_seconds {
        (s_max.saturating_mul(1000), cc.must_revalidate || cc.proxy_revalidate)
    } else if let Some(max_age) = cc.max_age_seconds {
        (max_age.saturating_mul(1000), cc.must_revalidate || cc.proxy_revalidate)
    } else if let Some(exp) = expires_ms {
        let base_date = date_ms.unwrap_or(now_ms);
        let lifetime = exp.saturating_sub(base_date);
        (lifetime, cc.must_revalidate || cc.proxy_revalidate)
    } else {
        // Fallback to configured default TTL
        (default_ttl_ms, cc.must_revalidate || cc.proxy_revalidate)
    };

    let stale_while_revalidate_ms = cc.stale_while_revalidate_seconds.map(|s| s.saturating_mul(1000));
    let stale_if_error_ms = cc.stale_if_error_seconds.map(|s| s.saturating_mul(1000));

    FreshnessDecision {
        is_cacheable: true,
        ttl_ms,
        must_revalidate,
        stale_while_revalidate_ms,
        stale_if_error_ms,
        etag,
        last_modified,
        initial_age_ms,
        date_ms,
        reason: "Cacheable per RFC 9111",
    }
}

/// Result of combining a stored response with a `304 Not Modified` (RFC 9111 section 4.3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotModifiedMerge {
    /// Refresh the stored entry in place with these values.
    Refresh { ttl_ms: u64, etag: Option<String>, meta: FreshnessMetadata },
    /// The 304 carries a validator that differs from the stored one: it must not refresh (or
    /// relabel) the stored body.
    ValidatorMismatch,
    /// The 304's headers forbid storage (no-store, private, invalid Cache-Control, ...).
    Uncacheable,
}

fn header_present(headers: &[(Option<HeaderName>, HeaderValue)], name: &str) -> bool {
    headers
        .iter()
        .any(|(k, _)| k.as_ref().is_some_and(|n| n.as_str().eq_ignore_ascii_case(name)))
}

/// Merges a 304's headers into the stored entry's freshness state. Only headers actually present
/// in the 304 replace stored values; everything else (stale-while-revalidate, stale-if-error,
/// must-revalidate, Last-Modified, lifetime) is retained. A differing validator yields
/// `ValidatorMismatch` (weak comparison: the `W/` prefix is ignored).
pub fn merge_not_modified(
    stored_ttl_ms: u64,
    stored_etag: Option<&str>,
    stored_meta: Option<&FreshnessMetadata>,
    headers_304: &[(Option<HeaderName>, HeaderValue)],
    now_ms: u64,
    forwarded_authorization: bool,
) -> NotModifiedMerge {
    let d = evaluate_response_freshness(200, headers_304, now_ms, true, 0, stored_ttl_ms, forwarded_authorization);
    if !d.is_cacheable {
        return NotModifiedMerge::Uncacheable;
    }
    if let (Some(new), Some(old)) = (d.etag.as_deref(), stored_etag) {
        let strip = |s: &str| s.trim().trim_start_matches("W/").to_string();
        if strip(new) != strip(old) {
            return NotModifiedMerge::ValidatorMismatch;
        }
    }
    let has_cc = header_present(headers_304, "cache-control");
    let has_expires = header_present(headers_304, "expires");
    let ttl_ms = if has_cc || has_expires { d.ttl_ms } else { stored_ttl_ms };
    let (must_revalidate, swr, sie) = if has_cc {
        (d.must_revalidate, d.stale_while_revalidate_ms, d.stale_if_error_ms)
    } else {
        stored_meta.map_or((false, None, None), |m| {
            (m.must_revalidate, m.stale_while_revalidate_ms, m.stale_if_error_ms)
        })
    };
    let last_modified = d
        .last_modified
        .clone()
        .or_else(|| stored_meta.and_then(|m| m.last_modified.clone()));
    NotModifiedMerge::Refresh {
        ttl_ms,
        etag: d.etag.clone().or_else(|| stored_etag.map(|s| s.to_string())),
        meta: FreshnessMetadata {
            must_revalidate,
            stale_while_revalidate_ms: swr,
            stale_if_error_ms: sie,
            last_modified,
            date_ms: d.date_ms,
            initial_age_ms: d.initial_age_ms,
        },
    }
}

impl FreshnessDecision {
    pub fn uncacheable(reason: &'static str) -> Self {
        Self {
            is_cacheable: false,
            ttl_ms: 0,
            must_revalidate: false,
            stale_while_revalidate_ms: None,
            stale_if_error_ms: None,
            etag: None,
            last_modified: None,
            initial_age_ms: 0,
            date_ms: None,
            reason,
        }
    }
}

/// Parses standard HTTP date strings into epoch milliseconds.
///
/// Supports:
/// 1. IMF-fixdate (RFC 7231 / RFC 9110): `Sun, 06 Nov 1994 08:49:37 GMT`
/// 2. RFC 850 format: `Sunday, 06-Nov-94 08:49:37 GMT`
/// 3. ANSI C asctime() format: `Sun Nov  6 08:49:37 1994`
///
/// Total function: pure std, no external crates, never panics.
pub fn parse_http_date(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.len() < 10 {
        return None;
    }

    // Try IMF-fixdate / RFC 1123: "Sun, 06 Nov 1994 08:49:37 GMT"
    if let Some(pos) = s.find(',') {
        let remainder = s[pos + 1..].trim();
        let parts: Vec<&str> = remainder.split_whitespace().collect();
        if parts.len() >= 3 {
            // Check if parts[0] is day or day-month-year
            if let Some((day_str, month_str, year_str)) = parse_date_part(parts[0]) {
                // RFC 850 format: "06-Nov-94"
                let time_part = parts.get(1)?;
                return parse_datetime_parts(&day_str, &month_str, &year_str, time_part);
            } else if parts.len() >= 4 {
                // IMF-fixdate: parts[0] = day, parts[1] = month, parts[2] = year, parts[3] = time
                let day_str = parts[0];
                let month_str = parts[1];
                let year_str = parts[2];
                let time_str = parts[3];
                return parse_datetime_parts(day_str, month_str, year_str, time_str);
            }
        }
    }

    // Try ANSI C asctime(): "Sun Nov  6 08:49:37 1994"
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() == 5 {
        // parts[0] = "Sun", parts[1] = "Nov", parts[2] = "6", parts[3] = "08:49:37", parts[4] = "1994"
        let month_str = parts[1];
        let day_str = parts[2];
        let time_str = parts[3];
        let year_str = parts[4];
        return parse_datetime_parts(day_str, month_str, year_str, time_str);
    }

    None
}

fn parse_date_part(s: &str) -> Option<(String, String, String)> {
    let subparts: Vec<&str> = s.split('-').collect();
    if subparts.len() == 3 {
        Some((
            subparts[0].to_string(),
            subparts[1].to_string(),
            subparts[2].to_string(),
        ))
    } else {
        None
    }
}

fn parse_month_name(m: &str) -> Option<u32> {
    match m.to_ascii_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

fn parse_datetime_parts(
    day_str: &str,
    month_str: &str,
    year_str: &str,
    time_str: &str,
) -> Option<u64> {
    let day = day_str.parse::<u32>().ok()?;
    let month = parse_month_name(month_str)?;
    let mut year = year_str.parse::<i64>().ok()?;

    // Handle 2-digit years in RFC 850
    if year < 70 {
        year += 2000;
    } else if year < 100 {
        year += 1900;
    }

    let time_parts: Vec<&str> = time_str.split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour = time_parts[0].parse::<u64>().ok()?;
    let minute = time_parts[1].parse::<u64>().ok()?;
    let second = time_parts[2].parse::<u64>().ok()?;

    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }

    let days = days_from_civil(year, month, day)?;
    if days < 0 {
        return None;
    }

    let total_secs = (days as u64)
        .saturating_mul(86_400)
        .saturating_add(hour.saturating_mul(3_600))
        .saturating_add(minute.saturating_mul(60))
        .saturating_add(second);

    Some(total_secs.saturating_mul(1000))
}

/// Howard Hinnant algorithm: returns days since Unix Epoch (1970-01-01).
fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + (doe as i64) - 719_468)
}

/// Calculates current age in seconds according to RFC 9111 §4.2.3.
pub fn calculate_current_age_seconds(created_at_ms: u64, initial_age_ms: u64, now_ms: u64) -> u64 {
    let resident_ms = now_ms.saturating_sub(created_at_ms);
    let total_age_ms = initial_age_ms.saturating_add(resident_ms);
    total_age_ms / 1000
}

/// RFC 9110 §8.8.3.2 Weak ETag matching for `If-None-Match`.
///
/// Weak comparison:
/// - Strips `W/` prefix from either side before comparing opaque tag strings.
/// - Handles comma-separated list of tags in client request.
/// - Matches `*` to any existing entity tag.
pub fn matches_if_none_match(client_if_none_match: &str, cached_etag: Option<&str>) -> bool {
    let cached = match cached_etag {
        Some(e) => normalize_etag(e),
        None => return false,
    };

    let client_trimmed = client_if_none_match.trim();
    if client_trimmed == "*" {
        return true;
    }

    for tag in client_trimmed.split(',') {
        let tag = tag.trim();
        if tag == "*" {
            return true;
        }
        if normalize_etag(tag) == cached {
            return true;
        }
    }

    false
}

fn normalize_etag(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("W/").or_else(|| s.strip_prefix("w/")) {
        rest.trim()
    } else {
        s
    }
}

/// Evaluates `If-Modified-Since` conditional header per RFC 9110 §13.1.3.
/// Returns `true` if resource is NOT modified (i.e. cached resource is older than or equal to client timestamp).
pub fn matches_if_modified_since(
    client_if_modified_since: &str,
    cached_last_modified: Option<&str>,
    cached_created_at_ms: u64,
) -> bool {
    let client_ms = match parse_http_date(client_if_modified_since) {
        Some(ms) => ms,
        None => return false, // Invalid date => ignore header
    };

    let resource_ms = if let Some(lm) = cached_last_modified {
        match parse_http_date(lm) {
            Some(ms) => ms,
            None => cached_created_at_ms,
        }
    } else {
        cached_created_at_ms
    };

    // If resource time <= client condition, the resource has NOT been modified since client's copy
    resource_ms <= client_ms
}

/// RFC 9111 §4.3.4: 304 Not Modified header merge.
/// Updates freshness headers while keeping resource body intact.
pub fn merge_304_headers(
    existing_metadata: &mut FreshnessMetadata,
    new_headers: &[(Option<HeaderName>, HeaderValue)],
    now_ms: u64,
) {
    let mut cc_values = Vec::new();
    let mut date_str = None;
    let mut age_seconds = None;

    for (k, v) in new_headers {
        if let Some(name) = k {
            let n = name.as_str();
            if n.eq_ignore_ascii_case("cache-control") {
                if let Ok(s) = v.to_str() {
                    cc_values.push(s);
                }
            } else if n.eq_ignore_ascii_case("date") {
                if let Ok(s) = v.to_str() {
                    date_str = Some(s.trim());
                }
            } else if n.eq_ignore_ascii_case("age") {
                if let Ok(s) = v.to_str() {
                    if let Ok(val) = s.trim().parse::<u64>() {
                        age_seconds = Some(val);
                    }
                }
            } else if n.eq_ignore_ascii_case("last-modified") {
                if let Ok(s) = v.to_str() {
                    existing_metadata.last_modified = Some(s.trim().to_string());
                }
            }
        }
    }

    let cc = CacheControlDirectives::parse(&cc_values);
    if cc.must_revalidate || cc.proxy_revalidate {
        existing_metadata.must_revalidate = true;
    }
    if let Some(swr) = cc.stale_while_revalidate_seconds {
        existing_metadata.stale_while_revalidate_ms = Some(swr.saturating_mul(1000));
    }
    if let Some(sie) = cc.stale_if_error_seconds {
        existing_metadata.stale_if_error_ms = Some(sie.saturating_mul(1000));
    }

    if let Some(d) = date_str.and_then(parse_http_date) {
        existing_metadata.date_ms = Some(d);
        let apparent_age = now_ms.saturating_sub(d);
        let age_val = age_seconds.unwrap_or(0).saturating_mul(1000);
        existing_metadata.initial_age_ms = apparent_age.max(age_val);
    }
}
