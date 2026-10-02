use super::*;
use crate::blueprint::tests::fixture;
use crate::enfusion_pak::DirSource;

#[test]
fn tokenizer_and_block_shapes() {
    let src = r#"GenericEntity : "{0011}Prefabs/Base.et" {
 ID "AB12"
 components {
  MeshObject "{22}" { Object "{33}Assets/x.xob" LODFactors { 20 5 1 } }
  DoorComponent "{44}" : "{55}Prefabs/door.ct" { Enabled 0 AngleRange -120 DoorAnimationType WholeEntity }
  m_vCenter PointInfo "{66}" { Offset 1.5 2 -3 }
  "Additional hit zones" { SCR_WindowHitZone Default { "Kinetic multiplier" 4 } }
  Tags { "OpenGate" "Other" }
  Flags 0x403 0
 }
 {
  $grp Building : "{77}Prefabs/win.et" { { ID "1" coords 1 2 3 } { ID "2" angles 0 -90 0 scale 1.5 } }
  GenericEntity : "{88}Prefabs/table.et" { ID "3" components { Hierarchy "{99}" { PivotID "socket_a" AutoTransform 1 } } coords 0 0 -0.1 }
 }
}"#;
    let roots = parse_et(src).expect("parse");
    assert_eq!(roots.len(), 1);
    let r = &roots[0];
    assert_eq!(r.name, "GenericEntity");
    assert_eq!(r.base.as_deref(), Some("Prefabs/Base.et"));
    assert_eq!(r.prop("ID"), Some("AB12"));
    let c = r.block("components").unwrap();
    let mesh = c.block("MeshObject").unwrap();
    assert_eq!(mesh.guid.as_deref(), Some("{22}"));
    assert_eq!(mesh.prop("Object"), Some("{33}Assets/x.xob"));
    assert_eq!(
        mesh.block("LODFactors").unwrap().values,
        vec!["20", "5", "1"]
    );
    let door = c.block("DoorComponent").unwrap();
    assert_eq!(door.base.as_deref(), Some("Prefabs/door.ct"));
    assert_eq!(door.prop_f64("AngleRange"), Some(-120.0));
    assert_eq!(door.prop("DoorAnimationType"), Some("WholeEntity"));
    assert_eq!(door.prop("Enabled"), Some("0"));
    let center = c.block("m_vCenter").unwrap();
    assert_eq!(center.types, vec!["PointInfo"]);
    assert_eq!(center.prop_vec3("Offset"), Some([1.5, 2.0, -3.0]));
    let hz = c.block("Additional hit zones").unwrap();
    let wz = hz.block("SCR_WindowHitZone").unwrap();
    assert_eq!(wz.types, vec!["Default"]);
    assert_eq!(wz.prop_f64("Kinetic multiplier"), Some(4.0));
    assert_eq!(c.block("Tags").unwrap().values, vec!["OpenGate", "Other"]);
    assert_eq!(c.prop_values("Flags").unwrap(), ["0x403", "0"]);
    assert_eq!(r.anon.len(), 1);
    let kids = &r.anon[0].blocks;
    assert!(kids[0].grp && kids[0].anon.len() == 2);
    assert_eq!(kids[0].anon[1].prop_vec3("angles"), Some([0.0, -90.0, 0.0]));
    assert_eq!(kids[1].prop_vec3("coords"), Some([0.0, 0.0, -0.1]));
    assert_eq!(strip_guid("{ABC}Prefabs/x.et"), "Prefabs/x.et");
    assert_eq!(strip_guid("plain"), "plain");
    assert!(parse_et("A { B { }").is_err(), "unbalanced");
}

fn fixture_source() -> DirSource {
    DirSource {
        root: fixture("prefab"),
    }
}

