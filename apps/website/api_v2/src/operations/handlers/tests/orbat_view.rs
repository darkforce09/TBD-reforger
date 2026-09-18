//! Pins for the member directory's offset pagination: the page oracle the SQL must match, the
//! shared bounds it derives from, and the source shape of the query itself.

use crate::core::http::pagination::PageParams;

const SRC: &str = include_str!("../orbat_view.rs");

/// Pure page oracle over a sorted username list — mirrors SQL `ORDER BY username ASC
/// LIMIT $limit OFFSET $offset`. Member index 20 is invisible at offset 0 and appears
/// only when offset ≥ 20.
fn page_usernames<'a>(sorted: &'a [&str], limit: usize, offset: usize) -> Vec<&'a str> {
    sorted.iter().copied().skip(offset).take(limit).collect()
}

#[test]
fn member_at_index_20_requires_offset() {
    let names: Vec<String> = (0..25).map(|i| format!("user_{i:02}")).collect();
    let sorted: Vec<&str> = names.iter().map(String::as_str).collect();
    assert_eq!(sorted.len(), 25);

    let page0 = page_usernames(&sorted, 20, 0);
    assert_eq!(page0.len(), 20);
    assert!(
        !page0.contains(&"user_20"),
        "default first page must not include member at index 20"
    );

    let page_off = page_usernames(&sorted, 20, 20);
    assert!(
        page_off.contains(&"user_20"),
        "offset=20 must surface member at index 20"
    );
    assert_eq!(page_off.first().copied(), Some("user_20"));
}

#[test]
fn page_params_default_limit_is_20_offset_0() {
    assert_eq!(
        PageParams {
            limit: None,
            offset: None
        }
        .bounds(),
        (20, 0)
    );
    assert_eq!(
        PageParams {
            limit: Some(20),
            offset: Some(20)
        }
        .bounds(),
        (20, 20)
    );
}

/// Source ratchet: production `search_members` must bind LIMIT + OFFSET (never a hard-coded
/// `LIMIT 20` with no offset) and emit the list envelope fields.
#[test]
fn search_members_binds_limit_offset_and_list_envelope() {
    let production = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("production source before the sibling-test declaration");
    let handler = production
        .split("pub async fn search_members")
        .nth(1)
        .expect("search_members handler")
        .split("\npub async fn ")
        .next()
        .expect("handler body until next pub async fn");
    let collapsed: String = handler.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        !collapsed.contains("ORDER BY username ASC LIMIT 20\""),
        "search_members must not hard-code LIMIT 20 without OFFSET"
    );
    assert!(
        !collapsed.contains("ORDER BY username ASC LIMIT 20"),
        "search_members must not hard-code LIMIT 20 without OFFSET"
    );
    assert!(
        collapsed.contains("push_bind(limit)") && collapsed.contains("push_bind(offset)"),
        "search_members must bind limit and offset parameters"
    );
    assert!(
        collapsed.contains("OFFSET"),
        "search_members SQL must include OFFSET"
    );
    assert!(
        collapsed.contains("\"total\"")
            && collapsed.contains("\"limit\"")
            && collapsed.contains("\"offset\""),
        "search_members must return {{data,total,limit,offset}}"
    );
}
