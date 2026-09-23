//! A fleet scenario in words, and the checks a registration passes before it is sent.
//!
//! **Role:** validates a terrain key, a scenario header id and a display name exactly as the
//! backend does, builds the registration they make, and states who last changed a mapping and when.
//! **Position:** read by the fleet scenario sheet's form and list.
//! **Signals & state:** none; pure over its arguments.
//! **Invariants:** the checks mirror the backend and the contract — a terrain key is lowercase
//! letters, digits and underscores starting with a letter, at most 64 bytes; a scenario id is
//! `{sixteen uppercase hex digits}` then a path of letters, digits, `_`, `.`, `/` or `-` ending in
//! `.conf`; a display name is trimmed and 1 to 128 bytes — so a registration the backend would
//! refuse is never sent.

use crate::v2::core::api::dto::{FleetScenario, FleetScenarioUpdate};
use crate::v2::core::utils::utc_timestamp::utc_label;

/// A terrain key as the compiler writes it, or what is wrong with it.
pub(crate) fn validated_terrain_key(text: &str) -> Result<String, String> {
    let key = text.trim();
    let well_formed = !key.is_empty()
        && key.len() <= 64
        && key.starts_with(|c: char| c.is_ascii_lowercase())
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if well_formed {
        Ok(key.to_string())
    } else {
        Err(
            "A terrain key is lowercase letters, digits and underscores, starting with a letter, \
             at most 64 bytes — for example arland"
                .to_string(),
        )
    }
}

/// A scenario header resource, or what is wrong with it.
pub(crate) fn validated_scenario_id(text: &str) -> Result<String, String> {
    let id = text.trim();
    let well_formed = id
        .strip_prefix('{')
        .and_then(|rest| rest.split_once('}'))
        .is_some_and(|(guid, path)| {
            guid.len() == 16
                && guid
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
                && path.len() > ".conf".len()
                && path.ends_with(".conf")
                && path
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'/' | b'-'))
        });
    if well_formed {
        Ok(id.to_string())
    } else {
        Err(
            "A scenario id is a scenario header resource: {16 uppercase hex digits} then a .conf \
             path — for example {1111222233334444}Missions/TBD_Arland.conf"
                .to_string(),
        )
    }
}

/// A display name, trimmed, or what is wrong with it.
pub(crate) fn validated_display_name(text: &str) -> Result<String, String> {
    let name = text.trim();
    match name.len() {
        0 => Err("Name the scenario as operators should read it".to_string()),
        1..=128 => Ok(name.to_string()),
        _ => Err("A display name is at most 128 bytes".to_string()),
    }
}

/// The terrain a registration is for and the body it sends, or the first thing wrong with it.
pub(crate) fn scenario_registration(
    terrain_key: &str,
    scenario_id: &str,
    display_name: &str,
) -> Result<(String, FleetScenarioUpdate), String> {
    let terrain = validated_terrain_key(terrain_key)?;
    let update = FleetScenarioUpdate {
        scenario_id: validated_scenario_id(scenario_id)?,
        display_name: validated_display_name(display_name)?,
    };
    Ok((terrain, update))
}

/// Who last changed a mapping, and when; the viewer's own change reads as "you".
pub(crate) fn updated_line(scenario: &FleetScenario, me: Option<&str>) -> String {
    let by = if me == Some(scenario.updated_by.as_str()) {
        "you".to_string()
    } else {
        scenario.updated_by.clone()
    };
    format!("Updated by {by}, {}", utc_label(&scenario.updated_at))
}
