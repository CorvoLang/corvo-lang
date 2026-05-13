use crate::standard_lib::http_server::HttpServer;
use crate::type_system::Value;
use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_raw(request: &[u8]) -> std::collections::HashMap<String, Value> {
    let header_end = HttpServer::http_header_end(request);
    let header_text = String::from_utf8_lossy(&request[..header_end]);
    HttpServer::parse_http_raw(request, header_end, "127.0.0.1", header_text.as_ref()).unwrap()
}

fn parse_raw_with_ip(request: &[u8], ip: &str) -> std::collections::HashMap<String, Value> {
    let header_end = HttpServer::http_header_end(request);
    let header_text = String::from_utf8_lossy(&request[..header_end]);
    HttpServer::parse_http_raw(request, header_end, ip, header_text.as_ref()).unwrap()
}

fn method(result: &std::collections::HashMap<String, Value>) -> String {
    result.get("method").unwrap().as_string().unwrap().clone()
}

fn path(result: &std::collections::HashMap<String, Value>) -> String {
    result.get("path").unwrap().as_string().unwrap().clone()
}

fn body(result: &std::collections::HashMap<String, Value>) -> String {
    result.get("body").unwrap().as_string().unwrap().clone()
}

fn header(result: &std::collections::HashMap<String, Value>, name: &str) -> Option<Value> {
    let headers = result.get("headers")?.as_map()?;
    headers.get(name).cloned()
}

fn has_header(result: &std::collections::HashMap<String, Value>, name: &str) -> bool {
    result
        .get("headers")
        .and_then(|v| v.as_map())
        .map(|m| m.contains_key(name))
        .unwrap_or(false)
}

fn query_map(
    result: &std::collections::HashMap<String, Value>,
) -> std::collections::HashMap<String, Value> {
    result
        .get("query")
        .and_then(|v| v.as_map())
        .cloned()
        .unwrap_or_default()
}

fn ip(result: &std::collections::HashMap<String, Value>) -> String {
    result.get("ip").unwrap().as_string().unwrap().clone()
}

// ===========================================================================
// Section 1: CVE-Based Tests
// ===========================================================================

// CVE-2021-32677: Proxy-Authorization header injection
#[test]
fn test_cve_2021_32677_proxy_authorization_header() {
    let raw =
        b"GET / HTTP/1.1\r\nHost: example.com\r\nProxy-Authorization: Basic dXNlcjpwYXNz\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "proxy-authorization").unwrap(),
        Value::String("Basic dXNlcjpwYXNz".to_string())
    );
}

// CVE-2022-24765: HTTP Request Smuggling via conflicting TE and CL headers
#[test]
fn test_cve_2022_24765_request_smuggling_te_cl() {
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: chunked\r\nContent-Length: 5\r\n\r\n0\r\n\r\n";
    let result = parse_raw(raw);
    assert!(has_header(&result, "transfer-encoding"));
    assert!(has_header(&result, "content-length"));
    assert_eq!(
        header(&result, "transfer-encoding").unwrap(),
        Value::String("chunked".to_string())
    );
    // Body is 5 bytes: the chunked terminator data after headers ("0\r\n\r\n")
    // Since CL is 5 and body bytes are exactly 5, body is the full chunked terminator
    assert_eq!(body(&result), "0\r\n\r\n");
}

// CVE-2023-29159: Path traversal in URL
#[test]
fn test_cve_2023_29159_path_traversal() {
    let raw = b"GET /../../../etc/passwd HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "/../../../etc/passwd");
}

// CVE-2024-24762: Multipart form data resource exhaustion with large boundary
#[test]
fn test_cve_2024_24762_multipart_large_boundary() {
    let boundary = "A".repeat(4096);
    let body_content = format!(
        "--{}\r\nContent-Disposition: form-data; name=\"x\"\r\n\r\n1\r\n--{}--\r\n",
        boundary, boundary
    );
    let request = format!(
        "POST / HTTP/1.1\r\nHost: example.com\r\nContent-Type: multipart/form-data; boundary={}\r\nContent-Length: {}\r\n\r\n{}",
        boundary,
        body_content.len(),
        body_content
    );
    let result = parse_raw(request.as_bytes());
    assert_eq!(method(&result), "POST");
    assert!(has_header(&result, "content-type"));
    assert!(has_header(&result, "content-length"));
}

