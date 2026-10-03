//! The vehicle form's validation, held to the backend validator's boundaries case for case.

use super::{trimmed, FormTarget, VehicleDraft, VehicleField};
use crate::foundation::transport::dto::vehicles::{Vehicle, VehicleWrite};

fn draft(name: &str, faction: &str, armor_type: &str) -> VehicleDraft {
    VehicleDraft {
        name: name.into(),
        faction: faction.into(),
        armor_type: armor_type.into(),
        ..VehicleDraft::default()
    }
}

fn problem_fields(draft: &VehicleDraft) -> Vec<VehicleField> {
    draft
        .validate()
        .expect_err("the draft must be refused")
        .into_iter()
        .map(|problem| problem.field)
        .collect()
}

fn stored_vehicle() -> Vehicle {
    Vehicle {
        id: "00000000-0000-4000-3000-000000000001".into(),
        name: "BTR-70".into(),
        faction: "USSR".into(),
        armor_type: "Light Armour".into(),
        amphibious: "Yes".into(),
        primary_threat: "Autocannon".into(),
        profile_image_url: "/uploads/btr70.png".into(),
    }
}

#[test]
fn the_limits_and_required_fields_are_the_backend_validators() {
    let table: Vec<(VehicleField, Option<usize>, bool)> = VehicleField::ALL
        .into_iter()
        .map(|field| (field, field.max_chars(), field.is_required()))
        .collect();
    assert_eq!(
        table,
        vec![
            (VehicleField::Name, Some(120), true),
            (VehicleField::Faction, Some(60), true),
            (VehicleField::ArmorType, Some(60), true),
            (VehicleField::Amphibious, Some(60), false),
            (VehicleField::PrimaryThreat, Some(120), false),
            (VehicleField::ProfileImageUrl, None, false),
        ]
    );
}

#[test]
fn a_valid_draft_becomes_a_write_body_with_every_value_trimmed() {
    let typed = VehicleDraft {
        name: "  M1A2 ".into(),
        faction: " BLUFOR".into(),
        armor_type: "MBT  ".into(),
        amphibious: "   ".into(),
        primary_threat: " ATGM ".into(),
        profile_image_url: " https://example.com/m1.png ".into(),
    };
    assert_eq!(
        typed.validate(),
        Ok(VehicleWrite {
            name: "M1A2".into(),
            faction: "BLUFOR".into(),
            armor_type: "MBT".into(),
            amphibious: String::new(),
            primary_threat: "ATGM".into(),
            profile_image_url: "https://example.com/m1.png".into(),
        })
    );
}

#[test]
fn blank_required_fields_are_refused_one_problem_each_in_form_order() {
    assert_eq!(
        problem_fields(&draft("  ", "", "\t\n")),
        vec![
            VehicleField::Name,
            VehicleField::Faction,
            VehicleField::ArmorType
        ]
    );
    assert_eq!(
        problem_fields(&draft("\u{feff}", "F", "A")),
        vec![VehicleField::Name],
        "a byte order mark alone is blank, as the backend trims it"
    );
    let problems = draft("", "F", "A").validate().unwrap_err();
    assert_eq!(problems[0].message, "Name is required.");
}

#[test]
fn optional_fields_may_be_empty_or_blank() {
    let mut typed = draft("BTR-70", "OPFOR", "APC");
    typed.amphibious = " ".into();
    typed.primary_threat = "\u{feff}".into();
    typed.profile_image_url = "  ".into();
    let body = typed.validate().expect("blank optionals are accepted");
    assert!(body.amphibious.is_empty());
    assert!(body.primary_threat.is_empty());
    assert!(body.profile_image_url.is_empty());
}

