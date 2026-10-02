//! Game material → [`SurfaceKind`]. The COLL chunk's subrange table names a
//! `Common/Materials/Game/*.gamemat` per triangle run (see [`super::super::mesh_decoding`]); the stem of that path
//! decides the class. Never derived from visual `.emat` names — the visual LODs and the
//! collision mesh are different triangle sets.

use website_map_engine::spatial::bvh::surface::SurfaceKind;

/// Classify a game-material path (or bare stem). Case-insensitive on the stem.
///
/// | stem | kind |
/// |---|---|
/// | `glass*`, `plexiglass*` | Glass |
/// | `foliage*`, `grass*`, `moss*`, `seaweed*` | Foliage |
/// | anything else (wood, brick, metal, concrete, …) | Opaque |
#[must_use]
pub fn kind_for_gamemat(path: &str) -> SurfaceKind {
    let stem = gamemat_stem(path);
    if stem.starts_with("glass") || stem.starts_with("plexiglass") {
        SurfaceKind::Glass
    } else if stem.starts_with("foliage")
        || stem.starts_with("grass")
        || stem.starts_with("moss")
        || stem.starts_with("seaweed")
    {
        SurfaceKind::Foliage
    } else {
        SurfaceKind::Opaque
    }
}

/// `{GUID}Common/Materials/Game/glass_armored.gamemat` → `glass_armored` (lower-case).
#[must_use]
pub fn gamemat_stem(path: &str) -> String {
    let p = path.replace('\\', "/");
    let p = match p.find('}') {
        Some(i) if p.starts_with('{') => &p[i + 1..],
        _ => p.as_str(),
    };
    let leaf = p.rsplit('/').next().unwrap_or(p);
    let stem = leaf.split('.').next().unwrap_or(leaf);
    stem.to_ascii_lowercase()
}

/// A collider record's layer-preset name (`Building`, `FireView`, `Glass`, `Foliage`,
/// `Bush`, `Tree`, `Door`, …) as a second opinion: `Glass*` → Glass, `Foliage` / `Bush` →
/// Foliage, else `None` (no opinion — the game material decides).
#[must_use]
pub fn kind_for_layer(layer: &str) -> Option<SurfaceKind> {
    let l = layer.to_ascii_lowercase();
    if l.starts_with("glass") {
        Some(SurfaceKind::Glass)
    } else if l == "foliage" || l == "bush" {
        Some(SurfaceKind::Foliage)
    } else {
        None
    }
}

/// Does a collider on this layer preset stop a projectile? The Workbench collision-layer table
/// (BIKI "Arma Reforger: Collision Layer"): presets that carry the FireGeometry layer —
/// `FireGeo`, every `*Fire*` preset (`BuildingFire`, `FireView`, `TreeFireView`, `RockFireView`,
/// `PropFireView`, `GlassFire`, …), `Wheel`, the terrain — do; the plain Static / Dynamic
/// presets (`Building`, `Tree`, `Prop`, `PropView`, `Door`, `Ladder`, `Vehicle`, the character
/// layers, `Debris`, …) are the character / vehicle physics shells bullets pass through — the
/// coarse trunk box beside a tree's fire trunk, the 6 m box around a pole fence. `Foliage` /
/// `Bush` (soft, concealment) and `Glass*` (a pane) keep their soft kinds. `None` = an unknown
/// name, kept (no opinion). Pinned by the oracle: dropping the static shells took
/// the village cell from 97.9 % to the measured number in that commit.
#[must_use]
pub fn preset_stops_projectile(layer: &str) -> Option<bool> {
    let l = layer.to_ascii_lowercase();
    if l.contains("fire") || l == "wheel" || l == "terrain" {
        return Some(true);
    }
    if l == "foliage" || l == "bush" || l.starts_with("glass") {
        return Some(true);
    }
    const STATIC_OR_DYNAMIC: &[&str] = &[
        "building",
        "tree",
        "treepart",
        "prop",
        "propview",
        "door",
        "ladder",
        "vehicle",
        "character",
        "characterai",
        "charnocollide",
        "debris",
        "interaction",
        "main",
        "cover",
        "projectile",
        "weapon",
        "item",
        "itemview",
        "static",
        "dynamic",
    ];
    if STATIC_OR_DYNAMIC.contains(&l.as_str()) {
        return Some(false);
    }
    None
}

/// Parse a `--kind <record>=<kind>` override.
pub fn parse_kind_override(s: &str) -> Option<(u16, SurfaceKind)> {
    let (rec, kind) = s.split_once('=')?;
    let rec: u16 = rec.trim().parse().ok()?;
    let kind = match kind.trim().to_ascii_lowercase().as_str() {
        "opaque" | "0" => SurfaceKind::Opaque,
        "glass" | "1" => SurfaceKind::Glass,
        "foliage" | "2" => SurfaceKind::Foliage,
        _ => return None,
    };
    Some((rec, kind))
}

#[cfg(test)]
#[path = "../tests/surface_kind/tests.rs"]
mod tests;