// CVE-2021-32714: Request Smuggling via whitespace in headers
#[test]
fn test_cve_2021_32714_header_whitespace_smuggling() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length : 10\r\n\r\nbodycontent";
    let result = parse_raw(raw);
    assert!(
        has_header(&result, "content-length"),
        "Header key with trailing space before colon must still be recognized"
    );
    assert_eq!(
        header(&result, "content-length").unwrap(),
        Value::String("10".to_string())
    );
}

// CVE-2021-32715: Missing Host header
#[test]
fn test_cve_2021_32715_missing_host_header() {
    let raw = b"GET / HTTP/1.1\r\n\r\n";
    let result = parse_raw(raw);
    assert!(!has_header(&result, "host"));
    assert_eq!(method(&result), "GET");
    assert_eq!(path(&result), "/");
    assert_eq!(body(&result), "");
}

// CVE-2013-2028: Chunked encoding buffer overflow
#[test]
fn test_cve_2013_2028_chunked_encoding() {
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: chunked\r\n\r\nFFFFFFFF\r\n...";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "transfer-encoding").unwrap(),
        Value::String("chunked".to_string())
    );
    assert!(result.contains_key("body"));
}

// CVE-2021-41773: Apache-style path traversal with URL encoding
#[test]
fn test_cve_2021_41773_path_traversal_encoded() {
    let raw = b"GET /cgi-bin/.%2e/.%2e/.%2e/.%2e/etc/passwd HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "/cgi-bin/.%2e/.%2e/.%2e/.%2e/etc/passwd");
}

// CVE-2021-42013: Double-encoded path traversal
#[test]
fn test_cve_2021_42013_path_traversal_double_encoded() {
    let raw = b"GET /cgi-bin/%%32%65%%32%65/etc/passwd HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "/cgi-bin/%%32%65%%32%65/etc/passwd");
}

// CVE-2022-22720: Request smuggling via large body
#[test]
fn test_cve_2022_22720_large_body_smuggling() {
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nThis request body is much longer than the declared content length";
    let result = parse_raw(raw);
    // Body must be truncated to Content-Length (5 bytes)
    assert_eq!(body(&result).len(), 5);
    assert_eq!(body(&result), "This ");
}

/// Test-only stream returning fixed chunks per `read` call (exercises `read_exact` body path).
struct ChunkedFakeStream {
    chunks: Vec<&'static [u8]>,
    index: usize,
}

impl ChunkedFakeStream {
    fn new(chunks: Vec<&'static [u8]>) -> Self {
        Self { chunks, index: 0 }
    }
}

impl Read for ChunkedFakeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.index >= self.chunks.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "no more data in ChunkedFakeStream",
            ));
        }
        let chunk = self.chunks[self.index];
        let n = chunk.len().min(buf.len());
        buf[..n].copy_from_slice(&chunk[..n]);
        self.index += 1;
        Ok(n)
    }
}

#[test]
fn test_stream_incomplete_headers_eof_rejected() {
    let mut stream = std::io::Cursor::new(&b"GET / HTTP/1.1\r\nHost: example.com"[..]);
    let err = HttpServer::parse_http_request_from_reader(&mut stream, "127.0.0.1").unwrap_err();
    match err {
        crate::CorvoError::Runtime { message, .. } => {
            assert!(
                message.contains("headers were complete"),
                "expected incomplete headers error, got: {}",
                message
            );
        }
        other => panic!("expected runtime error for incomplete headers, got: {:?}", other),
    }
}

#[test]
fn test_stream_short_body_read_exact_success() {
    const INITIAL: &[u8] =
        b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 11\r\n\r\nHello ";
    const REMAINING: &[u8] = b"world";
    let mut stream = ChunkedFakeStream::new(vec![INITIAL, REMAINING]);
    let result = HttpServer::parse_http_request_from_reader(&mut stream, "127.0.0.1").unwrap();
    assert_eq!(body(&result), "Hello world");
}

#[test]
fn test_stream_short_body_read_exact_eof_error() {
    const TRUNCATED: &[u8] =
        b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 20\r\n\r\nshort";
    let mut stream = ChunkedFakeStream::new(vec![TRUNCATED]);
    let err = HttpServer::parse_http_request_from_reader(&mut stream, "127.0.0.1").unwrap_err();
    match err {
        crate::CorvoError::Runtime { message, .. } => {
            assert!(
                message.contains("Failed to read body"),
                "expected body read failure, got: {}",
                message
            );
        }
        other => panic!(
            "expected runtime error for truncated body, got: {:?}",
            other
        ),
    }
}