#[test]
fn each_limit_admits_its_boundary_and_refuses_one_character_more() {
    for field in VehicleField::ALL {
        let Some(limit) = field.max_chars() else {
            continue;
        };
        let mut at_limit = draft("N", "F", "A");
        at_limit.set_text(field, "x".repeat(limit));
        assert!(
            at_limit.validate().is_ok(),
            "{field:?}: {limit} characters fit"
        );

        let mut over = draft("N", "F", "A");
        over.set_text(field, "x".repeat(limit + 1));
        let problems = over.validate().expect_err("one character over is refused");
        assert_eq!(problems.len(), 1, "{field:?}");
        assert_eq!(problems[0].field, field);
        assert!(
            problems[0]
                .message
                .contains(&format!("at most {limit} characters")),
            "{field:?}: {}",
            problems[0].message
        );
    }
}

#[test]
fn limits_count_trimmed_characters_not_bytes() {
    let at_limit = "é".repeat(120);
    let padded = draft(&format!("  {at_limit}  "), "F", "A");
    let body = padded.validate().expect("120 two-byte characters fit");
    assert_eq!(body.name, at_limit);

    let over = draft(&"é".repeat(121), "F", "A");
    assert_eq!(problem_fields(&over), vec![VehicleField::Name]);
}

#[test]
fn profile_image_url_admits_https_and_site_paths_and_refuses_everything_else() {
    for url in [
        "https://example.com/a.png",
        "/uploads/a.webp",
        "",
        " https://example.com/padded.png ",
    ] {
        let mut typed = draft("N", "F", "A");
        typed.profile_image_url = url.into();
        let body = typed
            .validate()
            .unwrap_or_else(|_| panic!("{url:?} is accepted"));
        assert_eq!(body.profile_image_url, trimmed(url));
    }
    for url in [
        "http://example.com/a.png",
        "javascript:alert(1)",
        "data:image/png;base64,AAAA",
        "//evil.example/a.png",
        "https:///a.png",
        "/",
        "uploads/a.png",
        "https://example.com/a b.png",
        "HTTPS://example.com/a.png",
        "/uploads\\a.png",
    ] {
        let mut typed = draft("N", "F", "A");
        typed.profile_image_url = url.into();
        let problems = typed.validate().expect_err(&format!("{url:?} is refused"));
        assert_eq!(problems.len(), 1, "{url:?}");
        assert_eq!(problems[0].field, VehicleField::ProfileImageUrl);
        assert!(
            problems[0]
                .message
                .starts_with("Profile image URL must be an https:// URL"),
            "{url:?}: {}",
            problems[0].message
        );
    }
}

#[test]
fn a_new_vehicle_opens_empty_and_an_edit_opens_with_the_stored_values() {
    assert_eq!(FormTarget::Create.draft(), VehicleDraft::default());
    let vehicle = stored_vehicle();
    let opened = FormTarget::Edit(vehicle.clone()).draft();
    for field in VehicleField::ALL {
        let stored = match field {
            VehicleField::Name => &vehicle.name,
            VehicleField::Faction => &vehicle.faction,
            VehicleField::ArmorType => &vehicle.armor_type,
            VehicleField::Amphibious => &vehicle.amphibious,
            VehicleField::PrimaryThreat => &vehicle.primary_threat,
            VehicleField::ProfileImageUrl => &vehicle.profile_image_url,
        };
        assert_eq!(opened.text(field), stored, "{field:?}");
    }
    assert_eq!(FormTarget::Create.title(), "Add vehicle");
    assert_eq!(FormTarget::Edit(vehicle).title(), "Edit vehicle");
}

#[test]
fn the_target_picks_post_for_a_new_vehicle_and_put_for_a_stored_one() {
    let body = draft("N", "F", "A").validate().expect("valid");
    let create = FormTarget::Create.request(body.clone());
    assert_eq!(create.method(), "POST");
    assert_eq!(create.path(), "/vehicle-database");

    let replace = FormTarget::Edit(stored_vehicle()).request(body.clone());
    assert_eq!(replace.method(), "PUT");
    assert_eq!(
        replace.path(),
        "/vehicle-database/00000000-0000-4000-3000-000000000001"
    );
    assert_eq!(replace.body(), Some(&body));
}
