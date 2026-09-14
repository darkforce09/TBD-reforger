//! The wiki's pure helpers and the guard on its administrator affordances.

use super::super::category_nav::category_order;
use super::super::markdown_article::updated_day;
use super::resolve_slug;
use serde_json::json;

#[test]
fn slug_resolution_falls_back_to_first() {
    let pages = vec![
        json!({"slug": "field-manual", "category": "Doctrine", "title": "Field Manual"}),
        json!({"slug": "radio-procedure", "category": "Doctrine", "title": "Radio"}),
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
}

#[test]
fn categories_preserve_first_seen_order() {
    let pages = vec![
        json!({"slug": "a", "category": "Doctrine"}),
        json!({"slug": "b", "category": "Administration"}),
        json!({"slug": "c", "category": "Doctrine"}),
    ];
    assert_eq!(
        category_order(&pages),
        vec!["Doctrine".to_string(), "Administration".to_string()]
    );
}

#[test]
fn updated_day_takes_iso_prefix() {
    assert_eq!(updated_day("2026-07-14T10:12:00Z"), "2026-07-14");
    assert_eq!(updated_day(""), "—");
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
                    while let Some(n) = chars.next() {
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
    let production = crate::v2::core::test_support::pins::wiki_source();
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
