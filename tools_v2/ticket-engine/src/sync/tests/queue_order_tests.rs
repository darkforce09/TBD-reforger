use super::*;

/// A ready ticket row with a spec, so the queue admits it.
fn ready(id: &str, order: i64) -> Value {
    json!({
        "id": id,
        "title": id,
        "status": "ready",
        "order": order,
        "spec": "spec.md",
    })
}

/// Ready tickets that share an `order` enter the queue in numeric id order, so a four-digit
/// id follows every three-digit id instead of landing between two three-digit ids.
#[test]
fn queue_breaks_order_ties_by_numeric_id() {
    let registry = json!({
        "next_id": 1001,
        "tickets": [
            ready("T-1000", 5),
            ready("T-101", 5),
            ready("T-100", 5),
            ready("T-999", 4),
        ],
    });
    let queue = generate_queue_json(&registry);
    let ids: Vec<&str> = queue["tickets"]
        .as_array()
        .expect("queue tickets")
        .iter()
        .map(|t| t["id"].as_str().expect("queue id"))
        .collect();
    assert_eq!(ids, ["T-999", "T-100", "T-101", "T-1000"]);
}
