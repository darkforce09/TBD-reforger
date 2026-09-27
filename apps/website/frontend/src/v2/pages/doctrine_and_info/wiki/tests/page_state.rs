//! A draft remembers the revision its edit started from.

use super::*;

#[test]
fn wiki_draft_starts_from_the_article_revision_it_was_begun_on() {
    let draft = updated_draft(None, 4, "# Field Manual".into());
    assert_eq!(
        draft,
        WikiDraft {
            body_md: "# Field Manual".into(),
            base_revision: 4,
        }
    );
}

#[test]
fn wiki_draft_keeps_its_base_revision_when_the_article_moves_on() {
    let begun = updated_draft(None, 4, "# Field".into());
    // The article reloads at revision 6 while the author keeps typing.
    let continued = updated_draft(Some(&begun), 6, "# Field Manual".into());
    assert_eq!(continued.base_revision, 4);
    assert_eq!(continued.body_md, "# Field Manual");
}
