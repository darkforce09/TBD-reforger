use super::*;

#[test]
fn the_mirror_matches_the_frontend() {
    assert_eq!(object_alias_slug("Ammo Box US 01"), "ammo_box_us_01");
    assert_eq!(object_alias_slug("  -- Weird -- "), "weird");
    assert_eq!(object_alias_slug("!!!"), "object", "empty falls back");
    let known = derive_object_alias(KNOWN_CHECKPOINT_GUID, "E Sandbag Barricade US 04");
    assert_eq!(known, POC_ALIAS, "the KNOWN reverse-hit beats the slug");
    let path = "{A}PrefabsEditable/Compositions/X.et";
    assert_eq!(derive_object_alias(path, "Fuel Depot"), "comp:fuel_depot");
    let p2 = "{A}Prefabs/P.et";
    assert_eq!(derive_object_alias(p2, "Fuel Depot"), "prop:fuel_depot");
}
