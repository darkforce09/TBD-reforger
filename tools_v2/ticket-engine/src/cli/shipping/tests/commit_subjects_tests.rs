use super::*;

use std::path::PathBuf;

/// The boundary pins: a two-digit id against a three-digit one and against its own dotted
/// children, dot-segment included, plus the leading
/// guard and per-subject dedupe. A loose rule would hand one ticket another ticket's commits.
#[test]
fn subject_id_boundary_pins() {
    assert_eq!(subject_ids("T-902: fix the thing"), vec!["T-902"]);
    assert_eq!(subject_ids("T-90.1 polish pass"), vec!["T-90.1"]);
    assert_eq!(subject_ids("T-90.10: deeper"), vec!["T-90.10"]);
    assert_eq!(
        subject_ids("T-90: done; T-90.1 ready"),
        vec!["T-90", "T-90.1"]
    );
    assert_eq!(subject_ids("revert T-90"), vec!["T-90"]);
    assert_eq!(subject_ids("wave(T-90) closes"), vec!["T-90"]);
    assert_eq!(subject_ids("Revert \"T-233: page\""), vec!["T-233"]);
    assert_eq!(subject_ids("XT-90 is not a ticket"), Vec::<String>::new());
    assert_eq!(subject_ids("T-90 then T-90 again"), vec!["T-90"]);
    assert_eq!(subject_ids("T-90.1.2: grandchild"), vec!["T-90.1.2"]);
    assert_eq!(subject_ids("no ids here"), Vec::<String>::new());
}

/// A `+02:00` input normalizes to `Z` and satisfies the stamp validator; `+00:00` and an
/// already-`Z` input both come out canonical; a naive date-time refuses rather than being read
/// as UTC.
#[test]
fn utc_normalization() {
    assert_eq!(
        to_utc_z("2026-08-15T01:08:31+02:00").unwrap(),
        "2026-08-14T23:08:31Z"
    );
    assert_eq!(
        to_utc_z("2026-06-13T18:20:53+00:00").unwrap(),
        "2026-06-13T18:20:53Z"
    );
    assert_eq!(
        to_utc_z("2026-08-14T10:00:00Z").unwrap(),
        "2026-08-14T10:00:00Z"
    );
    validate_rfc3339_utc("stamp", &to_utc_z("2026-08-15T01:08:31+02:00").unwrap())
        .expect("canonical");
    assert!(to_utc_z("2026-08-14 10:00").is_err(), "naive must refuse");
}

/// The SHA shape a mined `shipped_at` must satisfy: 7–40 lowercase hex, so a date, a ticket id or
/// a branch name can never be mistaken for a landing commit.
#[test]
fn shape_predicates() {
    assert!(crate::is_sha_shaped("5e5d3bbd"));
    assert!(crate::is_sha_shaped(
        "b071c49e3b84f59e5cfc279c2f05b04de32b850a"
    ));
    assert!(!crate::is_sha_shaped("2026-07-26"));
    assert!(!crate::is_sha_shaped("T-128"));
    assert!(!crate::is_sha_shaped("slice/T-197"));
    assert!(!crate::is_sha_shaped("abc123")); // 6 hex — too short
}

/// Live-repo smoke: the miner reads real history — a shipped id has subject commits, their dates come
/// back `Z`-canonical, and the list is oldest-first.
#[test]
fn mine_subjects_live_repo_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools_v2")
        .parent()
        .expect("repository root")
        .to_path_buf();
    let map = mine_subjects(&root).expect("mine live history");
    let t9171 = map.get("T-917.1").expect("T-917.1 has subject commits");
    assert!(t9171.len() >= 2, "vocab commit + ship commit");
    for c in t9171 {
        validate_rfc3339_utc("mined", &c.date_utc).expect("Z-canonical");
    }
    assert!(
        t9171.first().unwrap().date_utc <= t9171.last().unwrap().date_utc,
        "oldest-first"
    );
}
