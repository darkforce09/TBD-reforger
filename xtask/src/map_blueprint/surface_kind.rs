//! Game material → [`SurfaceKind`] (T-090.11.2). The COLL chunk's subrange table names a
//! `Common/Materials/Game/*.gamemat` per triangle run (see `xob.rs`); the stem of that path
//! decides the class. Never derived from visual `.emat` names — the visual LODs and the
//! collision mesh are different triangle sets.

use map_engine_core::bvh::SurfaceKind;

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
mod tests {
    use super::*;

    #[test]
    fn gamemat_stems_classify() {
        assert_eq!(
            kind_for_gamemat("{EA270CE454C419FD}Common/Materials/Game/wood.gamemat"),
            SurfaceKind::Opaque
        );
        assert_eq!(
            kind_for_gamemat("Common/Materials/Game/glass.gamemat"),
            SurfaceKind::Glass
        );
        assert_eq!(
            kind_for_gamemat("Common/Materials/Game/Glass_Armored.gamemat"),
            SurfaceKind::Glass
        );
        assert_eq!(kind_for_gamemat("plexiglass.gamemat"), SurfaceKind::Glass);
        assert_eq!(
            kind_for_gamemat("Common/Materials/Game/foliage_conifer.gamemat"),
            SurfaceKind::Foliage
        );
        assert_eq!(
            kind_for_gamemat("grass_lush_tall.gamemat"),
            SurfaceKind::Foliage
        );
        assert_eq!(kind_for_gamemat("moss.gamemat"), SurfaceKind::Foliage);
        assert_eq!(kind_for_gamemat("tiles_roof.gamemat"), SurfaceKind::Opaque);
        assert_eq!(
            kind_for_gamemat("Common/Materials/Game/Tree/bark.gamemat"),
            SurfaceKind::Opaque
        );
        assert_eq!(gamemat_stem("{X}A/B/wood_floor.gamemat"), "wood_floor");
        assert_eq!(kind_for_layer("GlassFire"), Some(SurfaceKind::Glass));
        assert_eq!(kind_for_layer("Foliage"), Some(SurfaceKind::Foliage));
        assert_eq!(kind_for_layer("Bush"), Some(SurfaceKind::Foliage));
        assert_eq!(kind_for_layer("Building"), None);
        assert_eq!(
            parse_kind_override("3=glass"),
            Some((3, SurfaceKind::Glass))
        );
        assert_eq!(
            parse_kind_override("1 = Foliage"),
            Some((1, SurfaceKind::Foliage))
        );
        assert_eq!(parse_kind_override("x=glass"), None);
        assert_eq!(parse_kind_override("1=rock"), None);
    }
}
