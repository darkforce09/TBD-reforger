//! Tests for [`super`] — `Range` header parsing and the 206 / 416 arithmetic.

use super::*;

#[test]
fn the_three_single_range_forms_parse() {
    assert_eq!(
        parse_range_header("bytes=0-499"),
        Ok(ByteRangeRequest::Bounded {
            start: 0,
            end_inclusive: 499
        })
    );
    assert_eq!(
        parse_range_header(" Bytes = 9500- "),
        Ok(ByteRangeRequest::OpenEnded { start: 9500 })
    );
    assert_eq!(
        parse_range_header("bytes=-500"),
        Ok(ByteRangeRequest::Suffix { length: 500 })
    );
    assert_eq!(
        parse_range_header("bytes=5-5"),
        Ok(ByteRangeRequest::Bounded {
            start: 5,
            end_inclusive: 5
        })
    );
}

#[test]
fn anything_but_one_well_formed_byte_range_is_refused() {
    for (header, expected) in [
        ("items=0-1", RangeHeaderError::UnsupportedUnit),
        ("bytes=0-1,5-6", RangeHeaderError::MultipleRanges),
        ("bytes", RangeHeaderError::Malformed),
        ("bytes=", RangeHeaderError::Malformed),
        ("bytes=-", RangeHeaderError::Malformed),
        ("bytes=5", RangeHeaderError::Malformed),
        ("bytes=9-3", RangeHeaderError::Malformed),
        ("bytes=a-3", RangeHeaderError::Malformed),
        ("bytes=+1-3", RangeHeaderError::Malformed),
        ("bytes=1--3", RangeHeaderError::Malformed),
        ("bytes=99999999999999999999-", RangeHeaderError::Malformed),
    ] {
        assert_eq!(parse_range_header(header), Err(expected), "{header}");
    }
}

fn partial(start: u64, end_inclusive: u64, total_len: u64) -> RangeResponsePlan {
    RangeResponsePlan::Partial(ByteSlice {
        start,
        end_inclusive,
        total_len,
    })
}

#[test]
fn satisfiable_ranges_resolve_to_the_right_slice() {
    for (header, total_len, expected) in [
        ("bytes=0-499", 10_000, partial(0, 499, 10_000)),
        ("bytes=9500-", 10_000, partial(9500, 9999, 10_000)),
        ("bytes=-500", 10_000, partial(9500, 9999, 10_000)),
        ("bytes=9990-20000", 10_000, partial(9990, 9999, 10_000)),
        ("bytes=-20000", 10_000, partial(0, 9999, 10_000)),
        ("bytes=0-0", 1, partial(0, 0, 1)),
        ("bytes=9999-", 10_000, partial(9999, 9999, 10_000)),
        (
            "bytes=152713000-152713113",
            152_713_114,
            partial(152_713_000, 152_713_113, 152_713_114),
        ),
    ] {
        assert_eq!(
            plan_range_response(Some(header), total_len),
            expected,
            "{header}"
        );
    }
}

#[test]
fn unsatisfiable_ranges_answer_416() {
    for (header, total_len) in [
        ("bytes=10000-", 10_000),
        ("bytes=10000-10005", 10_000),
        ("bytes=-0", 10_000),
        ("bytes=0-0", 0),
        ("bytes=-5", 0),
    ] {
        assert_eq!(
            plan_range_response(Some(header), total_len),
            RangeResponsePlan::Unsatisfiable { total_len },
            "{header}"
        );
    }
    assert_eq!(unsatisfied_content_range(10_000), "bytes */10000");
}

#[test]
fn a_missing_or_unusable_header_serves_the_full_body() {
    assert_eq!(plan_range_response(None, 10), RangeResponsePlan::FullBody);
    for header in ["bytes=0-1,4-5", "items=0-1", "garbage"] {
        assert_eq!(
            plan_range_response(Some(header), 10),
            RangeResponsePlan::FullBody,
            "{header}"
        );
    }
}

#[test]
fn a_slice_reports_its_length_bounds_and_content_range() {
    let slice = ByteSlice {
        start: 9500,
        end_inclusive: 9999,
        total_len: 10_000,
    };
    assert_eq!(slice.len(), 500);
    assert!(!slice.is_empty());
    assert_eq!(slice.end_exclusive(), 10_000);
    assert_eq!(slice.content_range(), "bytes 9500-9999/10000");
    assert_eq!(STATUS_PARTIAL_CONTENT, 206);
    assert_eq!(STATUS_RANGE_NOT_SATISFIABLE, 416);
}
