//! Source pins for the two approval-queue SQL contracts that cannot be checked without a
//! database otherwise: the queue's unique paging order, and the `updated_at` bump both review
//! writes owe every other status write.

use super::{APPROVE_MISSION_SQL, LIST_APPROVALS_SQL, REJECT_MISSION_SQL};

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

/// Approve/reject must bump `missions.updated_at`, matching sibling status writes.
///
/// Perturbation RED: strip `updated_at = now()` from either const → this test fails.
#[test]
fn approve_and_reject_sql_bump_updated_at() {
    assert!(
        APPROVE_MISSION_SQL.contains("updated_at = now()"),
        "approve UPDATE must set updated_at = now(); got: {APPROVE_MISSION_SQL}"
    );
    assert!(
        REJECT_MISSION_SQL.contains("updated_at = now()"),
        "reject UPDATE must set updated_at = now(); got: {REJECT_MISSION_SQL}"
    );
}
