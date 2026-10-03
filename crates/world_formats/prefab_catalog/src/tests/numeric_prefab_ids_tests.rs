//! Unit tests of the numeric prefab ids: which JSON numbers name a prefab.

use crate::numeric_prefab_ids::*;
use world_file_formats::ids::PrefabId;

#[test]
fn prefab_id_from_f64_names_a_prefab_only_for_the_exact_f64_of_a_u32() {
    assert_eq!(prefab_id_from_f64(0.0), Some(PrefabId::new(0)));
    assert_eq!(prefab_id_from_f64(9.0), Some(PrefabId::new(9)));
    assert_eq!(
        prefab_id_from_f64(f64::from(u32::MAX)),
        Some(PrefabId::new(u32::MAX))
    );
    for pid in [
        9.5,
        -9.0,
        -0.0,
        -0.5,
        f64::from(u32::MAX) + 1.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        assert_eq!(prefab_id_from_f64(pid), None, "pid {pid:?} names no prefab");
    }
    for n in [0_u32, 1, 9, 1622, 65_535, 65_536, u32::MAX] {
        let pid = f64::from(n);
        assert_eq!(prefab_id_from_f64(pid), Some(PrefabId::new(n)));
        assert_eq!(
            f64::from(prefab_id_from_f64(pid).unwrap().get()).to_bits(),
            pid.to_bits()
        );
    }
}
