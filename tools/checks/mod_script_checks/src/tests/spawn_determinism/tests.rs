use super::*;

#[test]
fn extract_drops_mission_load_failure() {
    assert!(!extract(INVALID).contains("Mission loaded but invalid"));
}

#[test]
fn assess_healthy_passes() {
    let d = tempfile_dir("tbd-det-test-h");
    let p = d.join("h.log");
    fs::write(&p, HEALTHY).unwrap();
    assert_eq!(assess_run_silent(&p, "t"), 0);
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn assess_stale_fails() {
    let d = tempfile_dir("tbd-det-test-s");
    let p = d.join("s.log");
    fs::write(&p, STALE).unwrap();
    assert_eq!(assess_run_silent(&p, "t"), 1);
    let _ = fs::remove_dir_all(&d);
}
