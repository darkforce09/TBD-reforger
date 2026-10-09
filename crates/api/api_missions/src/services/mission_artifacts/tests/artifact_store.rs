//! The findings an artifact compile refusal reports.

use super::{MAX_REPORTED_FINDINGS, reported_findings};

#[test]
fn refusals_report_each_distinct_finding_once_with_the_full_count() {
    let findings: Vec<String> = (0..30)
        .flat_map(|slot| {
            let finding = format!("/slots/{slot}/uid: \"\" is shorter than 1 character");
            [finding.clone(), finding]
        })
        .collect();
    let (count, unique) = reported_findings(findings);
    assert_eq!(count, 30);
    assert_eq!(unique[0], "/slots/0/uid: \"\" is shorter than 1 character");
    assert!(
        MAX_REPORTED_FINDINGS < count,
        "the cap bounds the body, the count stays whole"
    );
}
