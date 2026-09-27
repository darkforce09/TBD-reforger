//! Guards on the pager arithmetic: the page count, the neighbours, the past-the-end fallback and
//! the captions, including the served golden page.

use super::{member_count_caption, PagerPosition};
use crate::v2::core::api::dto::administration::PersonnelPage;
use crate::v2::core::test_support::fixtures::golden;
use crate::v2::pages::administration::personnel::roster_query::RosterQuery;

fn at(page: i64, per_page: i64, total: i64) -> PagerPosition {
    PagerPosition {
        page,
        per_page,
        total,
    }
}

fn address(page: i64, per_page: i64, q: &str) -> RosterQuery {
    RosterQuery {
        page,
        per_page,
        q: q.to_string(),
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
fn personnel_pagination_captions_name_the_position_and_the_count() {
    assert_eq!(at(1, 20, 0).caption(), "Page 1 of 1");
    assert_eq!(at(2, 10, 57).caption(), "Page 2 of 6");
    assert_eq!(member_count_caption(0), "0 members");
    assert_eq!(member_count_caption(1), "1 member");
    assert_eq!(member_count_caption(6), "6 members");
}

#[test]
fn personnel_pagination_golden_page_is_one_page_with_both_moves_disabled() {
    // The DOM oracle answers every roster address with this golden, so the pager it captures
    // reads "Page 1 of 1" with previous and next both disabled.
    let served: PersonnelPage = serde_json::from_str(golden!("GET__admin__users.json"))
        .expect("the roster golden decodes as a personnel page");
    let position = PagerPosition::served(&served);
    assert_eq!(position, at(1, 20, 6));
    assert_eq!(served.items.len(), 6);
    assert_eq!(position.caption(), "Page 1 of 1");
    assert_eq!(member_count_caption(position.total), "6 members");
    assert_eq!(position.previous_page(), None);
    assert_eq!(position.next_page(), None);
    assert!(!position.is_past_the_end());
}
