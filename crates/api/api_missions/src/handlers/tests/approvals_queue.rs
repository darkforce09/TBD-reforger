//! Source pins for the approval-queue SQL contracts that cannot be checked without a database
//! otherwise: the queue's unique paging order, and the `updated_at` bump every review decision
//! owes every other status write.

use super::LIST_APPROVALS_SQL;

/// LIMIT/OFFSET over tied COALESCE keys needs a unique trailing key.
#[test]
fn list_approvals_sql_orders_by_id_after_submitted_at() {
    let order_idx = LIST_APPROVALS_SQL
        .find("ORDER BY")
        .expect("LIST_APPROVALS_SQL must ORDER BY");
    let order = &LIST_APPROVALS_SQL[order_idx..];
    assert!(
        order.contains("ASC,") && order.contains("m.id ASC"),
        "approvals queue ORDER BY must end with unique `, m.id ASC` tiebreaker; got: {order}"
    );
    // Guard against a regression that puts id first or drops the timestamp key.
    assert!(
        order.find("COALESCE").expect("COALESCE in ORDER BY")
            < order.find("m.id ASC").expect("m.id ASC in ORDER BY"),
        "m.id ASC must trail the COALESCE submitted_at key, not replace it"
    );
}

/// Review decisions must bump `missions.updated_at`, matching sibling status writes.
///
/// Perturbation RED: strip `updated_at = now()` from the decision's mission UPDATE → this test
/// fails.
#[test]
fn review_decisions_bump_updated_at() {
    const DECISIONS: &str = include_str!("../../services/mission_reviews.rs");
    let update = DECISIONS
        .split("UPDATE missions SET status")
        .nth(1)
        .expect("the decision updates the mission status");
    let statement = update.split("WHERE id = $1").next().unwrap();
    assert!(
        statement.contains("updated_at = now()"),
        "the decision's mission UPDATE must set updated_at = now(); got: {statement}"
    );
}
