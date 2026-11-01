use phantom_cache_demo::api::server::{decode_chunked_body, parse_http_response};
use phantom_cache_demo::core::cache::CacheKey;
use phantom_cache_demo::core::coalesce::OriginError;
use proptest::prelude::*;

// ============================================================================
// 1. TABLE-DRIVEN ADVERSARIAL HTTP/1.1 PARSER CONFORMANCE TESTS
// ============================================================================

#[test]
fn test_parser_valid_standard_response() {
    let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 12\r\n\r\nHello World!";
    let (status, headers, body) = parse_http_response(raw).expect("Valid response must parse");
    assert_eq!(status, 200);
    assert_eq!(headers.len(), 2);
    assert_eq!(body, b"Hello World!");
}

#[test]
fn test_parser_valid_chunked_stream_with_extensions() {
    let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5;ext=foo;bar=1\r\nhello\r\n6;ext2\r\n world\r\n0;final=true\r\n\r\n";
    let (status, _, body) = parse_http_response(raw).expect("Valid chunked stream must parse");
    assert_eq!(status, 200);
    assert_eq!(body, b"hello world");
}

#[test]
fn test_parser_conflicting_content_length_rejected() {
    // Smuggling vector: two conflicting Content-Length headers
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nContent-Length: 20\r\n\r\n1234567890";
    let err = parse_http_response(raw).unwrap_err();
    assert!(
        matches!(err, OriginError::SecurityError(_)),
        "Conflicting Content-Length must be rejected as SecurityError, got {:?}",
        err
    );
}

#[test]
fn test_parser_comma_separated_conflicting_content_length_rejected() {
    // Smuggling vector: comma-separated conflicting Content-Length
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 10, 20\r\n\r\n1234567890";
    let err = parse_http_response(raw).unwrap_err();
    assert!(matches!(err, OriginError::SecurityError(_)));
}

#[test]
fn test_parser_identical_duplicate_content_length_accepted() {
    // RFC 9112 §5.1: Duplicate Content-Length with same value is valid
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Length: 5\r\n\r\nhello";
    let (status, _, body) = parse_http_response(raw).expect("Duplicate identical CL must parse");
    assert_eq!(status, 200);
    assert_eq!(body, b"hello");
}

#[test]
fn test_parser_conflicting_cl_and_transfer_encoding_chunked_rejected() {
    // RFC 9112 §6.1 smuggling vector: Both CL and chunked
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
    let err = parse_http_response(raw).unwrap_err();
    assert!(
        matches!(err, OriginError::SecurityError(_)),
        "Both CL and TE chunked must be rejected as SecurityError, got {:?}",
        err
    );
}

#[test]
fn test_parser_unsupported_transfer_encoding_rejected() {
    let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\ncompressed_data";
    let err = parse_http_response(raw).unwrap_err();
    assert!(matches!(err, OriginError::SecurityError(_)));
}

#[test]
fn test_parser_invalid_hex_chunk_size_rejected() {
    let raw_chunk = b"ZZ\r\ninvalid_chunk\r\n0\r\n\r\n";
    let err = decode_chunked_body(raw_chunk, 1024 * 1024).unwrap_err();
    assert!(matches!(err, OriginError::SecurityError(_)));
}

#[test]
fn test_parser_overflow_chunk_size_rejected() {
    // Extreme 64-bit integer overflow attempt in hex chunk size
    let raw_chunk = b"FFFFFFFFFFFFFFFF\r\ndata\r\n0\r\n\r\n";
    let err = decode_chunked_body(raw_chunk, 1024 * 1024).unwrap_err();
    assert!(matches!(err, OriginError::ResponseTooLarge | OriginError::SecurityError(_)));
}

#[test]
fn test_parser_empty_chunk_size_line_rejected() {
    let raw_chunk = b"\r\n\r\n0\r\n\r\n";
    let err = decode_chunked_body(raw_chunk, 1024 * 1024).unwrap_err();
    assert!(matches!(err, OriginError::SecurityError(_)));
}

#[test]
fn test_parser_truncated_chunk_data_rejected() {
    // Chunk declares 10 bytes (0xA), but only 5 provided before EOF
    let raw_chunk = b"A\r\n12345";
    let err = decode_chunked_body(raw_chunk, 1024 * 1024).unwrap_err();
    assert!(matches!(err, OriginError::SecurityError(_)));
}

#[test]
fn test_parser_missing_crlf_after_chunk_data_rejected() {
    // Missing CRLF after chunk data
    let raw_chunk = b"5\r\nhelloXX0\r\n\r\n";
    let err = decode_chunked_body(raw_chunk, 1024 * 1024).unwrap_err();
    assert!(matches!(err, OriginError::SecurityError(_)));
}

#[test]
fn test_parser_truncated_response_content_length() {
    // Header claims 20 bytes, body only has 10
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\n1234567890";
    let err = parse_http_response(raw).unwrap_err();
    assert!(matches!(err, OriginError::SecurityError(_)));
}