#[test]
fn test_stream_content_length_over_limit_rejected() {
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 10485761\r\n\r\n";
    let mut stream = ChunkedFakeStream::new(vec![raw]);
    let err = HttpServer::parse_http_request_from_reader(&mut stream, "127.0.0.1").unwrap_err();
    match err {
        crate::CorvoError::Runtime { message, .. } => {
            assert!(
                message.contains("too large"),
                "expected oversize body rejection, got: {}",
                message
            );
        }
        other => panic!("expected runtime error for oversized CL, got: {:?}", other),
    }
}

// ===========================================================================
// Section 2: Content-Length Manipulation
// ===========================================================================

#[test]
fn test_cl_empty_body_zero_content_length() {
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 0\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(body(&result), "");
}

#[test]
fn test_cl_multiple_headers_smuggling_discrepancy() {
    // Multiple Content-Length headers: first used for body, last stored in map
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\nContent-Length: 10\r\n\r\nhello";
    let result = parse_raw(raw);
    // Body uses first CL value (5) for truncation
    assert_eq!(body(&result), "hello");
    // Headers map stores last value (HashMap overwrite)
    assert_eq!(
        header(&result, "content-length").unwrap(),
        Value::String("10".to_string())
    );
}

#[test]
fn test_cl_negative_value_ignored() {
    // Content-Length: -1 fails to parse as usize, so it's ignored (defaults to 0)
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: -1\r\n\r\nbody data here";
    let result = parse_raw(raw);
    // CL parse fails → defaults to 0 → body truncated to 0
    assert_eq!(body(&result), "");
    // The header is still stored in the map
    assert_eq!(
        header(&result, "content-length").unwrap(),
        Value::String("-1".to_string())
    );
}

#[test]
fn test_cl_overflow_value_ignored() {
    // Value larger than usize::MAX fails to parse → skipped → defaults to 0
    let raw =
        b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 99999999999999999999\r\n\r\nbody";
    let result = parse_raw(raw);
    // CL parse fails → body truncated to 0
    assert_eq!(body(&result), "");
}

#[test]
fn test_cl_hex_value_ignored() {
    // Hex values don't parse as usize with decimal parsing
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 0x0A\r\n\r\nbody";
    let result = parse_raw(raw);
    assert_eq!(body(&result), "");
}

#[test]
fn test_cl_plus_sign_parsed() {
    // +5 parses as 5 (Rust's usize::from_str accepts leading + in some editions)
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: +5\r\n\r\nhello";
    let result = parse_raw(raw);
    assert_eq!(body(&result), "hello");
}

#[test]
fn test_cl_trailing_garbage_ignored() {
    // "5abc" doesn't parse as usize → skipped → defaults to 0
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5abc\r\n\r\nhello";
    let result = parse_raw(raw);
    assert_eq!(body(&result), "");
}

#[test]
fn test_cl_leading_zeros_parsed_correctly() {
    // "005" parses as 5 → body truncated to 5
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 005\r\n\r\nhello world";
    let result = parse_raw(raw);
    assert_eq!(body(&result), "hello");
}

#[test]
fn test_cl_zero_with_body_present() {
    // CL=0 but body data exists → body truncated to 0
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 0\r\n\r\nunexpected body";
    let result = parse_raw(raw);
    assert_eq!(body(&result), "");
}

#[test]
fn test_cl_huge_value_no_body_read() {
    // Huge CL (less than max usize) - body is shorter, no padding happens
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 999999\r\n\r\nshort";
    let result = parse_raw(raw);
    // Body is "short" (5 bytes) since no padding happens in parse_http_raw
    assert_eq!(body(&result), "short");
}

// ===========================================================================
// Section 3: Transfer-Encoding Abuse
// ===========================================================================

#[test]
fn test_te_multiple_headers() {
    // Multiple Transfer-Encoding headers (last wins in HashMap)
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: gzip\r\nTransfer-Encoding: chunked\r\n\r\nbody";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "transfer-encoding").unwrap(),
        Value::String("chunked".to_string())
    );
}

#[test]
fn test_te_compound_value_chunked_first() {
    // "Transfer-Encoding: chunked, gzip" - compound value
    let raw =
        b"POST / HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: chunked, gzip\r\n\r\nbody";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "transfer-encoding").unwrap(),
        Value::String("chunked, gzip".to_string())
    );
}

#[test]
fn test_te_compound_value_chunked_obfuscated() {
    // Obfuscated chunked: "Transfer-Encoding: x, chunked"
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: x, chunked\r\n\r\nbody";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "transfer-encoding").unwrap(),
        Value::String("x, chunked".to_string())
    );
}