#[test]
fn resolver_walks_inheritance_sockets_and_children() {
    let src = fixture_source();
    let mut r = PrefabResolver::new(&src);
    let house = r.resolve("Prefabs/Houses/House_Wood.et").expect("house");
    assert_eq!(house.class, "SCR_DestructibleBuildingEntity");
    assert_eq!(
        house.chain,
        vec![
            "Prefabs/Houses/House_Base.et".to_string(),
            "Prefabs/Core/Building_Base.et".to_string()
        ]
    );
    // Mesh comes from House_Base; Building_Base's placeholder never wins.
    assert_eq!(house.mesh.as_deref(), Some("Assets/Houses/House.xob"));
    assert_eq!(
        house.slot_bones,
        vec![
            (
                "socket_door_left".to_string(),
                "Prefabs/Doors/DoorSet.et".to_string()
            ),
            (
                "socket_win".to_string(),
                "Prefabs/Windows/Window.et".to_string()
            ),
        ]
    );
    // Base children (door + two windows) come first, then the furniture composition.
    let kinds: Vec<(&str, Option<&str>)> = house
        .children
        .iter()
        .map(|c| (c.prefab.as_str(), c.pivot_id.as_deref()))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ("Prefabs/Doors/DoorSet.et", Some("socket_door_left_01")),
            ("Prefabs/Windows/Window.et", Some("socket_win_01")),
            ("Prefabs/Windows/Window.et", Some("socket_win_02")),
            ("Prefabs/Core/Probe.et", None),
            ("Prefabs/Furniture/Furniture_01.et", None),
        ]
    );
    assert_eq!(house.children[0].coords, [0.0, 0.0, -0.1]);
    assert_eq!(house.children[3].coords, [2.655, 2.289, 4.876]);
    assert_eq!(house.children[4].id.as_deref(), Some("F1"));
    assert!(house.door.is_none() && house.sliding.is_none());

    let furniture = r
        .resolve("Prefabs/Furniture/Furniture_01.et")
        .expect("furniture");
    assert_eq!(furniture.children.len(), 3);
    assert_eq!(furniture.children[0].prefab, "Prefabs/Props/Table.et");
    assert_eq!(furniture.children[0].coords, [1.035, 0.28, -7.666]);
    assert_eq!(furniture.children[0].angles_deg, [0.0, 91.667, 0.0]);
    assert_eq!(furniture.children[2].scale, 1.152);
    assert_eq!(furniture.children[2].angles_deg, [88.816, -180.0, 96.7]);

    let set = r.resolve("Prefabs/Doors/DoorSet.et").expect("door set");
    assert_eq!(set.mesh.as_deref(), Some("Assets/Doors/DoorFrame.xob"));
    assert_eq!(set.children.len(), 1);
    assert_eq!(
        set.children[0].pivot_id.as_deref(),
        Some("socket_door_LEFT")
    );
    assert_eq!(set.children[0].prefab, "Prefabs/Doors/Door_Leaf.et");

    let leaf = r.resolve("Prefabs/Doors/Door_Leaf.et").expect("leaf");
    assert_eq!(leaf.mesh.as_deref(), Some("Assets/Doors/Door_Leaf.xob"));
    let d = leaf.door.as_ref().expect("rotating door");
    assert_eq!(d.angle_range_deg, -120.0);
    assert!(d.angle_range_explicit);
    assert_eq!(d.closed_angle_deg, 0.0);
    assert!(leaf.sliding.is_none());

    let plain = r
        .resolve("Prefabs/Doors/Door_Plain.et")
        .expect("plain door");
    let d = plain.door.as_ref().expect("door from the base chain");
    assert_eq!(d.angle_range_deg, DEFAULT_ANGLE_RANGE_DEG);
    assert!(!d.angle_range_explicit);

    let barn = r.resolve("Prefabs/Doors/Door_Sliding.et").expect("sliding");
    assert!(
        barn.door.is_none(),
        "DoorComponent Enabled 0 drops the rotating door"
    );
    assert_eq!(barn.sliding.as_ref().unwrap().opened_distance, 2.05);

    let window = r.resolve("Prefabs/Windows/Window.et").expect("window");
    assert_eq!(window.mesh.as_deref(), Some("Assets/Windows/Win.xob"));
    assert_eq!(
        window.slot_bones,
        vec![(
            "socket_glass".to_string(),
            "Prefabs/Windows/Glass.et".to_string()
        )]
    );
    assert_eq!(window.children.len(), 2);
    assert_eq!(
        window.children[1].pivot_id.as_deref(),
        Some("socket_glass_002")
    );
    let glass = r.resolve("Prefabs/Windows/Glass.et").expect("glass");
    assert_eq!(glass.mesh.as_deref(), Some("Assets/Windows/Glass_01.xob"));
    assert!(glass.hierarchy_pivot.is_none());
    // Memoized: a second resolve is the same Rc.
    let again = r.resolve("prefabs/windows/glass.et").unwrap();
    assert!(Rc::ptr_eq(&glass, &again));
    assert!(r.resolve("Prefabs/Missing.et").is_err());
}
