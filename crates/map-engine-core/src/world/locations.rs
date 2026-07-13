//! Parse `locations.json` into [`LocationLabel`] rows (T-152.8).

#![forbid(unsafe_code)]

use crate::label::LabelSpec;
use crate::world::importance_declutter::LocationLabel;

/// Parse a `locations.json` array payload.
///
/// # Errors
/// Returns a message when JSON is not an array of location objects.
pub fn parse_locations_json(json: &str) -> Result<Vec<LocationLabel>, String> {
    serde_json::from_str(json).map_err(|e| format!("locations json: {e}"))
}

/// Map locations → text [`LabelSpec`] for glyph packing (importance scaled to u16).
#[must_use]
pub fn locations_to_label_specs(locations: &[LocationLabel]) -> Vec<LabelSpec> {
    locations
        .iter()
        .enumerate()
        .map(|(i, loc)| LabelSpec {
            id: i as u32,
            x: loc.x.round() as i32,
            y: loc.y.round() as i32,
            importance: (loc.importance.clamp(0.0, 1.0) * 10_000.0).round() as u16,
            text: loc.name.trim().to_string(),
        })
        .filter(|s| !s.text.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_row() {
        let json = r#"[
          {"id":"everon-morton","name":"Morton","x":5135.24,"y":4011.78,"importance":0.7,"kind":"village"}
        ]"#;
        let rows = parse_locations_json(json).expect("parse");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Morton");
        let specs = locations_to_label_specs(&rows);
        assert_eq!(specs[0].text, "Morton");
    }
}