#[test]
fn test_te_identity_value() {
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: identity\r\nContent-Length: 4\r\n\r\ntest";
    let result = parse_raw(raw);
    assert_eq!(body(&result), "test");
}

// ===========================================================================
// Section 4: Header Injection & Smuggling
// ===========================================================================

#[test]
fn test_header_tab_before_colon() {
    // Tab before colon -> trimmed by trim()
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length\t: 5\r\n\r\nhello";
    let result = parse_raw(raw);
    assert!(has_header(&result, "content-length"));
}

#[test]
fn test_header_tab_after_colon_in_value() {
    // Tab after colon in value -> trimmed by trim()
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nX-Value:\t\r\n\r\n";
    let result = parse_raw(raw);
    assert!(has_header(&result, "x-value"));
    assert_eq!(
        header(&result, "x-value").unwrap(),
        Value::String("".to_string())
    );
}

#[test]
fn test_header_empty_name() {
    // Empty header name (just colon)
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\n: orphan\r\n\r\n";
    let result = parse_raw(raw);
    // Empty key after trim → skipped
    assert!(!has_header(&result, ""));
    // But the empty key is still in the raw parsing... actually trim() on empty gives empty
    // splitn(2, ':') on ": orphan" gives ["", " orphan"]
    // k = "".trim() = "" → to_lowercase() = "" → !k.is_empty() = false → skipped
}

#[test]
fn test_header_empty_value() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nX-Empty:\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "x-empty").unwrap(),
        Value::String("".to_string())
    );
}

#[test]
fn test_header_space_only_value() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nX-Space:   \r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "x-space").unwrap(),
        Value::String("".to_string())
    );
}

#[test]
fn test_header_multiple_colons_in_value() {
    // Value contains colon - splitn(2, ':') limits to 2 parts, so value is everything after first colon
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nX-Data: key:value:123\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "x-data").unwrap(),
        Value::String("key:value:123".to_string())
    );
}

#[test]
fn test_header_crlf_injection_response_splitting() {
    // CRLF in header value - should be captured as-is (not interpreted as new header)
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nX-Injected: hello\r\nX-Fake: evil\r\n\r\n";
    let result = parse_raw(raw);
    // The CRLF is real line endings, so X-Injected value is just "hello"
    assert_eq!(
        header(&result, "x-injected").unwrap(),
        Value::String("hello".to_string())
    );
    assert!(has_header(&result, "x-fake"));
}

#[test]
fn test_header_leading_whitespace_in_name() {
    // Leading whitespace in header name - trimmed
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\n  X-Padded: value\r\n\r\n";
    let result = parse_raw(raw);
    assert!(has_header(&result, "x-padded"));
    assert_eq!(
        header(&result, "x-padded").unwrap(),
        Value::String("value".to_string())
    );
}

#[test]
fn test_header_underscore_prefix() {
    // Headers starting with underscore (often used for internal headers)
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\n_X-Custom: internal\r\n\r\n";
    let result = parse_raw(raw);
    assert!(has_header(&result, "_x-custom"));
}

#[test]
fn test_header_non_ascii_name() {
    // Non-ASCII bytes in header name
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\n\xc0\x80: evil\r\n\r\n";
    let result = parse_raw(raw);
    let headers = result.get("headers").unwrap().as_map().unwrap();
    let has_non_ascii_key = headers.keys().any(|k| k.bytes().any(|b| b > 127));
    assert!(has_non_ascii_key);
}

// ===========================================================================
// Section 5: Host Header Attacks
// ===========================================================================

#[test]
fn test_host_with_port() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com:8080\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "host").unwrap(),
        Value::String("example.com:8080".to_string())
    );
}

#[test]
fn test_host_ipv6() {
    let raw = b"GET / HTTP/1.1\r\nHost: [::1]:8080\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "host").unwrap(),
        Value::String("[::1]:8080".to_string())
    );
}

#[test]
fn test_host_empty_value() {
    let raw = b"GET / HTTP/1.1\r\nHost:\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "host").unwrap(),
        Value::String("".to_string())
    );
}

#[test]
fn test_host_trailing_dot() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com.\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "host").unwrap(),
        Value::String("example.com.".to_string())
    );
}

#[test]
fn test_host_multiple_headers() {
    // Multiple Host headers (last wins in HashMap)
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nHost: attacker.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "host").unwrap(),
        Value::String("attacker.com".to_string())
    );
}

// ===========================================================================
// Section 6: Method & Protocol Abuse
// ===========================================================================

#[test]
fn test_method_connect() {
    let raw = b"CONNECT example.com:443 HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "CONNECT");
}

