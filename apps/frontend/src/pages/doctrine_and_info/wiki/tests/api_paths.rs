//! The wiki's request paths: the article, the paged history, one revision, and the page count.

use super::*;

#[test]
fn wiki_paths_name_the_article_its_history_page_and_one_revision() {
    assert_eq!(PAGE_LIST_PATH, "/wiki");
    assert_eq!(article_path("field-manual"), "/wiki/field-manual");
    assert_eq!(
        revision_list_path("field-manual", 2),
        "/wiki/field-manual/revisions?page=2&per_page=10"
    );
    assert_eq!(
        revision_path("field-manual", 3),
        "/wiki/field-manual/revisions/3"
    );
}

#[test]
fn wiki_paths_never_ask_for_a_history_page_before_the_first() {
    assert_eq!(
        revision_list_path("field-manual", 0),
        "/wiki/field-manual/revisions?page=1&per_page=10"
    );
    assert_eq!(
        revision_list_path("field-manual", -4),
        "/wiki/field-manual/revisions?page=1&per_page=10"
    );
}

#[test]
fn wiki_paths_keep_a_slug_to_one_segment() {
    assert_eq!(article_path("a/b?c#d"), "/wiki/a%2Fb%3Fc%23d");
    assert_eq!(article_path("../admin"), "/wiki/..%2Fadmin");
    assert_eq!(article_path("radio procedure"), "/wiki/radio%20procedure");
    assert_eq!(
        revision_path("é", 1),
        "/wiki/%C3%A9/revisions/1",
        "every byte of a multi-byte character is encoded"
    );
}

#[test]
fn wiki_paths_count_history_pages() {
    assert_eq!(revision_page_count(0, 10), 1);
    assert_eq!(revision_page_count(1, 20), 1);
    assert_eq!(revision_page_count(10, 10), 1);
    assert_eq!(revision_page_count(11, 10), 2);
    assert_eq!(revision_page_count(21, 10), 3);
    assert_eq!(
        revision_page_count(5, 0),
        5,
        "a zero page size counts as one per page"
    );
}
