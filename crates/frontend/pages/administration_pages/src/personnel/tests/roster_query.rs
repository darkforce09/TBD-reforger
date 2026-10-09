//! Guards on the roster address and the pager over it: URL parsing with its fallbacks, the
//! canonical URL query, the request path, the moves that start again at the first page, the page
//! count, the neighbours and the past-the-end fallback, including the served golden page.

use super::{DEFAULT_PER_PAGE, FIRST_PAGE, PER_PAGE_OPTIONS, RosterQuery};
use crate::personnel::roster_pager::PagerPosition;
use frontend_api_dtos::administration::PersonnelPage;
use frontend_test_support::fixtures::golden;

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
    assert!(
        PER_PAGE_OPTIONS
            .iter()
            .all(|(value, _)| value.parse::<i64>().is_ok_and(|n| (1..=100).contains(&n)))
    );
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

/// A pager position: the page shown, the page size and the matching total.
fn at(page: i64, per_page: i64, total: i64) -> PagerPosition {
    PagerPosition {
        page,
        per_page,
        total,
    }
}

#[test]
fn personnel_pagination_page_count_rounds_up_and_never_drops_below_one() {
    assert_eq!(
        at(1, 20, 0).page_count(),
        1,
        "nobody matching is still one page"
    );
    assert_eq!(at(1, 20, 1).page_count(), 1);
    assert_eq!(at(1, 20, 20).page_count(), 1);
    assert_eq!(at(1, 20, 21).page_count(), 2);
    assert_eq!(at(1, 10, 57).page_count(), 6);
    assert_eq!(at(1, 100, 100).page_count(), 1);
    assert_eq!(at(1, 100, 101).page_count(), 2);
    assert_eq!(
        at(1, 0, 50).page_count(),
        1,
        "a size below one cannot divide"
    );
    assert_eq!(at(1, 20, -3).page_count(), 1);
}

#[test]
fn personnel_pagination_previous_is_absent_on_the_first_page() {
    assert_eq!(at(1, 20, 57).previous_page(), None);
    assert_eq!(at(2, 20, 57).previous_page(), Some(1));
    assert_eq!(at(3, 20, 57).previous_page(), Some(2));
}

#[test]
fn personnel_pagination_next_is_absent_on_the_last_page() {
    assert_eq!(at(1, 20, 57).next_page(), Some(2));
    assert_eq!(at(2, 20, 57).next_page(), Some(3));
    assert_eq!(at(3, 20, 57).next_page(), None);
    assert_eq!(
        at(1, 20, 0).next_page(),
        None,
        "an empty roster has no next page"
    );
    assert_eq!(
        at(1, 20, 20).next_page(),
        None,
        "an exactly full page is the last"
    );
}

#[test]
fn personnel_pagination_past_the_end_is_beyond_the_last_filled_page() {
    assert!(
        !at(1, 20, 0).is_past_the_end(),
        "page 1 of an empty roster is in range"
    );
    assert!(!at(3, 20, 57).is_past_the_end());
    assert!(at(4, 20, 57).is_past_the_end());
    assert!(at(2, 20, 0).is_past_the_end());
}

#[test]
fn personnel_pagination_past_the_end_page_falls_back_to_the_first_of_the_same_search() {
    let current = address(9, 50, "vance");
    assert_eq!(
        at(9, 50, 6).fallback_for(&current),
        Some(address(1, 50, "vance"))
    );
    assert_eq!(
        at(1, 50, 6).fallback_for(&address(1, 50, "vance")),
        None,
        "a page in range stays"
    );
    assert_eq!(
        at(9, 20, 6).fallback_for(&current),
        None,
        "an answer served for another page size is stale and acts on nothing"
    );
    assert_eq!(
        at(8, 50, 6).fallback_for(&current),
        None,
        "an answer served for another page is stale and acts on nothing"
    );
}

#[test]
fn personnel_pagination_golden_page_is_one_page_with_both_moves_disabled() {
    // The DOM oracle answers every roster address with this golden: one page, no moves.
    let served: PersonnelPage = serde_json::from_str(golden!("GET__admin__users.json"))
        .expect("the roster golden decodes as a personnel page");
    let position = PagerPosition::served(&served);
    assert_eq!(position, at(1, 20, 6));
    assert_eq!(served.items.len(), 6);
    assert_eq!(position.previous_page(), None);
    assert_eq!(position.next_page(), None);
    assert!(!position.is_past_the_end());
}
