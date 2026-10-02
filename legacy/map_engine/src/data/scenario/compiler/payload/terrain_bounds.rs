//! Role: terrain bounds.
//! Position: `mission/compiler/payload` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Terrain bounds using the supplied domain data.
#[must_use]
pub fn terrain_bounds(terrain: &str) -> [f64; 4] {
    match terrain {
        "arland" => [0.0, 0.0, 4096.0, 4096.0],

        _ => [0.0, 0.0, 12_800.0, 12_800.0],
    }
}
