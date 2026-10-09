use super::{PresentationField, RowShape, is_acceptable_thumbnail_url};

fn row() -> RowShape {
    RowShape {
        game_mode: "pve_coop".into(),
        max_players: 64,
        briefing: "Hold the bridge.".into(),
        thumbnail_url: "https://cdn.example/op.jpg".into(),
    }
}

/// The two variants are the two `PatchMissionInput` members. A column name that drifts from the
/// server's spelling ships a control whose PATCH is silently ignored — the handler builds its
/// UPDATE from named `Option`s, so an unknown key is dropped, not rejected. That failure is
/// invisible: the optimistic write stays on screen and the 200 confirms nothing.
#[test]
fn presentation_columns_are_the_patch_body_keys() {
    assert_eq!(PresentationField::Briefing.column(), "briefing");
    assert_eq!(PresentationField::Thumbnail.column(), "thumbnail_url");
    for f in [PresentationField::Briefing, PresentationField::Thumbnail] {
        assert!(
            !f.what().trim().is_empty(),
            "T-671: {f:?} has no name to put in a failure toast"
        );
    }
}

/// `read`/`write` address the field they name and nothing else — the property the shared setter
/// rests on. Perturbation this catches: a copy-paste in [`PresentationField::write`] that sends
/// the thumbnail into the briefing column (or vice versa), which would silently destroy one
/// value while appearing to save the other.
#[test]
fn each_field_addresses_only_its_own_column() {
    let mut r = row();
    PresentationField::Briefing.write(&mut r, "New orders.".into());
    assert_eq!(PresentationField::Briefing.read(&r), "New orders.");
    assert_eq!(
        PresentationField::Thumbnail.read(&r),
        "https://cdn.example/op.jpg",
        "T-671: writing the briefing must not touch the thumbnail"
    );
    PresentationField::Thumbnail.write(&mut r, "https://cdn.example/two.png".into());
    assert_eq!(
        PresentationField::Briefing.read(&r),
        "New orders.",
        "T-671: writing the thumbnail must not touch the briefing"
    );
    assert_eq!(r.game_mode, "pve_coop", "T-671: neither touches the shape");
    assert_eq!(r.max_players, 64);
}

/// The pre-flight agrees with `handlers/missions.rs::validated_thumbnail_url`: empty clears the
/// column, an absolute http/https URL is stored, everything else is refused. The `/uploads/…`
/// case is the load-bearing one — that is the shape the platform's only upload endpoint returns,
/// and it is precisely what this column cannot hold.
#[test]
fn thumbnail_accept_rule_matches_the_write_boundary() {
    for ok in [
        "",
        "   ",
        "https://cdn.example/t.jpg",
        "http://cdn.example/t.jpg",
        "https://cdn.example/a/b?c=d#e",
    ] {
        assert!(
            is_acceptable_thumbnail_url(ok),
            "T-671: {ok:?} is stored by validated_thumbnail_url and must not be refused here"
        );
    }
    for bad in [
        "javascript:alert(1)",
        "data:image/png;base64,AAAA",
        "/uploads/2f1c.png",
        "//evil.example/t.jpg",
        "t.jpg",
        "ftp://cdn.example/t.jpg",
        "https://",
    ] {
        assert!(
            !is_acceptable_thumbnail_url(bad),
            "T-671: {bad:?} would be refused by the server — refuse it beside the field"
        );
    }
}
