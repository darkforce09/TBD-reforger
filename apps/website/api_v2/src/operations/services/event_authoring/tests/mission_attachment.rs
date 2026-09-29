//! Unit tests for the attachment template checks: a template that seats nobody and a faction no
//! armory line could match are refused before any slot is written, and an accepted template
//! counts its slots.

use axum::http::StatusCode;

use super::{AttachmentTemplate, template_from_payload};
use crate::operations::services::{OrbatSlotTemplate, OrbatSquadTemplate};

fn squad(faction: &str, slots: usize) -> OrbatSquadTemplate {
    OrbatSquadTemplate {
        faction: faction.to_owned(),
        callsign: "ALPHA".to_owned(),
        squad: "Alpha 1-1".to_owned(),
        slots: (0..slots)
            .map(|index| OrbatSlotTemplate {
                role: format!("Rifleman {index}"),
                loadout: String::new(),
                tag: String::new(),
            })
            .collect(),
    }
}

#[test]
fn attachment_template_refuses_a_template_that_seats_nobody() {
    for squads in [vec![], vec![squad("BLUFOR", 0), squad("OPFOR", 0)]] {
        let error = AttachmentTemplate::requested(squads).expect_err("nobody could be seated");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert!(
            error.message.starts_with("`orbat` describes no slots"),
            "{}",
            error.message
        );
    }
}

#[test]
fn attachment_template_refuses_a_faction_no_armory_line_could_match() {
    for faction in ["", "\t", "  USA  "] {
        let error = AttachmentTemplate::requested(vec![squad("BLUFOR", 1), squad(faction, 2)])
            .expect_err("an unusable join key");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert!(
            error.message.starts_with("orbat[1]."),
            "{faction:?}: {}",
            error.message
        );
    }
}

#[test]
fn attachment_template_counts_the_slots_it_seats() {
    let template = AttachmentTemplate::requested(vec![
        squad("BLUFOR", 8),
        squad("BLUFOR", 0),
        squad("US Army", 3),
    ])
    .expect("a template that seats eleven");
    assert_eq!(template.slot_count(), 11);
}

/// An explicit `orbat` that does not deserialize is a 400 naming the payload, never a silent
/// fall back to the editor-derived squads.
#[test]
fn attachment_template_reports_an_unreadable_published_orbat() {
    let error =
        template_from_payload(br#"{"orbat":[{"faction":7}]}"#).expect_err("an unreadable orbat");
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    let squads = template_from_payload(br#"{"orbat":[{"faction":"BLUFOR","slots":[{}]}]}"#)
        .expect("a readable orbat");
    assert_eq!(squads.len(), 1);
}
