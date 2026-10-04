//! Shared fixtures of the mortar page tests: the test catalog, drafts, and one solved mission.

use super::inputs::battery::GunDraft;
use super::inputs::positions::{HeightChoice, MortarTerrain, PositionDraft};
use super::inputs::weapon_and_shell::{ArmamentSelection, ChargeChoice};
use super::inputs::wind::WindDraft;
use super::solve_bridge::{MissionDrafts, SolvedMission, solve_mission};
use ballistics_model::catalog::BallisticsCatalog;

/// The test catalog (`m252` and `2b14`; `m853a1`, `m821` and `o-832du`).
pub(crate) fn catalog() -> BallisticsCatalog {
    BallisticsCatalog::from_json_slice(include_bytes!("mortar_test_catalog.json"))
        .expect("the test catalog decodes")
}

/// A position with a manual height.
pub(crate) fn manual(grid: &str, height: &str) -> PositionDraft {
    PositionDraft {
        grid: grid.into(),
        height_choice: HeightChoice::Manual,
        manual_height: height.into(),
    }
}

/// A gun with a manual height.
pub(crate) fn gun(key: u32, label: &str, grid: &str, height: &str) -> GunDraft {
    GunDraft {
        key,
        label: label.into(),
        position: manual(grid, height),
    }
}

/// Two guns of `m252` firing `m853a1` at "064 129" with a 500 m burst in a 4 m/s wind.
pub(crate) fn solved_mission() -> SolvedMission {
    let chosen = ArmamentSelection {
        weapon_id: "m252".into(),
        shell_id: "m853a1".into(),
        charge: ChargeChoice::Recommended,
    };
    let target = manual("064 129", "40");
    let guns = [
        gun(0, "Gun 1", "055 125", "25"),
        gun(1, "Gun 2", "056 124", "30"),
    ];
    let wind = WindDraft {
        speed_m_s: "4".into(),
        from_deg: "200".into(),
    };
    let drafts = MissionDrafts {
        selection: &chosen,
        terrain: MortarTerrain::Everon,
        target: &target,
        guns: &guns,
        wind: &wind,
        burst_height: "500",
        crest_profile: None,
    };
    solve_mission(&catalog(), drafts, |_, _| None).expect("the fixture mission solves")
}
