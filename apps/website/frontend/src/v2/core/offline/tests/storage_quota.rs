use super::*;

#[test]
fn quota_verdict_refuses_a_download_larger_than_the_free_space() {
    let estimate = Some(StorageEstimate {
        quota: 1_000,
        usage: 400,
    });
    assert_eq!(quota_verdict(estimate, 600), QuotaVerdict::Enough);
    assert_eq!(quota_verdict(estimate, 0), QuotaVerdict::Enough);
    assert_eq!(
        quota_verdict(estimate, 601),
        QuotaVerdict::Short {
            needed: 601,
            available: 600
        }
    );
}

#[test]
fn quota_verdict_treats_usage_over_quota_as_no_free_space_and_no_estimate_as_unknown() {
    let over = Some(StorageEstimate {
        quota: 100,
        usage: 150,
    });
    assert_eq!(
        quota_verdict(over, 1),
        QuotaVerdict::Short {
            needed: 1,
            available: 0
        }
    );
    assert_eq!(quota_verdict(None, u64::MAX), QuotaVerdict::Unknown);
}

#[test]
fn bytes_from_js_number_accepts_only_finite_non_negative_numbers() {
    assert_eq!(bytes_from_js_number(Some(248_000_000.0)), Some(248_000_000));
    assert_eq!(bytes_from_js_number(Some(0.0)), Some(0));
    assert_eq!(bytes_from_js_number(Some(-1.0)), None);
    assert_eq!(bytes_from_js_number(Some(f64::NAN)), None);
    assert_eq!(bytes_from_js_number(Some(f64::INFINITY)), None);
    assert_eq!(bytes_from_js_number(None), None);
}
