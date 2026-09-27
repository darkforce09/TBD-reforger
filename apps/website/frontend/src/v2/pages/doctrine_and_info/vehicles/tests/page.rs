//! Guards on the vehicle index's administrator actions and on how its files write markup.

/// Every production file of the vehicle index, by name.
const PRODUCTION_FILES: [(&str, &str); 10] = [
    (
        "delete_confirmation.rs",
        include_str!("../delete_confirmation.rs"),
    ),
    ("mod.rs", include_str!("../mod.rs")),
    ("page.rs", include_str!("../page.rs")),
    ("spec_drawer.rs", include_str!("../spec_drawer.rs")),
    ("vehicle_draft.rs", include_str!("../vehicle_draft.rs")),
    (
        "vehicle_form_dialog.rs",
        include_str!("../vehicle_form_dialog.rs"),
    ),
    ("vehicle_grid.rs", include_str!("../vehicle_grid.rs")),
    ("vehicle_rows.rs", include_str!("../vehicle_rows.rs")),
    ("vehicle_writes.rs", include_str!("../vehicle_writes.rs")),
    ("write_refusal.rs", include_str!("../write_refusal.rs")),
];

/// Strips `//` and `/* */` comments, so a ban cannot trip on the text of a doc comment.
fn strip_rust_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
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

/// `src` with every run of whitespace collapsed to one space.
fn collapse_whitespace(src: &str) -> String {
    src.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The code of `file`, without comments and with whitespace collapsed.
fn code_of(file: &str) -> String {
    let (_, source) = PRODUCTION_FILES
        .iter()
        .find(|(name, _)| *name == file)
        .expect("the file is listed");
    collapse_whitespace(&strip_rust_comments(source))
}

/// The add, edit and delete actions must follow the signed-in role, never the browse-mode check
/// that treats a signed-out visitor as permitted.
#[test]
fn the_write_actions_follow_the_signed_in_administrator_role() {
    let code = code_of("page.rs");
    assert!(
        code.contains(
            "let is_admin = Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin))"
        ),
        "is_admin must be the memo that re-reads AuthStore.user through has_min_role_authed"
    );
    let masked = code.replace("has_min_role_authed", "HAS_MIN_ROLE_AUTHED");
    assert!(
        !masked.contains("has_min_role("),
        "the page must not call the browse-mode has_min_role("
    );
    assert!(
        code.contains("let actions = is_admin.get().then_some(actions);"),
        "the dossier receives its Edit and Delete actions only while is_admin holds"
    );
    assert!(
        code.contains("master_header(search, is_admin, on_add)"),
        "the add action is gated by the same memo"
    );
}

/// The add action renders only while the administrator memo holds.
#[test]
fn the_add_action_renders_only_for_an_administrator() {
    let code = code_of("vehicle_grid.rs");
    assert!(
        code.contains("is_admin .get() .then(||") || code.contains("is_admin.get().then(||"),
        "the header's add button must sit behind is_admin"
    );
}

/// Authored text reaches the page as text nodes and attributes only.
#[test]
fn no_vehicle_file_writes_raw_markup() {
    for (name, source) in PRODUCTION_FILES {
        let code = strip_rust_comments(source);
        assert!(!code.contains("inner_html"), "{name} writes inner_html");
    }
}

/// The delete confirmation describes the soft delete the backend performs.
#[test]
fn the_delete_confirmation_names_what_the_delete_does() {
    let code = code_of("delete_confirmation.rs");
    assert!(code.contains("title=DELETE_VEHICLE_TITLE"));
    assert!(code.contains("description=DELETE_VEHICLE_DESCRIPTION"));
    assert!(
        super::super::delete_confirmation::DELETE_VEHICLE_DESCRIPTION
            .contains("leaves the vehicle database for every member")
    );
}