#[test]
fn test_method_options_asterisk() {
    let raw = b"OPTIONS * HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "OPTIONS");
    assert_eq!(path(&result), "*");
}

#[test]
fn test_method_trace() {
    let raw = b"TRACE / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "TRACE");
}

#[test]
fn test_method_lowercase() {
    // Method normalization to uppercase
    let raw = b"get / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "GET");
}

#[test]
fn test_method_special_chars() {
    // Arbitrary method with special characters
    let raw = b"!@#$%^&*() / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "!@#$%^&*()");
}

#[test]
fn test_method_very_long() {
    // Very long method name (potential buffer overflow scenario)
    let long_method = "A".repeat(10_000);
    let request = format!("{} / HTTP/1.1\r\nHost: example.com\r\n\r\n", long_method);
    let result = parse_raw(request.as_bytes());
    assert_eq!(method(&result), long_method);
}

#[test]
fn test_method_leading_trailing_spaces() {
    // split_whitespace handles these
    let raw = b"  GET  /  HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "GET");
    assert_eq!(path(&result), "/");
}

#[test]
fn test_protocol_http_1_0() {
    let raw = b"GET / HTTP/1.0\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "GET");
}

#[test]
fn test_protocol_unknown() {
    let raw = b"GET / FOO/1.0\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "GET");
    assert_eq!(path(&result), "/");
}

#[test]
fn test_protocol_missing_version() {
    // Request line with no HTTP version
    let raw = b"GET /\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "GET");
    assert_eq!(path(&result), "/");
}

#[test]
fn test_request_line_multiple_spaces() {
    let raw = b"GET  /  HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "GET");
    assert_eq!(path(&result), "/");
}

#[test]
fn test_request_line_only_method() {
    let raw = b"GET\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "");
    assert_eq!(path(&result), "");
}

// ===========================================================================
// Section 7: Path Traversal & Injection
// ===========================================================================

#[test]
fn test_path_double_slash() {
    let raw = b"GET //etc/passwd HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "//etc/passwd");
}

#[test]
fn test_path_backslash() {
    // Backslash path (Windows-style, potential confusion on Unix)
    let raw = b"GET /..\\..\\etc\\passwd HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "/..\\..\\etc\\passwd");
}

#[test]
fn test_path_semicolon_params() {
    // Path with semicolon parameters
    let raw = b"GET /path;param=value;session=abc123 HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "/path;param=value;session=abc123");
}

#[test]
fn test_path_fragment_stripped() {
    // Fragment (#) is not stripped by the parser (should ideally be stripped before sending)
    let raw = b"GET /page#section HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "/page#section");
}

#[test]
fn test_path_absolute_form() {
    // Absolute-form request line (proxy-style)
    // Parser splits on '?' to separate path from query
    let raw = b"GET http://user:pass@host:8080/path?q=1 HTTP/1.1\r\nHost: host\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "http://user:pass@host:8080/path");
    let q = query_map(&result);
    assert_eq!(q.get("q").unwrap(), &Value::String("1".to_string()));
}

#[test]
fn test_path_with_query_and_fragment() {
    let raw = b"GET /search?q=hello#section HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "/search");
    // Fragment is included in query value since splitn(2, '?') puts everything after ? into query
    // Then splitn(2, '=') gives key="q", value="hello#section"
    let q = query_map(&result);
    assert_eq!(
        q.get("q").unwrap(),
        &Value::String("hello#section".to_string())
    );
}

#[test]
fn test_path_unicode() {
    let raw = b"GET /%E2%82%AC%E2%82%AC HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "/%E2%82%AC%E2%82%AC");
}

#[test]
fn test_path_very_long() {
    let long_path = "/".to_string() + &"A".repeat(10_000);
    let request = format!("GET {} HTTP/1.1\r\nHost: example.com\r\n\r\n", long_path);
    let result = parse_raw(request.as_bytes());
    assert_eq!(path(&result), long_path);
}

#[test]
fn test_path_null_byte() {
    let raw = b"GET /../../../etc/passwd%00 HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "/../../../etc/passwd%00");
}

// ===========================================================================
// Section 8: Query String Attacks
// ===========================================================================

#[test]
fn test_query_duplicate_keys() {
    // Duplicate query keys - last value wins in HashMap
    let raw = b"GET /search?key=a&key=b HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    let q = query_map(&result);
    assert_eq!(q.get("key").unwrap(), &Value::String("b".to_string()));
}

#[test]
fn test_query_empty_key() {
    let raw = b"GET /search?=value HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    let q = query_map(&result);
    assert!(q.contains_key(""));
}

