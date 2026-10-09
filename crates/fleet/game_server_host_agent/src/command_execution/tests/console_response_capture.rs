use super::*;

#[test]
fn a_character_across_the_bound_is_left_out_whole() {
    // A three-byte character starting two bytes before the bound.
    let reply = format!("{}€ and more", "x".repeat(CONSOLE_RESPONSE_MAX_BYTES - 2));
    let capture = ConsoleResponseCapture::of(&reply);
    assert_eq!(capture.response, reply[..CONSOLE_RESPONSE_MAX_BYTES - 2]);
    assert!(capture.response_truncated);
    // A character that ends exactly at the bound is kept.
    let reply = format!("{}€ and more", "x".repeat(CONSOLE_RESPONSE_MAX_BYTES - 3));
    let capture = ConsoleResponseCapture::of(&reply);
    assert_eq!(capture.response, reply[..CONSOLE_RESPONSE_MAX_BYTES]);
    assert!(capture.response.ends_with('€'));
    assert!(capture.response_truncated);
}
