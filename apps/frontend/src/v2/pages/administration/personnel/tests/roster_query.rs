//! Guards on the roster address: URL parsing with its fallbacks, the canonical URL query, the
//! request path, and the moves that start again at the first page.

use super::{RosterQuery, DEFAULT_PER_PAGE, FIRST_PAGE, PER_PAGE_OPTIONS};

/// Parse a raw URL query string the way the browser hands it to the router.
fn parse(query: &str) -> RosterQuery {
    let pairs: Vec<(String, String)> =
        url::form_urlencoded::parse(query.trim_start_matches('?').as_bytes())
            .into_owned()
            .collect();
    let value = |key: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    RosterQuery::from_url_values(value("page"), value("per_page"), value("q"))
}

fn address(page: i64, per_page: i64, q: &str) -> RosterQuery {
    RosterQuery {
        page,
        per_page,
        q: q.to_string(),
    }
}

#[test]
fn personnel_pagination_empty_url_is_the_first_page_of_twenty() {
    assert_eq!(parse(""), RosterQuery::default());
    assert_eq!(RosterQuery::default(), address(1, 20, ""));
    assert_eq!((FIRST_PAGE, DEFAULT_PER_PAGE), (1, 20));
}

#[test]
fn personnel_pagination_reads_page_per_page_and_search() {
    assert_eq!(
        parse("?page=3&per_page=50&q=vance"),
        address(3, 50, "vance")
    );
    assert_eq!(parse("per_page=100&page=12"), address(12, 100, ""));
    assert_eq!(parse("?q=Sgt.+Vance"), address(1, 20, "Sgt. Vance"));
    assert_eq!(
        parse("?q=%5BTBD%5D%20Kessler"),
        address(1, 20, "[TBD] Kessler")
    );
}

#[test]
fn personnel_pagination_junk_or_out_of_range_page_falls_back_to_the_first() {
    for junk in [
        "0",
        "-1",
        "+2",
        " 2",
        "2 ",
        "1.5",
        "two",
        "",
        "99999999999999999999",
    ] {
        assert_eq!(
            RosterQuery::from_url_values(Some(junk), Some("50"), None),
            address(1, 50, ""),
            "page {junk:?}"
        );
    }
}

#[test]
fn personnel_pagination_per_page_outside_the_offered_sizes_falls_back_to_twenty() {
    for junk in [
        "0", "-10", "5", "30", "101", "1000", "020", "ten", "", " 50",
    ] {
        assert_eq!(
            RosterQuery::from_url_values(Some("4"), Some(junk), None),
            address(4, 20, ""),
            "per_page {junk:?}"
        );
    }
    for (offered, _) in PER_PAGE_OPTIONS {
        let expected: i64 = offered.parse().expect("offered sizes are numbers");
        assert_eq!(
            RosterQuery::from_url_values(None, Some(offered), None).per_page,
            expected
        );
    }
}

#[test]
fn personnel_pagination_offers_ten_twenty_fifty_and_a_hundred_within_the_api_ceiling() {
    let offered: Vec<&str> = PER_PAGE_OPTIONS.iter().map(|(value, _)| *value).collect();
    assert_eq!(offered, ["10", "20", "50", "100"]);
    assert!(PER_PAGE_OPTIONS.iter().any(|(value, _)| *value == "20"));
    assert!(PER_PAGE_OPTIONS
        .iter()
        .all(|(value, _)| value.parse::<i64>().is_ok_and(|n| (1..=100).contains(&n))));
}

#[test]
fn personnel_pagination_url_query_is_canonical_and_round_trips() {
    assert_eq!(RosterQuery::default().to_url_query(), "?page=1&per_page=20");
    assert_eq!(
        address(2, 50, "sgt vance").to_url_query(),
        "?page=2&per_page=50&q=sgt+vance"
    );
    for original in [
        address(1, 10, ""),
        address(7, 100, "  padded  "),
        address(3, 20, "a&b=c?d#e"),
        address(9, 50, "[TBD] Pvt. Brandt"),
        address(2, 20, "Ünïcödé"),
    ] {
        assert_eq!(parse(&original.to_url_query()), original);
    }
}

#[test]
fn personnel_pagination_api_path_sends_page_size_and_trimmed_search() {
    assert_eq!(
        RosterQuery::default().api_path(),
        "/admin/users?page=1&per_page=20"
    );
    assert_eq!(
        address(3, 100, "  okafor ").api_path(),
        "/admin/users?page=3&per_page=100&q=okafor"
    );
    assert_eq!(
        address(1, 10, "a&b").api_path(),
        "/admin/users?page=1&per_page=10&q=a%26b"
    );
    assert_eq!(
        address(2, 50, " \t ").api_path(),
        "/admin/users?page=2&per_page=50",
        "a blank search is not sent"
    );
}

#[test]
fn personnel_pagination_new_search_starts_again_at_the_first_page() {
    let next = address(5, 50, "old").with_search("new text ".to_string());
    assert_eq!(next, address(1, 50, "new text "));
    assert_eq!(
        address(5, 50, "old").with_search(String::new()),
        address(1, 50, "")
    );
}

#[test]
fn personnel_pagination_new_page_size_starts_again_at_the_first_page() {
    assert_eq!(
        address(4, 20, "vance").with_per_page("100"),
        address(1, 100, "vance")
    );
    assert_eq!(
        address(4, 50, "vance").with_per_page("junk"),
        address(1, 20, "vance")
    );
}

#[test]
fn personnel_pagination_page_moves_keep_search_and_size() {
    let current = address(2, 10, "tbd");
    assert_eq!(current.with_page(3), address(3, 10, "tbd"));
    assert_eq!(current.with_page(0), address(1, 10, "tbd"));
    assert_eq!(current.with_page(-4), address(1, 10, "tbd"));
}

#[test]
fn personnel_pagination_page_keeps_the_address_in_the_url() {
    // The route component must read the address from the URL and write it back in place,
    // replacing the history entry, and fetch the roster at the address's own request path.
    let production = crate::v2::core::test_support::pins::personnel_source();
    for needle in [
        "use_query_map()",
        "use_navigate()",
        "replace: true",
        "RosterQuery::from_url_values(",
        ".to_url_query()",
        ".api_path()",
        "api_get::<PersonnelPage>",
    ] {
        assert!(
            production.contains(needle),
            "the personnel route must keep its address in the URL: missing {needle}"
        );
    }
}