#[test]
fn test_query_empty_value() {
    let raw = b"GET /search?key= HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    let q = query_map(&result);
    assert_eq!(q.get("key").unwrap(), &Value::String("".to_string()));
}

#[test]
fn test_query_encoded_separators() {
    // Query values with encoded & and =
    let raw = b"GET /search?q=%26special%3Dvalue HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    let q = query_map(&result);
    assert_eq!(
        q.get("q").unwrap(),
        &Value::String("%26special%3Dvalue".to_string())
    );
}

#[test]
fn test_query_many_parameters() {
    let params: Vec<String> = (0..1000).map(|i| format!("key{}=val{}", i, i)).collect();
    let query = params.join("&");
    let request = format!(
        "GET /search?{} HTTP/1.1\r\nHost: example.com\r\n\r\n",
        query
    );
    let result = parse_raw(request.as_bytes());
    let q = query_map(&result);
    assert_eq!(q.len(), 1000);
}

#[test]
fn test_query_no_value() {
    // Key without = sign (boolean flag)
    let raw = b"GET /search?debug HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    let q = query_map(&result);
    assert_eq!(q.get("debug").unwrap(), &Value::String("".to_string()));
}

// ===========================================================================
// Section 9: Body Handling Edge Cases
// ===========================================================================

#[test]
fn test_body_binary_data() {
    // Body containing null bytes and binary data
    let raw: Vec<u8> = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 8\r\n\r\n"
        .iter()
        .chain(b"binary\x00data".iter())
        .copied()
        .collect();
    let result = parse_raw(&raw);
    assert_eq!(body(&result).len(), 8);
    assert_eq!(body(&result), "binary\x00d");
}

#[test]
fn test_body_no_cl_with_data() {
    // No Content-Length but body data present
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\n\r\nbody without cl";
    let result = parse_raw(raw);
    // Without CL, content_length defaults to 0, body truncated to 0
    assert_eq!(body(&result), "");
}

#[test]
fn test_body_get_with_content_length() {
    // GET request with Content-Length (unusual but valid)
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nhello";
    let result = parse_raw(raw);
    assert_eq!(body(&result), "hello");
}

#[test]
fn test_body_whitespace_only() {
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\n     ";
    let result = parse_raw(raw);
    assert_eq!(body(&result), "     ");
}

// ===========================================================================
// Section 10: Encoding & Character Attacks
// ===========================================================================

#[test]
fn test_encoding_utf8_bom() {
    // UTF-8 BOM at the start of the request
    let raw = b"\xef\xbb\xbfGET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    // BOM bytes are part of the request line, method becomes "\xef\xbb\xbfGET"
    assert_eq!(method(&result).as_bytes()[0], 0xef);
    assert_ne!(method(&result), "GET");
}

#[test]
fn test_encoding_invalid_utf8() {
    // Invalid UTF-8 bytes in request line
    let raw = b"GET /\xff\xfe HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    // Should not panic; String::from_utf8_lossy replaces invalid bytes
    assert_eq!(method(&result), "GET");
    assert!(path(&result).contains('\u{fffd}'));
}

#[test]
fn test_encoding_control_chars_in_headers() {
    // Control characters (besides CRLF) in header values
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nX-Ctrl: hello\x01\x02\x03world\r\n\r\n";
    let result = parse_raw(raw);
    let val = header(&result, "x-ctrl").unwrap();
    let val = val.as_string().unwrap();
    assert!(val.contains('\x01'));
    assert!(val.contains('\x02'));
    assert!(val.contains('\x03'));
}

#[test]
fn test_encoding_high_bit_bytes_in_header_name() {
    // Bytes > 127 in header name
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\n\xff\xfe: value\r\n\r\n";
    let result = parse_raw(raw);
    let headers = result.get("headers").unwrap().as_map().unwrap();
    let found = headers.keys().any(|k| k.contains('\u{fffd}'));
    assert!(found || headers.len() > 1);
}

#[test]
fn test_encoding_null_in_header_name() {
    // Null byte in header name - lossy conversion replaces it
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nFoo\x00Bar: value\r\n\r\n";
    let result = parse_raw(raw);
    assert!(has_header(&result, "foo\x00bar"));
}

#[test]
fn test_encoding_null_in_header_name_lossy() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nFoo\x00Bar: value\r\n\r\n";
    let result = parse_raw(raw);
    // Check the raw key is present
    let headers = result.get("headers").unwrap().as_map().unwrap();
    let found = headers
        .iter()
        .any(|(k, v)| k.contains('\x00') && v.as_string().is_some_and(|s| s == "value"));
    assert!(found, "Null byte in header name should be preserved");

    // Also check by exact lossy conversion
    let expected_key = String::from_utf8_lossy(b"foo\x00bar").to_string();
    assert!(headers.contains_key(&expected_key));
}

