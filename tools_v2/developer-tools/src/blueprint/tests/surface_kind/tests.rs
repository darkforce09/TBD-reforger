use super::*;

#[test]
fn projectile_presets_follow_the_collision_layer_table() {
    for fire in [
        "FireGeo",
        "FireView",
        "BuildingFire",
        "BuildingFireView",
        "TreeFireView",
        "RockFireView",
        "PropFireView",
        "GlassFire",
        "Wheel",
        "Terrain",
    ] {
        assert_eq!(preset_stops_projectile(fire), Some(true), "{fire}");
    }
    for shell in [
        "Building",
        "Tree",
        "Prop",
        "PropView",
        "Door",
        "Ladder",
        "Vehicle",
        "Debris",
        "TreePart",
        "CharNoCollide",
    ] {
        assert_eq!(preset_stops_projectile(shell), Some(false), "{shell}");
    }
    // Soft layers keep their soft kinds; panes stay panes.
    for soft in ["Foliage", "Bush", "Glass"] {
        assert_eq!(preset_stops_projectile(soft), Some(true), "{soft}");
    }
    assert_eq!(preset_stops_projectile("?"), None);
    assert_eq!(preset_stops_projectile("SomethingNew"), None);
}

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