#[test]
fn test_parser_oversize_header_rejected() {
    // Header section > 64 KiB (MAX_HEADER_BYTES)
    let mut raw = Vec::new();
    raw.extend_from_slice(b"HTTP/1.1 200 OK\r\n");
    for i in 0..1200 {
        raw.extend_from_slice(format!("X-Custom-Header-{}: long_value_padding_for_overflow_{}\r\n", i, i).as_bytes());
    }
    raw.extend_from_slice(b"\r\nBody");

    assert!(raw.len() > 64 * 1024);
    let err = parse_http_response(&raw).unwrap_err();
    assert!(matches!(err, OriginError::ResponseTooLarge));
}

#[test]
fn test_parser_incomplete_response_missing_delimiter() {
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nIncomplete-Header";
    let err = parse_http_response(raw).unwrap_err();
    assert!(matches!(err, OriginError::NetworkError(_)));
}

// ============================================================================
// 2. INGRESS TENANT, PATH, AND QUERY BOUNDARY VALIDATION
// ============================================================================

#[test]
fn test_ingress_tenant_boundary_validation() {
    // Tenant ID <= 128 bytes is valid
    let valid_tenant = "a".repeat(128);
    let key = CacheKey::new(Some(&valid_tenant), "/test", None);
    assert!(key.validate().is_ok());

    // Tenant ID 129 bytes exceeds limit
    let invalid_tenant = "a".repeat(129);
    let invalid_key = CacheKey::new(Some(&invalid_tenant), "/test", None);
    assert!(matches!(invalid_key.validate().unwrap_err(), OriginError::TenantIdTooLong));
}

#[test]
fn test_ingress_path_boundary_validation() {
    // Path <= 2048 bytes is valid
    let valid_path = format!("/{}", "p".repeat(2047));
    let key = CacheKey::new(Some("t1"), &valid_path, None);
    assert!(key.validate().is_ok());

    // Path > 2048 bytes exceeds limit
    let invalid_path = format!("/{}", "p".repeat(2048));
    let invalid_key = CacheKey::new(Some("t1"), &invalid_path, None);
    assert!(matches!(invalid_key.validate().unwrap_err(), OriginError::UriTooLong));
}

#[test]
fn test_ingress_query_boundary_validation() {
    // M1: the canonical key (tenant:path:?query) is capped at 4096 bytes (cluster frame cap), which
    // is tighter than MAX_QUERY_LEN for any real tenant/path. 'cap - 10' bytes of query still fit
    // for tenant "t1" + path "/path" (canonical overhead 2+1+5+2); a 4096-byte query no longer does.
    let valid_query = "q".repeat(4096 - 10);
    let key = CacheKey::new(Some("t1"), "/path", Some(&valid_query));
    assert!(key.validate().is_ok());
    let over_cap_query = "q".repeat(4096);
    assert!(matches!(
        CacheKey::new(Some("t1"), "/path", Some(&over_cap_query)).validate().unwrap_err(),
        OriginError::UriTooLong
    ));

    // Query > 4096 bytes exceeds limit
    let invalid_query = "q".repeat(4097);
    let invalid_key = CacheKey::new(Some("t1"), "/path", Some(&invalid_query));
    assert!(matches!(invalid_key.validate().unwrap_err(), OriginError::UriTooLong));
}

// ============================================================================
// 3. PROPTEST PROPERTY-BASED ROBUSTNESS & NON-PANIC GUARANTEES
// ============================================================================

proptest! {
    // Fuzz test arbitrary byte sequences fed to parse_http_response: MUST NEVER PANIC
    #[test]
    fn test_proptest_arbitrary_bytes_no_panic(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        let _ = parse_http_response(&bytes);
    }

    // Fuzz test arbitrary byte sequences fed to decode_chunked_body: MUST NEVER PANIC
    #[test]
    fn test_proptest_chunked_decoder_arbitrary_bytes_no_panic(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        let _ = decode_chunked_body(&bytes, 64 * 1024);
    }

    // Structure-aware generator: valid chunks always reconstruct exact payload
    #[test]
    fn test_proptest_valid_chunk_roundtrip(chunks in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..500), 1..10)) {
        let mut wire = Vec::new();
        let mut expected = Vec::new();

        for chunk in &chunks {
            let chunk_header = format!("{:X}\r\n", chunk.len());
            wire.extend_from_slice(chunk_header.as_bytes());
            wire.extend_from_slice(chunk);
            wire.extend_from_slice(b"\r\n");
            expected.extend_from_slice(chunk);
        }
        // Terminal chunk
        wire.extend_from_slice(b"0\r\n\r\n");

        let decoded = decode_chunked_body(&wire, 1024 * 1024).expect("Valid chunks must decode");
        prop_assert_eq!(decoded, expected);
    }
}