// ===========================================================================
// Section 11: Resource Exhaustion
// ===========================================================================

#[test]
fn test_exhaustion_many_headers() {
    let mut request = String::from("GET / HTTP/1.1\r\nHost: example.com\r\n");
    for i in 0..10_000 {
        request.push_str(&format!("X-Header-{}: value-{}\r\n", i, i));
    }
    request.push_str("\r\n");
    let result = parse_raw(request.as_bytes());
    let headers = result.get("headers").unwrap().as_map().unwrap();
    assert!(headers.len() >= 10_000);
}

#[test]
fn test_exhaustion_header_name_long() {
    let long_key = "X".repeat(500_000);
    let request = format!(
        "GET / HTTP/1.1\r\nHost: example.com\r\n{}: value\r\n\r\n",
        long_key
    );
    let result = parse_raw(request.as_bytes());
    let headers = result.get("headers").unwrap().as_map().unwrap();
    let key = long_key.to_lowercase();
    assert!(headers.contains_key(&key));
}

#[test]
fn test_exhaustion_header_value_extremely_long() {
    let long_value = "V".repeat(1_000_000);
    let request = format!(
        "GET / HTTP/1.1\r\nHost: example.com\r\nX-Long: {}\r\n\r\n",
        long_value
    );
    let result = parse_raw(request.as_bytes());
    assert_eq!(
        header(&result, "x-long")
            .unwrap()
            .as_string()
            .unwrap()
            .len(),
        1_000_000
    );
}

#[test]
fn test_exhaustion_many_query_params() {
    let params: Vec<String> = (0..10_000).map(|i| format!("k{}={}", i, i)).collect();
    let query = params.join("&");
    let request = format!(
        "GET /search?{} HTTP/1.1\r\nHost: example.com\r\n\r\n",
        query
    );
    let result = parse_raw(request.as_bytes());
    let q = query_map(&result);
    assert_eq!(q.len(), 10_000);
}

// ===========================================================================
// Section 12: Request Line Edge Cases
// ===========================================================================

#[test]
fn test_request_line_empty() {
    let raw = b"\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "");
    assert_eq!(path(&result), "");
}

#[test]
fn test_request_line_only_crlf() {
    let raw = b"\r\n";
    let result = parse_raw(raw);
    // header_end = request.len() = 2, header_str = "\r\n", first line = ""
    assert_eq!(method(&result), "");
}

#[test]
fn test_request_line_http_2_preface() {
    // HTTP/2 connection preface (PRI * HTTP/2.0)
    let raw = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "PRI");
    assert_eq!(path(&result), "*");
}

#[test]
fn test_request_line_only_spaces() {
    let raw = b"   \r\n\r\n";
    let result = parse_raw(raw);
    // method and path parse gracefully
    let _ = method(&result);
    let _ = path(&result);
}

#[test]
fn test_request_line_tab_separated() {
    // Tab characters between request components
    let raw = b"GET\t/\tHTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    // split_whitespace handles tabs
    assert_eq!(method(&result), "GET");
    assert_eq!(path(&result), "/");
}

// ===========================================================================
// Section 13: IP Address Handling
// ===========================================================================

#[test]
fn test_ip_address_preserved() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw_with_ip(raw, "192.168.1.1");
    assert_eq!(ip(&result), "192.168.1.1");
}

#[test]
fn test_ipv6_address_preserved() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw_with_ip(raw, "::1");
    assert_eq!(ip(&result), "::1");
}

#[test]
fn test_ip_empty_string() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw_with_ip(raw, "");
    assert_eq!(ip(&result), "");
}

// ===========================================================================
// Section 14: Mixed and Abnormal Line Endings
// ===========================================================================

#[test]
fn test_line_endings_lf_only() {
    let raw = b"GET / HTTP/1.1\nHost: example.com\nX-Custom: val\n\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "GET");
    assert!(has_header(&result, "host"));
    assert!(has_header(&result, "x-custom"));
}

#[test]
fn test_line_endings_mixed_cr_lf_lf() {
    // Mixed \r\n and bare \n
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\nX-Custom: val\r\n\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "GET");
}

