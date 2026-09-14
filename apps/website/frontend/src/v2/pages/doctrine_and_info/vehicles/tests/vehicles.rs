//! The vehicle index's grouping helper.

use super::faction_order;
use serde_json::json;

#[test]
fn factions_preserve_first_seen_order() {
    let rows = vec![
        json!({"name": "BTR-70", "faction": "USSR"}),
        json!({"name": "M113A3", "faction": "US Army"}),
        json!({"name": "UAZ-469", "faction": "USSR"}),
    ];
    assert_eq!(
        faction_order(&rows),
        vec!["USSR".to_string(), "US Army".to_string()]
    );
}
