use crate::{Error, GameMode, TerrainType};

const TERRAINS: [TerrainType; 3] = [
    TerrainType::Everon,
    TerrainType::Arland,
    TerrainType::Custom,
];
const GAME_MODES: [GameMode; 3] = [GameMode::PveCoop, GameMode::Pvp, GameMode::Zeus];

#[test]
fn terrain_spelling_is_one_on_the_wire_in_sql_and_in_the_parse() {
    for terrain in TERRAINS {
        let json = serde_json::to_string(&terrain).unwrap();
        assert_eq!(json, format!("\"{}\"", terrain.as_str()));
        assert_eq!(terrain.as_str().parse::<TerrainType>(), Ok(terrain));
        assert_eq!(serde_json::from_str::<TerrainType>(&json).unwrap(), terrain);
    }
}

#[test]
fn game_mode_spelling_is_one_on_the_wire_in_sql_and_in_the_parse() {
    for mode in GAME_MODES {
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, format!("\"{}\"", mode.as_str()));
        assert_eq!(mode.as_str().parse::<GameMode>(), Ok(mode));
        assert_eq!(serde_json::from_str::<GameMode>(&json).unwrap(), mode);
    }
}

#[test]
fn unknown_and_differently_spelled_values_are_refused_as_given() {
    for refused in ["", "Everon", " everon", "malden"] {
        assert_eq!(
            refused.parse::<TerrainType>(),
            Err(Error::UnknownTerrain(refused.to_string()))
        );
    }
    for refused in ["", "PvP", "pve-coop", "coop"] {
        assert_eq!(
            refused.parse::<GameMode>(),
            Err(Error::UnknownGameMode(refused.to_string()))
        );
    }
}