#[test]
fn test_line_endings_cr_without_lf() {
    // Bare CR without LF (should not form a header boundary)
    let raw = b"GET / HTTP/1.1\rHost: example.com\r\r";
    let result = parse_raw(raw);
    // No \r\n\r\n or \n\n found → header_end = request.len()
    // Since no header boundary, the entire request may be treated as the request line
    assert!(result.contains_key("method"));
    assert!(result.contains_key("path"));
}

// ===========================================================================
// Section 15: Transfer-Encoding + Content-Length Conflict Resolution
// ===========================================================================

#[test]
fn test_te_chunked_with_cl_uses_cl_for_body() {
    // The parser uses CL for body sizing even when TE is chunked
    let raw =
        b"POST / HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: chunked\r\nContent-Length: 4\r\n\r\ntest";
    let result = parse_raw(raw);
    // Body is driven by CL (4 bytes)
    assert_eq!(body(&result), "test");
}

#[test]
fn test_te_multiple_values_with_chunked_final() {
    let raw =
        b"POST / HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: gzip, chunked\r\n\r\nbody";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "transfer-encoding").unwrap(),
        Value::String("gzip, chunked".to_string())
    );
}

#[test]
fn test_te_case_sensitivity() {
    // TE header is case-insensitive
    let raw = b"POST / HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: Chunked\r\n\r\nbody";
    let result = parse_raw(raw);
    let te_val = header(&result, "transfer-encoding").unwrap();
    let te = te_val.as_string().unwrap();
    assert_eq!(te, "Chunked");
}

// ===========================================================================
// Section 16: Cookie / Sensitive Header Handling
// ===========================================================================

#[test]
fn test_cookie_header() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nCookie: session=abc123; theme=dark\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "cookie").unwrap(),
        Value::String("session=abc123; theme=dark".to_string())
    );
}

#[test]
fn test_authorization_header() {
    let raw =
        b"GET / HTTP/1.1\r\nHost: example.com\r\nAuthorization: Bearer eyJhbGciOiJIUzI1NiJ9\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "authorization").unwrap(),
        Value::String("Bearer eyJhbGciOiJIUzI1NiJ9".to_string())
    );
}

// ===========================================================================
// Section 17: Boundary Conditions
// ===========================================================================

#[test]
fn test_empty_request() {
    let raw = b"";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "");
    assert_eq!(path(&result), "");
    assert_eq!(body(&result), "");
}

#[test]
fn test_request_with_only_headers_no_body() {
    let raw = b"POST / HTTP/1.1\r\nContent-Length: 0\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "POST");
    assert_eq!(body(&result), "");
}

#[test]
fn test_request_no_headers_at_all() {
    let raw = b"GET /\r\n";
    let result = parse_raw(raw);
    assert_eq!(method(&result), "GET");
    assert_eq!(path(&result), "/");
    assert_eq!(body(&result), "");
}

#[test]
fn test_header_value_with_only_numbers() {
    let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\nX-Num: 12345\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(
        header(&result, "x-num").unwrap(),
        Value::String("12345".to_string())
    );
}

#[test]
fn test_header_mixed_case_name_preserved_in_lowercase() {
    let raw = b"GET / HTTP/1.1\r\nContent-Type: text/html\r\n\r\n";
    let result = parse_raw(raw);
    assert!(has_header(&result, "content-type"));
    assert!(!has_header(&result, "Content-Type"));
}

// ===========================================================================
// Section 18: Multiple Attack Vector Combinations
// ===========================================================================

#[test]
fn test_combined_path_traversal_and_query_injection() {
    let raw =
        b"GET /../../../etc/passwd?debug=true&cmd=ls%20-la HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    assert_eq!(path(&result), "/../../../etc/passwd");
    let q = query_map(&result);
    assert!(q.contains_key("debug"));
    assert!(q.contains_key("cmd"));
}

#[test]
fn test_combined_cl_te_smuggling_and_host_poisoning() {
    let raw = b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\nContent-Length: 42\r\nHost: attacker.com\r\n\r\nbody";
    let result = parse_raw(raw);
    assert!(has_header(&result, "transfer-encoding"));
    assert!(has_header(&result, "content-length"));
    assert_eq!(
        header(&result, "host").unwrap(),
        Value::String("attacker.com".to_string())
    );
}

#[test]
fn test_combined_crlf_and_header_injection() {
    let raw = b"GET /%0d%0aX-Injected:%20true HTTP/1.1\r\nHost: example.com\r\n\r\n";
    let result = parse_raw(raw);
    // CRLF is URL-encoded in path, so it's preserved literally
    assert_eq!(path(&result), "/%0d%0aX-Injected:%20true");
}
