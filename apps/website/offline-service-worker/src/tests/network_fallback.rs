//! Tests for [`super`] — when the saved copy answers, its marker and its date.

use super::*;

#[test]
fn gateway_and_server_failures_prefer_the_saved_copy() {
    for status in [500, 502, 503, 504, 520, 599] {
        assert!(is_gateway_or_server_failure(status), "{status}");
        assert!(
            prefers_saved_copy(NetworkAnswer::Status(status)),
            "a {status} from a proxy whose API is down must be answered from the saved copy"
        );
    }
}

#[test]
fn an_unreachable_network_prefers_the_saved_copy() {
    assert!(prefers_saved_copy(NetworkAnswer::Unreachable));
}

#[test]
fn successes_redirects_and_refusals_are_never_masked() {
    for status in [
        0, 200, 204, 206, 301, 304, 400, 401, 403, 404, 409, 416, 429, 499, 600,
    ] {
        assert!(
            !prefers_saved_copy(NetworkAnswer::Status(status)),
            "{status} is the server's real answer"
        );
    }
    assert!(!is_gateway_or_server_failure(499));
    assert!(!is_gateway_or_server_failure(600));
}

#[test]
fn the_saved_copy_marker_is_a_lowercase_custom_header() {
    assert_eq!(SAVED_COPY_HEADER, "x-served-from-offline-cache");
    assert_eq!(SAVED_COPY_HEADER, SAVED_COPY_HEADER.to_ascii_lowercase());
}

#[test]
fn an_imf_fixdate_reads_as_day_month_year_and_utc_clock() {
    assert_eq!(
        saved_on_from_date_header("Sun, 28 Sep 2026 14:05:09 GMT").as_deref(),
        Some("28 Sep 2026, 14:05 UTC")
    );
    assert_eq!(
        saved_on_from_date_header("Thu, 01 Jan 2026 00:00:00 GMT").as_deref(),
        Some("1 Jan 2026, 00:00 UTC")
    );
}

#[test]
fn a_date_that_is_not_an_imf_fixdate_reads_as_none() {
    for date in [
        "",
        "yesterday",
        "Sun, 28 Sep 2026 14:05:09 CEST",
        "Sunday, 28-Sep-26 14:05:09 GMT",
        "Sun Sep 28 14:05:09 2026",
        "Sun, 28 Sepx 2026 14:05:09 GMT",
        "Sun, 28 Sep 2026 14:5:09 GMT",
        "Sun, 8 Sep 2026 14:05:09 GMT",
    ] {
        assert_eq!(saved_on_from_date_header(date), None, "{date:?}");
    }
}
