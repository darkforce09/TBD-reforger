//! The guard on the modpacks page's administrator affordances.

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

/// The modpack create and edit affordances must not use the browse-mode role check, which
/// treats a signed-out visitor as permitted. Binds to the live `is_admin` memo and bans the
/// free call.
#[test]
fn admin_affordance_uses_authed_reactive_role() {
    let production = crate::v2::core::test_support::pins::modpacks_source();
    let code = collapse_ws(&strip_rust_comments(&production));
    assert!(
        code.contains(
            "let is_admin = Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin))"
        ),
        "is_admin must be the Memo that re-reads AuthStore.user via has_min_role_authed \
         (dead Memo + browse-mode has_min_role is a fail)"
    );
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
