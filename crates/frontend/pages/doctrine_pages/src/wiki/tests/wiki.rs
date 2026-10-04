//! The wiki's route helpers and the guards on its administrator affordances and its rendering.

use super::super::category_nav::category_order;
use super::resolve_slug;
use frontend_api_dtos::DataEnvelope;
use frontend_api_dtos::wiki::WikiPageSummary;
use frontend_test_support::fixtures::golden;

/// A summary with only the fields these tests read.
fn summary(slug: &str, category: &str, title: &str) -> WikiPageSummary {
    WikiPageSummary {
        slug: slug.into(),
        category: category.into(),
        title: title.into(),
        icon: String::new(),
        nav_order: 0,
        revision: 1,
        updated_at: "2026-07-14T10:12:00Z".into(),
    }
}

#[test]
fn slug_resolution_falls_back_to_first() {
    let pages = vec![
        summary("field-manual", "Doctrine", "Field Manual"),
        summary("radio-procedure", "Doctrine", "Radio"),
    ];
    assert_eq!(resolve_slug(&pages, None).as_deref(), Some("field-manual"));
    assert_eq!(
        resolve_slug(&pages, Some("radio-procedure")).as_deref(),
        Some("radio-procedure")
    );
    assert_eq!(
        resolve_slug(&pages, Some("nope")).as_deref(),
        Some("field-manual")
    );
    assert_eq!(resolve_slug(&[], Some("field-manual")), None);
}

#[test]
fn categories_preserve_first_seen_order() {
    let pages = vec![
        summary("a", "Doctrine", "A"),
        summary("b", "Administration", "B"),
        summary("c", "Doctrine", "C"),
        summary("d", "", "D"),
    ];
    assert_eq!(
        category_order(&pages),
        vec!["Doctrine".to_string(), "Administration".to_string()]
    );
}

#[test]
fn wiki_index_reads_the_captured_summaries_in_their_order() {
    let list: DataEnvelope<WikiPageSummary> =
        serde_json::from_str(golden!("GET__wiki.json")).unwrap();
    assert_eq!(
        resolve_slug(&list.data, None).as_deref(),
        Some("field-manual"),
        "the oracle's /wiki route opens the first manual"
    );
    assert_eq!(
        category_order(&list.data),
        vec!["Doctrine".to_string(), "Administration".to_string()]
    );
}

/// Strips `//` and `/* */` so a ban cannot trip on the text of a doc comment.
fn strip_rust_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    for n in chars.by_ref() {
                        if n == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '*' && matches!(chars.peek(), Some('/')) {
                            chars.next();
                            break;
                        }
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The wiki's edit and save affordances must not use the browse-mode role check, which treats
/// a signed-out visitor as permitted. Binds to the live `is_admin` memo and bans the free call.
#[test]
fn admin_affordance_uses_authed_reactive_role() {
    let production = crate::source_pins::wiki_source();
    let code = collapse_ws(&strip_rust_comments(&production));
    assert!(
        code.contains(
            "let is_admin = Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin))"
        ),
        "is_admin must be the Memo that re-reads AuthStore.user via has_min_role_authed \
         (dead Memo + browse-mode has_min_role is a fail)"
    );
    // Mask the authed helper so a free `has_min_role(` call stands out.
    let masked = code.replace("has_min_role_authed", "HAS_MIN_ROLE_AUTHED");
    assert!(
        !masked.contains("has_min_role("),
        "production must not call browse-mode has_min_role( — use has_min_role_authed only"
    );
    let one_shot = format!("store.has_min_role({}::Admin)", "Role");
    assert!(
        !code.contains(&one_shot),
        "one-shot store.has_min_role(Admin) freezes pre-bootstrap None as admin"
    );
}

/// The wiki renders authored content as text nodes and attributes only: no production file of
/// the page writes inner HTML.
#[test]
fn wiki_source_never_writes_inner_html() {
    let production = crate::source_pins::wiki_source();
    let code = strip_rust_comments(&production);
    assert!(
        !code.contains("inner_html"),
        "the wiki must never set inner_html; content renders as text nodes"
    );
    assert!(
        !code.contains("set_inner_html") && !code.contains("InnerHtml"),
        "the wiki must never reach for the inner-HTML attribute"
    );
}
