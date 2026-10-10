//! The two recorded agent CLI dialects parse into token counts, and every other shape fails
//! closed instead of becoming zero.

use super::parse_tokens_from_cli_json;
use serde_json::json;

#[test]
fn both_recorded_dialects_parse() {
    let claude = json!({"usage": {"input_tokens": 10, "output_tokens": 5,
        "cache_read_input_tokens": 100, "cache_creation_input_tokens": 7}});
    let counts = parse_tokens_from_cli_json(&claude).expect("claude dialect");
    assert_eq!(
        (
            counts.input,
            counts.output,
            counts.cache_read,
            counts.cache_write
        ),
        (10, 5, 100, 7)
    );
    assert_eq!(counts.reasoning, None);

    let cursor = json!({"usage": {"inputTokens": 3, "outputTokens": 4, "reasoningTokens": 2,
        "totalTokens": 9}});
    let counts = parse_tokens_from_cli_json(&cursor).expect("cursor dialect, total with reasoning");
    assert_eq!(
        (counts.input, counts.output, counts.reasoning),
        (3, 4, Some(2))
    );

    let top_level = json!({"inputTokens": 1, "outputTokens": 1});
    assert!(parse_tokens_from_cli_json(&top_level).is_ok());
}

#[test]
fn a_missing_or_unknown_usage_fails_closed() {
    for cli in [
        json!({"result": "done"}),
        json!({"usage": {"prompt": 3}}),
        json!({"usage": {"input_tokens": -1, "output_tokens": 2}}),
        json!({"usage": {"input_tokens": 1}}),
        json!({"usage": {"inputTokens": 3, "outputTokens": 4, "totalTokens": 99}}),
        json!({"usage": {"input_tokens": u64::MAX, "output_tokens": 1}}),
    ] {
        assert!(parse_tokens_from_cli_json(&cli).is_err(), "{cli}");
    }
}
