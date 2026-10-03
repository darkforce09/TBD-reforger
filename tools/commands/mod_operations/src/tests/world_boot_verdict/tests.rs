use super::*;

const SCEN: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";

fn good() -> String {
    format!(
        "DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{SCEN}'.\n\
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'\n\
SCRIPT    (E): [TBD][Mission] NO MISSION YET - no machine credential is configured (backend=), and no verified artifact is cached in $profile:TBD_MissionArtifactCache.\n"
    )
}

#[test]
fn good_log_passes() {
    assert!(assess_quiet(&good(), SCEN));
}

#[test]
fn bad_missing_rejects_exit_equiv_1() {
    let log = format!(
        "DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{SCEN}'.\n\
SCRIPT    (E): string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=MISSING'\n"
    );
    assert!(
        !assess_quiet(&log, SCEN),
        "bad-missing must be rejected (assess exit 1); exit 0 = hollow"
    );
}

#[test]
fn bad_unknown_class_rejects() {
    let log = format!(
        "DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{SCEN}'.\n\
WORLD     (E): Unknown class 'TBD_ThisComponentDoesNotExist' at offset 530(0x212)\n\
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'\n"
    );
    assert!(!assess_quiet(&log, SCEN));
}

#[test]
fn selftest_harness_ok() {
    assert_eq!(cmd_selftest(), 0);
}
