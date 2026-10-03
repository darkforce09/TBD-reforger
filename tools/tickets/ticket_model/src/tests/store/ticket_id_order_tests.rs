use super::*;

/// Sorts `ids` by [`ticket_id_order_key`] and returns them.
fn in_key_order(ids: &[&str]) -> Vec<String> {
    let mut sorted: Vec<String> = ids.iter().map(|id| (*id).to_string()).collect();
    sorted.sort_by(|a, b| ticket_id_order_key(a.as_str()).cmp(&ticket_id_order_key(b.as_str())));
    sorted
}

/// Sorts `ids` by plain string order and returns them.
fn in_string_order(ids: &[&str]) -> Vec<String> {
    let mut sorted: Vec<String> = ids.iter().map(|id| (*id).to_string()).collect();
    sorted.sort();
    sorted
}

#[test]
fn a_four_digit_parent_sorts_after_every_three_digit_parent() {
    assert_eq!(
        in_key_order(&["T-1000", "T-999", "T-101", "T-100"]),
        ["T-100", "T-101", "T-999", "T-1000"],
    );
    assert!(ticket_id_order_key("T-999") < ticket_id_order_key("T-1000"));
    assert!(ticket_id_order_key("T-101") < ticket_id_order_key("T-1000"));
}

#[test]
fn children_keep_string_order_inside_their_parent() {
    let ids = [
        "T-069",
        "T-068.2",
        "T-068",
        "T-068.10.5",
        "T-067.9",
        "T-068.10",
    ];
    let expected = [
        "T-067.9",
        "T-068",
        "T-068.10",
        "T-068.10.5",
        "T-068.2",
        "T-069",
    ];
    assert_eq!(in_key_order(&ids), expected);
    assert_eq!(
        in_key_order(&ids),
        in_string_order(&ids),
        "three-digit parents order exactly as a plain string sort"
    );
}

#[test]
fn a_child_sorts_inside_its_parents_run() {
    assert_eq!(
        in_key_order(&["T-1001", "T-1000.1", "T-999.12", "T-1000", "T-100.4"]),
        ["T-100.4", "T-999.12", "T-1000", "T-1000.1", "T-1001"],
    );
}

#[test]
fn an_id_without_a_parent_numeral_sorts_last() {
    assert_eq!(
        in_key_order(&["X-1", "T-1000", "T-abc", "T-", "T-100"]),
        ["T-100", "T-1000", "T-", "T-abc", "X-1"],
    );
}

#[test]
fn borrowed_and_owned_keys_carry_the_same_order() {
    assert_eq!(ticket_id_order_key("T-1000.2"), (1000, "T-1000.2"));
    assert_eq!(
        ticket_id_order_key(String::from("T-068.10.5")),
        (68, String::from("T-068.10.5"))
    );
}
