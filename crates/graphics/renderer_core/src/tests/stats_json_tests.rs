//! The statistics JSON writer: field order, value spellings, precision and escapes.

use crate::stats_json::StatsJson;

#[test]
fn an_empty_object_is_two_braces() {
    assert_eq!(StatsJson::new().finish(), "{}");
    assert_eq!(StatsJson::default().finish(), "{}");
}

#[test]
fn fields_appear_in_the_order_they_are_added_with_no_whitespace() {
    let mut json = StatsJson::new();
    json.text("backend", "webgpu")
        .count("instances", 12u64)
        .count("chunks", 3u32)
        .flag("submitted_last_frame", true)
        .flag("compute_cull", false)
        .decimal("gen_ms", 1.5, 1)
        .optional_decimal("gpu_frame_ms", None, 3)
        .optional_decimal("frame_ms", Some(0.5), 3);
    assert_eq!(
        json.finish(),
        "{\"backend\":\"webgpu\",\"instances\":12,\"chunks\":3,\"submitted_last_frame\":true,\
         \"compute_cull\":false,\"gen_ms\":1.5,\"gpu_frame_ms\":null,\"frame_ms\":0.500}"
    );
}

#[test]
fn decimals_match_the_standard_fixed_precision_formatting() {
    for (value, places) in [
        (0.0, 1),
        (1.05, 1),
        (2.675, 2),
        (16.666_666_6, 4),
        (-3.5, 3),
        (1.0e9, 4),
        (f64::NAN, 1),
        (f64::INFINITY, 4),
    ] {
        let mut json = StatsJson::new();
        json.decimal("v", value, places);
        assert_eq!(json.finish(), format!("{{\"v\":{value:.places$}}}"));
    }
}

#[test]
fn counts_print_the_full_unsigned_range() {
    let mut json = StatsJson::new();
    json.count("max", u64::MAX).count("zero", 0u32);
    assert_eq!(
        json.finish(),
        format!("{{\"max\":{},\"zero\":0}}", u64::MAX)
    );
}

#[test]
fn quotes_backslashes_and_control_characters_are_escaped() {
    let mut json = StatsJson::new();
    json.text("mode", "a\"b\\c\nd\u{1}é");
    assert_eq!(json.finish(), "{\"mode\":\"a\\\"b\\\\c\\nd\\u0001é\"}");
}
