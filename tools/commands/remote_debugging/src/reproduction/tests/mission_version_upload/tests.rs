use super::*;

#[test]
fn extract_token_matches_sed() {
    assert_eq!(
        extract_token("http://localhost:3000/auth/callback#access_token=abc123&x=1"),
        "abc123"
    );
    assert_eq!(extract_token("http://example/?nope=1"), "");
    assert_eq!(
        extract_token("https://x/#foo=1&access_token=tok%2Fval&refresh=r"),
        "tok%2Fval"
    );
}
