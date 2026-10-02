//! The terrain identifier keeps the wire shape of a plain string.

use super::*;

#[test]
fn terrain_id_serialises_as_the_bare_string_in_both_directions() {
    let terrain = TerrainId::new("everon");
    assert_eq!(serde_json::to_string(&terrain).unwrap(), r#""everon""#);
    assert_eq!(
        serde_json::from_str::<TerrainId>(r#""arland""#).unwrap(),
        TerrainId::new("arland")
    );
    assert!(serde_json::from_str::<TerrainId>("7").is_err());
}

#[test]
fn terrain_id_displays_and_compares_as_its_spelling() {
    let terrain = TerrainId::new(String::from("everon"));
    assert_eq!(terrain.as_str(), "everon");
    assert_eq!(terrain.to_string(), "everon");
    assert_eq!(terrain, "everon");
    assert!(terrain != "arland");
}
