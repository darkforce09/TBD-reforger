//! The pure parts of the transport and the login: header dumps, callback fragments, encoding.
use super::development_login::access_token_from_callback;
use super::http_exchange::parse_headers;
use super::*;

#[test]
fn header_dump_skips_the_status_line_and_keeps_colons_in_values() {
    let dump = "HTTP/1.1 302 Found\r\nLocation: http://localhost:5173/auth/callback#access_token=abc\r\nETag: \"ff\"\r\n\r\n";
    let headers = parse_headers(dump);
    assert_eq!(
        headers,
        [
            (
                "Location".to_string(),
                "http://localhost:5173/auth/callback#access_token=abc".to_string()
            ),
            ("ETag".to_string(), "\"ff\"".to_string()),
        ]
    );
    let answer = ApiAnswer {
        status: 302,
        headers,
        body: Vec::new(),
    };
    assert_eq!(answer.header("location").map(str::len), Some(52));
    assert_eq!(answer.header("etag"), Some("\"ff\""));
}

#[test]
fn callback_fragment_yields_the_access_token_only_when_present() {
    assert_eq!(
        access_token_from_callback("http://x/auth/callback#access_token=tok&refresh_token=r"),
        Some("tok".to_string())
    );
    assert_eq!(
        access_token_from_callback("http://x/auth/callback#refresh_token=r&access_token=tok"),
        Some("tok".to_string())
    );
    assert_eq!(
        access_token_from_callback("http://x/auth/callback#access_token="),
        None
    );
    assert_eq!(
        access_token_from_callback("http://x/auth/callback?access_token=tok"),
        None
    );
}

#[test]
fn components_are_percent_encoded_except_unreserved_bytes() {
    assert_eq!(encode_component("A-z_0.9~"), "A-z_0.9~");
    assert_eq!(
        encode_component("playtest ended/now"),
        "playtest%20ended%2Fnow"
    );
    assert_eq!(encode_component("é"), "%C3%A9");
}

#[test]
fn refusal_code_and_excerpt_read_the_error_body() {
    let answer = ApiAnswer {
        status: 409,
        headers: Vec::new(),
        body: br#"{"error":"busy","details":{"code":"DEPLOYMENT_IN_PROGRESS"}}"#.to_vec(),
    };
    assert_eq!(
        answer.refusal_code().as_deref(),
        Some("DEPLOYMENT_IN_PROGRESS")
    );
    assert!(answer.excerpt().contains("busy"));
}
